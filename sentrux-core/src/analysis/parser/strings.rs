//! String/comment stripping utilities for complexity counting.
//!
//! Extracted from parser_imports.rs to keep that module under 500 lines.
//! These functions remove string literals, block comments, and triple-quoted
//! strings from source code so that keywords inside them don't inflate
//! complexity counts.

// ── String/comment stripping ────────────────────────────────────────────

/// Handle a line while inside a multi-line triple-quoted string.
/// Returns (output_line, still_in_triple_quote).
fn handle_triple_quote_line(trimmed: &str, tq_char: char) -> (Option<String>, bool) {
    let tq_pattern: String = std::iter::repeat_n(tq_char, 3).collect();
    if let Some(close_pos) = trimmed.find(&tq_pattern) {
        let after = &trimmed[close_pos + 3..];
        let after_trimmed = after.trim_start();
        if after_trimmed.is_empty() {
            (None, false)
        } else {
            (Some(strip_string_literals(after)), false)
        }
    } else {
        (None, true) // still inside triple-quoted string
    }
}

/// Handle a line while inside a block comment (with nesting support).
/// Returns (output_line, new_depth).
fn handle_block_comment_line(trimmed: &str, mut depth: u32) -> (Option<String>, u32) {
    let mut pos = 0;
    let bytes = trimmed.as_bytes();
    while pos + 1 < bytes.len() {
        if bytes[pos] == b'/' && bytes[pos + 1] == b'*' {
            depth += 1;
            pos += 2;
        } else if bytes[pos] == b'*' && bytes[pos + 1] == b'/' {
            depth = depth.saturating_sub(1);
            pos += 2;
            if depth == 0 {
                let after = trimmed[pos..].trim_start();
                if after.is_empty() {
                    return (None, 0);
                }
                return (Some(strip_string_literals(after)), 0);
            }
        } else {
            pos += 1;
        }
    }
    (None, depth)
}

/// Strip inline block comments (/* ... */) from a line, handling unclosed blocks.
/// Returns (output_line, new_block_depth).
fn strip_inline_block_comments(trimmed: &str) -> (Option<String>, u32) {
    let mut work = trimmed.to_string();
    let mut had_comment = false;
    loop {
        if let Some(start_pos) = work.find("/*") {
            had_comment = true;
            if let Some(end_pos) = work[start_pos + 2..].find("*/") {
                let before = &work[..start_pos];
                let after = &work[start_pos + 2 + end_pos + 2..];
                work = format!("{} {}", before, after);
                continue;
            } else {
                let before = work[..start_pos].trim();
                if before.is_empty() {
                    return (None, 1);
                }
                return (Some(strip_string_literals(before)), 1);
            }
        }
        break;
    }
    if had_comment {
        let work_trimmed = work.trim();
        if work_trimmed.is_empty() {
            return (None, 0);
        }
        return (Some(strip_string_literals(work_trimmed)), 0);
    }
    (None, 0) // no block comment found — caller should handle
}

/// Detect unclosed triple-quote on a line (Python only). [ref:6c60c4ee]
/// Returns Some(tq_char) if we entered a triple-quote, with optional output line.
fn detect_python_triple_quote(trimmed: &str) -> Option<(char, Option<String>)> {
    let stripped_singles = strip_string_literals(trimmed);
    for tq_char in ['"', '\''] {
        let tq: String = std::iter::repeat_n(tq_char, 3).collect();
        let count = stripped_singles.matches(&tq).count();
        if count % 2 == 1 {
            if let Some(pos) = trimmed.find(&tq) {
                let before = &trimmed[..pos];
                if !before.trim().is_empty() {
                    return Some((tq_char, Some(strip_string_literals(before))));
                }
            }
            return Some((tq_char, None));
        }
    }
    None
}

/// Check if a trimmed line is a single-line comment.
/// For `*` patterns (block-comment continuation), we require the `*` to not be
/// followed by an identifier character (to avoid matching pointer dereferences
/// like `*ptr` or expressions like `* count`). A bare `*` or `* ` followed by
/// non-alphanumeric/underscore text is treated as a block comment continuation.
fn is_single_line_comment(trimmed: &str, hash_is_comment: bool) -> bool {
    if trimmed.starts_with("//") {
        return true;
    }
    if trimmed.starts_with("*/") || trimmed == "*" {
        return true;
    }
    // Block-comment continuation: `* text` but NOT `* expr = ...` (pointer deref).
    // Heuristic: a comment continuation line starting with `* ` should not contain
    // assignment operators or semicolons, which indicate code.
    if trimmed.starts_with("* ") {
        let rest = &trimmed[2..];
        // If the rest contains code indicators, it's likely pointer arithmetic, not a comment.
        if !rest.contains('=') && !rest.contains(';') && !rest.contains('(') {
            return true;
        }
    }
    if hash_is_comment && trimmed.starts_with('#') {
        return true;
    }
    false
}

/// Mutable state for the strip_strings_and_comments line processor.
struct StripState {
    block_comment_depth: u32,
    in_triple_quote: Option<char>,
}

impl StripState {
    fn new() -> Self {
        Self { block_comment_depth: 0, in_triple_quote: None }
    }

    /// Process one line, returning the stripped output (or None to skip the line).
    fn process_line(&mut self, line: &str, hash_is_comment: bool, has_triple_quote_strings: bool) -> Option<String> {
        let trimmed = line.trim_start();

        if let Some(tq_char) = self.in_triple_quote {
            let (out, still_in) = handle_triple_quote_line(trimmed, tq_char);
            self.in_triple_quote = if still_in { Some(tq_char) } else { None };
            return out;
        }

        if self.block_comment_depth > 0 {
            let (out, new_depth) = handle_block_comment_line(trimmed, self.block_comment_depth);
            self.block_comment_depth = new_depth;
            return out;
        }

        if is_single_line_comment(trimmed, hash_is_comment) {
            return None;
        }

        if trimmed.contains("/*") {
            let (out, depth) = strip_inline_block_comments(trimmed);
            self.block_comment_depth = depth;
            if out.is_some() || depth > 0 {
                return out;
            }
        }

        if has_triple_quote_strings {
            if let Some((tq_char, out)) = detect_python_triple_quote(trimmed) {
                self.in_triple_quote = Some(tq_char);
                return out;
            }
        }
        Some(strip_string_literals(line))
    }
}

/// Strip strings and comments from source code, returning only code lines.
/// Handles block comments (with nesting for Rust), single-line comments,
/// triple-quoted strings (Python), and string literals.
///
/// Language-specific behavior is driven by the language profile (Layer 2):
/// - `hash_is_comment`: from `profile.semantics.hash_is_comment`
/// - `has_triple_quote_strings`: from `profile.semantics.has_triple_quote_strings`
pub(crate) fn strip_strings_and_comments(body: &str, lang: &str) -> String {
    let profile = crate::analysis::lang_registry::profile(lang);
    let hash_is_comment = profile.semantics.hash_is_comment;
    let has_triple_quote_strings = profile.semantics.has_triple_quote_strings;
    let mut state = StripState::new();

    body.lines()
        .filter_map(|line| state.process_line(line, hash_is_comment, has_triple_quote_strings))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Helpers for strip_string_literals ────────────────────────────────

/// Try to parse a Python-style string prefix (r, b, f, rb, br, fr, rf) at position `i`.
/// Returns `Some((prefix_len, is_raw, quote_char))` if a prefixed string starts here.
fn try_prefixed_string(chars: &[char], i: usize) -> Option<(usize, bool, char)> {
    let len = chars.len();
    let c = chars[i];
    if !(c == 'r' || c == 'b' || c == 'f') || i + 1 >= len {
        return None;
    }
    // Two-char prefix (rb, br, fr, rf) followed by quote
    if i + 2 < len
        && matches!((c, chars[i + 1]), ('r', 'b') | ('b', 'r') | ('r', 'f') | ('f', 'r'))
        && (chars[i + 2] == '"' || chars[i + 2] == '\'')
    {
        let is_raw = c == 'r' || chars[i + 1] == 'r';
        return Some((2, is_raw, chars[i + 2]));
    }
    // Single-char prefix followed by quote
    if chars[i + 1] == '"' || chars[i + 1] == '\'' {
        return Some((1, c == 'r', chars[i + 1]));
    }
    None
}

/// Consume a prefixed string literal content, stripping it.
/// Returns the new position after the closing quote.
fn consume_prefixed_string(chars: &[char], mut i: usize, is_raw: bool, quote: char, result: &mut String) -> usize {
    let len = chars.len();
    while i < len {
        if !is_raw && chars[i] == '\\' && i + 1 < len {
            i += 2;
            continue;
        }
        if chars[i] == quote {
            result.push(quote);
            return i + 1;
        }
        i += 1;
    }
    i
}

/// Count consecutive '#' characters starting at position `j`.
/// Returns (hash_count, position_after_hashes).
fn count_hashes(chars: &[char], j: usize) -> (usize, usize) {
    let mut hashes = 0;
    let mut pos = j;
    while pos < chars.len() && chars[pos] == '#' {
        hashes += 1;
        pos += 1;
    }
    (hashes, pos)
}

/// Check if position `pos` is the closing `"###` of a Rust raw string.
/// Returns true if exactly `hashes` '#' chars follow the '"' at `pos`.
fn is_raw_string_close(chars: &[char], pos: usize, hashes: usize) -> bool {
    let mut k = 0;
    while k < hashes && pos + 1 + k < chars.len() && chars[pos + 1 + k] == '#' {
        k += 1;
    }
    k == hashes
}

/// Emit the raw string delimiter (r###"...or..."###) into result.
fn emit_raw_delim(result: &mut String, quote: char, hashes: usize) {
    if quote == 'r' { result.push('r'); }
    for _ in 0..hashes { result.push('#'); }
    result.push('"');
}

/// Scan for the closing delimiter of a Rust raw string and emit it.
/// Returns the position after the closing delimiter, or `len` if not found.
fn scan_raw_string_close(chars: &[char], start: usize, hashes: usize, result: &mut String) -> usize {
    let len = chars.len();
    let mut pos = start;
    while pos < len {
        if chars[pos] == '"' && is_raw_string_close(chars, pos, hashes) {
            result.push('"');
            for _ in 0..hashes { result.push('#'); }
            return pos + 1 + hashes;
        }
        pos += 1;
    }
    pos
}

/// Try to parse a Rust raw string `r#"..."#` at position `i`.
/// Returns `Some(new_i)` after consuming it, or `None` if not a Rust raw string.
fn try_rust_raw_string(chars: &[char], i: usize, result: &mut String) -> Option<usize> {
    let len = chars.len();
    if chars[i] != 'r' || i + 1 >= len || chars[i + 1] != '#' {
        return None;
    }
    let (hashes, j) = count_hashes(chars, i + 1);
    if j >= len || chars[j] != '"' {
        return None;
    }
    emit_raw_delim(result, 'r', hashes);
    Some(scan_raw_string_close(chars, j + 1, hashes, result))
}

/// Consume a template expression `${...}` inside a backtick string.
/// Returns the new position after the closing `}`.
fn consume_template_expression(chars: &[char], mut i: usize, result: &mut String) -> usize {
    let len = chars.len();
    let mut depth = 1u32;
    while i < len && depth > 0 {
        if chars[i] == '{' {
            depth += 1;
        } else if chars[i] == '}' {
            depth -= 1;
        }
        result.push(chars[i]);
        i += 1;
    }
    i
}

/// Consume a backtick template literal, preserving `${...}` expressions.
/// Returns the new position after the closing backtick.
fn consume_backtick_literal(chars: &[char], mut i: usize, result: &mut String) -> usize {
    let len = chars.len();
    while i < len {
        if chars[i] == '\\' {
            i = (i + 2).min(len);
        } else if chars[i] == '$' && i + 1 < len && chars[i + 1] == '{' {
            result.push('$');
            result.push('{');
            i = consume_template_expression(chars, i + 2, result);
        } else if chars[i] == '`' {
            result.push('`');
            return i + 1;
        } else {
            i += 1;
        }
    }
    i
}

/// Consume a triple-quoted string, stripping content.
/// Returns the new position after the closing triple-quote.
fn consume_triple_quote(chars: &[char], mut i: usize, quote: char, result: &mut String) -> usize {
    let len = chars.len();
    while i + 2 < len {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == quote && chars[i + 1] == quote && chars[i + 2] == quote {
            result.push(quote);
            result.push(quote);
            result.push(quote);
            return i + 3;
        }
        i += 1;
    }
    len
}

/// Consume a regular quoted string, stripping content.
/// Returns the new position after the closing quote.
fn consume_regular_string(chars: &[char], mut i: usize, quote: char, result: &mut String) -> usize {
    let len = chars.len();
    while i < len {
        if chars[i] == '\\' {
            i = (i + 2).min(len);
        } else if chars[i] == quote {
            result.push(quote);
            return i + 1;
        } else {
            i += 1;
        }
    }
    i
}

/// Strip content of string literals (single/double/backtick quoted) to prevent
/// keywords inside strings from inflating complexity counts.
/// Preserves the quotes themselves so line structure is maintained.
pub(crate) fn strip_string_literals(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    // For ASCII-only lines (most source code), convert bytes directly to chars
    // avoiding UTF-8 decode overhead. Both paths still need Vec<char> for helper functions.
    let chars: Vec<char> = if line.is_ascii() {
        line.bytes().map(|b| b as char).collect()
    } else {
        line.chars().collect()
    };
    let len = chars.len();
    let mut i = 0;
    while i < len {
        let c = chars[i];

        // Handle Python-style string prefixes (r"...", b"...", f"...", rb"...", etc.)
        if let Some((prefix_len, is_raw, quote)) = try_prefixed_string(&chars, i) {
            for k in 0..prefix_len { result.push(chars[i + k]); }
            result.push(quote);
            i = consume_prefixed_string(&chars, i + prefix_len + 1, is_raw, quote, &mut result);
            continue;
        }

        // Handle Rust raw strings: r#"..."#, r##"..."##, etc.
        if let Some(new_i) = try_rust_raw_string(&chars, i, &mut result) {
            i = new_i;
            continue;
        }

        // Handle backtick template literals — preserve ${...} expressions
        if c == '`' {
            result.push(c);
            i = consume_backtick_literal(&chars, i + 1, &mut result);
            continue;
        }

        // Handle single/double quotes (including triple-quotes)
        if c == '"' || c == '\'' {
            if i + 2 < len && chars[i + 1] == c && chars[i + 2] == c {
                result.push(c);
                result.push(c);
                result.push(c);
                i = consume_triple_quote(&chars, i + 3, c, &mut result);
            } else {
                result.push(c);
                i = consume_regular_string(&chars, i + 1, c, &mut result);
            }
            continue;
        }

        result.push(c);
        i += 1;
    }
    result
}
