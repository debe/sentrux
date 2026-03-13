//! Import normalization, per-language extraction, base-class extraction,
//! complexity counting, and string/comment stripping utilities.
//!
//! Extracted from parser.rs to keep the main parser module focused on
//! tree-sitter integration and caching.
//!
//! Per-language extractors live in lang_extractors.rs.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use super::lang_extractors;
pub(crate) use super::strings::strip_strings_and_comments;

// ── Import extraction & normalization ───────────────────────────────────

/// Extract and **normalize** import module paths from raw source text.
///
/// # Universal contract
/// The output is a Vec of **normalized module paths**: slash-separated segments,
/// no language syntax (no braces, no quotes, no keywords, no semicolons).
/// The resolver is completely language-agnostic — all language knowledge lives here.
///
/// Examples of normalized output:
///   Python  `from os.path import join`                       → ["os/path"]
///   Python  `import os, sys`                                 → ["os", "sys"]
///   Python  `from .utils import foo`                         → [".utils"]
///   Rust    `use crate::models::episode::{Episode, Inj}`    → ["crate/models/episode"]
///   Rust    `use crate::models::{episode, primitive}`        → ["crate/models/episode", "crate/models/primitive"]
///   Rust    `mod graph;`                                     → ["graph"]
///   Go      `import ("fmt" "os")`                            → ["fmt", "os"]
///   Java    `import com.example.UserService;`                → ["com/example/UserService"]
///   C       `#include "mylib.h"`                             → ["mylib.h"]
///   Ruby    `require 'json'`                                 → ["json"]
///   HTML    `<script src="./app.js">`                        → ["./app.js"]
pub(crate) fn extract_import_modules(text: &str, lang: &str) -> Vec<String> {
    // Step 1: Language-specific extraction — get raw module strings
    let raw_modules: Vec<String> = match lang {
        "python" => lang_extractors::extract_python(text),
        "rust" => lang_extractors::extract_rust(text),
        "go" => lang_extractors::extract_go(text),
        "c" | "cpp" => lang_extractors::extract_c_cpp(text),
        "java" | "csharp" => lang_extractors::extract_java_csharp(text),
        "ruby" => lang_extractors::extract_ruby(text),
        "php" => lang_extractors::extract_php(text),
        "gdscript" => lang_extractors::extract_gdscript(text),
        "html" => lang_extractors::extract_html(text),
        "css" | "scss" | "sass" => lang_extractors::extract_css(text),
        "scala" | "swift" | "kotlin" => lang_extractors::extract_jvm_like(text),
        "elixir" => lang_extractors::extract_elixir(text),
        _ => lang_extractors::extract_fallback(text),
    };

    // Step 2: Language-aware normalization.
    // Languages that use dots as module separators (Python, Java, Scala, etc.)
    // always convert dots → slashes. File-path languages (C, HTML, CSS) never do.
    // This replaces the fragile heuristic that guessed based on extension length,
    // which incorrectly treated Python module names like "config", "utils" as file
    // extensions and skipped dot conversion. [ref:daa66d13]
    let dots_are_separators = lang_uses_dot_separator(lang);
    raw_modules
        .into_iter()
        .map(|m| normalize_module_path(&m, dots_are_separators))
        .filter(|m| !m.is_empty())
        .collect()
}

/// Whether '.' is a module separator (not a file extension) for this language.
/// Reads from the language profile (Layer 2). Falls back to false for unknown languages.
pub(crate) fn lang_uses_dot_separator(lang: &str) -> bool {
    crate::analysis::lang_registry::profile(lang).semantics.dot_is_module_separator
}

/// Normalize a module path to slash-separated form.
/// `dots_are_separators`: true for languages where '.' means module separator
/// (Python, Java, C#, Scala, Kotlin, Ruby, PHP). False for file-path languages
/// (C/C++, Go, HTML, CSS) and Rust (uses :: which is always converted).
pub(crate) fn normalize_module_path(raw: &str, dots_are_separators: bool) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }

    // Preserve leading dots (relative imports) but normalize the rest
    let (prefix, rest) = if s.starts_with('.') {
        let dot_count = s.bytes().take_while(|&b| b == b'.').count();
        (&s[..dot_count], &s[dot_count..])
    } else {
        ("", s)
    };

    // Always convert '::' → '/' (Rust paths).
    // Convert '.' → '/' only when the language uses dots as module separators.
    // File-path languages (C, HTML, CSS) keep dots as-is (they're file extensions).
    let mut normalized = rest.replace("::", "/");
    if dots_are_separators && !normalized.contains('/') && rest.contains('.') {
        // Only convert dots when no slashes present (avoids mangling file paths
        // that were already slash-separated by the :: conversion).
        normalized = normalized.replace('.', "/");
    }

    format!("{}{}", prefix, normalized)
}

// ── Bash & HTML import helpers ──────────────────────────────────────────

/// Extract bash `source ./file.sh` and `. ./file.sh` as imports.
/// Tree-sitter captures these as regular commands (no distinct import syntax),
/// so we scan the source text after parsing. Comments are skipped via `#` prefix.
/// Variable-containing paths ($DIR/lib.sh) are skipped — can't resolve at static time.
/// Extract the source path from a single bash line, if it's a `source` or `. ` command.
/// Returns None if the line is a comment or not a source command.
fn extract_bash_source_path(trimmed: &str) -> Option<&str> {
    if trimmed.starts_with('#') {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("source ") {
        let no_comment = rest.split('#').next().unwrap_or(rest);
        Some(no_comment.trim().trim_matches(|c: char| c == '"' || c == '\''))
    } else if trimmed.starts_with(". ") && !trimmed.starts_with("..") {
        let no_comment = trimmed[2..].split('#').next().unwrap_or(&trimmed[2..]);
        Some(no_comment.trim().trim_matches(|c: char| c == '"' || c == '\''))
    } else {
        None
    }
}

pub(crate) fn extract_bash_imports(content: &[u8], imports: &mut Vec<String>, import_set: &mut HashSet<String>) {
    let text = match std::str::from_utf8(content) {
        Ok(t) => t,
        Err(_) => return,
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(p) = extract_bash_source_path(trimmed) {
            if !p.is_empty() && !p.contains('$') && import_set.insert(p.to_string()) {
                imports.push(p.to_string());
            }
        }
    }
}

/// HTML post-filter: the query captures ALL attributes as @import.module
/// (since tree-sitter #eq? predicates aren't evaluated by QueryCursor).
/// Keep only values that look like local file references.
pub(crate) fn filter_html_imports(imports: &mut Vec<String>) {
    let before = imports.len();
    imports.retain(|imp| {
        // Must have a file extension
        if !imp.contains('.') { return false; }
        // Skip external URLs
        if imp.contains("://") { return false; }
        // Skip data URIs, anchors, mailto, tel
        if imp.starts_with("data:") || imp.starts_with('#')
            || imp.starts_with("mailto:") || imp.starts_with("tel:")
        { return false; }
        // Skip common non-file attribute values
        if imp == "stylesheet" || imp == "module" || imp == "text/javascript"
            || imp == "text/css" || imp == "noopener"
        { return false; }
        true
    });
    let filtered = before - imports.len();
    if filtered > 0 && before > 10 {
        eprintln!("[html_imports] filtered {}/{} non-file attributes", filtered, before);
    }
}

// ── Base class extraction ───────────────────────────────────────────────

/// Extract base/parent class names from a class definition AST node.
/// Walks the node's children to find language-specific superclass structures.
pub(crate) fn extract_base_classes(node: tree_sitter::Node, content: &[u8], lang: &str) -> Option<Vec<String>> {
    let mut bases = Vec::new();
    match lang {
        "python" => lang_extractors::extract_bases_python(node, content, &mut bases),
        "java" | "kotlin" | "csharp" | "scala" => lang_extractors::extract_bases_jvm(node, content, &mut bases),
        "typescript" | "javascript" => {
            lang_extractors::extract_bases_by_kinds(node, content, &["class_heritage", "extends_clause"], &mut bases);
        }
        "ruby" => lang_extractors::extract_bases_by_kinds(node, content, &["superclass"], &mut bases),
        "cpp" | "c" => lang_extractors::extract_bases_by_kinds(node, content, &["base_class_clause"], &mut bases),
        "php" => {
            lang_extractors::extract_bases_by_kinds(node, content, &["base_clause", "class_interface_clause"], &mut bases);
        }
        _ => lang_extractors::extract_bases_generic(node, content, &mut bases),
    }
    if bases.is_empty() { None } else { Some(bases) }
}

// ── Complexity counting ─────────────────────────────────────────────────

/// Get language-specific complexity keywords from the language profile.
/// Falls back to the profile's compiled defaults when plugin.toml omits [semantics.complexity].
fn complexity_keywords_for(lang: &str) -> Vec<String> {
    crate::analysis::lang_registry::profile(lang).semantics.complexity.cc.clone()
}

/// Deduplicate and sort keywords longest-first for non-overlapping matching.
fn prepare_keywords<'a>(keywords: &[&'a str]) -> Vec<&'a str> {
    let mut seen_kw = HashSet::new();
    let mut deduped: Vec<&str> = keywords
        .iter()
        .filter(|kw| seen_kw.insert(**kw))
        .copied()
        .collect();
    deduped.sort_unstable_by_key(|k| std::cmp::Reverse(k.len()));
    deduped
}

/// Check if position `abs` is already consumed by a longer keyword match.
fn is_consumed(consumed: &[(usize, usize)], abs: usize) -> bool {
    consumed.iter().any(|&(s, e)| abs >= s && abs < e)
}

/// Check left boundary: keyword at `abs` must not be preceded by an identifier char.
fn left_boundary_ok(bytes: &[u8], abs: usize, is_operator: bool) -> bool {
    is_operator || abs == 0 || {
        let c = bytes[abs - 1];
        !c.is_ascii_alphanumeric() && c != b'_'
    }
}

/// Count keyword occurrences in a line using position-tracking to avoid overlaps.
fn count_keyword_in_line(
    line: &str, _trimmed: &str, kw: &str, is_operator: bool, consumed: &mut Vec<(usize, usize)>,
) -> u32 {
    let mut count = 0u32;
    // Scan for all occurrences within the line
    let bytes = line.as_bytes();
    let kw_len = kw.len();
    let mut pos = 0;
    while pos + kw_len <= bytes.len() {
        let idx = match line[pos..].find(kw) {
            Some(idx) => idx,
            None => break,
        };
        let abs = pos + idx;
        if left_boundary_ok(bytes, abs, is_operator) && !is_consumed(consumed, abs) {
            count += 1;
            consumed.push((abs, abs + kw_len));
        }
        pos = abs + 1;
    }
    count
}

pub(crate) fn count_complexity(body: &str, lang: &str) -> u32 {
    let keywords = complexity_keywords_for(lang);
    let code_lines = strip_strings_and_comments(body, lang);
    let kw_refs: Vec<&str> = keywords.iter().map(|s| s.as_str()).collect();
    let deduped_keywords = prepare_keywords(&kw_refs);

    let mut cc = 1u32;
    for line in code_lines.lines() {
        let trimmed = line.trim_start();
        let mut consumed: Vec<(usize, usize)> = Vec::new();
        for &kw in &deduped_keywords {
            let is_operator = kw == "&&" || kw == "||";
            cc += count_keyword_in_line(line, trimmed, kw, is_operator, &mut consumed);
        }
    }
    cc
}

/// Get branch keywords for cognitive complexity from the language profile.
fn cog_branch_keywords_for(lang: &str) -> Vec<String> {
    crate::analysis::lang_registry::profile(lang).semantics.complexity.cog_branch.clone()
}

/// Get nesting-increasing keywords for cognitive complexity from the language profile.
fn cog_nesting_keywords_for(lang: &str) -> Vec<String> {
    crate::analysis::lang_registry::profile(lang).semantics.complexity.cog_nesting.clone()
}

/// Check if any branch keyword matches this trimmed line. Returns branch penalty if matched.
/// Only matches at start of line (control-flow statements), not inside identifiers.
fn cog_branch_penalty(trimmed: &str, branch_kw: &[&str], nesting: i32) -> u32 {
    for &kw in branch_kw {
        let kw_t = kw.trim_start();
        if trimmed.starts_with(kw_t) {
            return 1 + nesting.max(0) as u32;
        }
    }
    0
}

/// Count logical operators (&&, ||, and, or) on a trimmed line.
fn cog_logic_penalty(trimmed: &str) -> u32 {
    const LOGIC_OPS: &[&str] = &["&&", "||", " and ", " or "];
    LOGIC_OPS.iter().map(|&op| trimmed.matches(op).count() as u32).sum()
}

/// Update nesting depth based on brace counts and nesting keywords.
/// Strips string literals first so braces inside strings (e.g., `"{"`) are not counted.
fn cog_update_nesting(trimmed: &str, nesting: i32, nesting_kw: &[&str]) -> i32 {
    let stripped = super::strings::strip_string_literals(trimmed);
    let opens = stripped.bytes().filter(|&b| b == b'{').count() as i32;
    let closes = stripped.bytes().filter(|&b| b == b'}').count() as i32;
    let is_nesting_line = nesting_kw.iter().any(|&kw| {
        let kw_t = kw.trim_start();
        trimmed.starts_with(kw_t)
    });
    let new = if is_nesting_line && opens > closes {
        nesting + 1
    } else {
        nesting + opens - closes
    };
    new.max(0)
}

/// Cognitive complexity (SonarSource 2016): nesting-weighted branch count.
/// Each branch keyword adds (1 + current_nesting_depth). Nesting keywords
/// (if/for/while/match/loop) increase depth for their body.
pub(crate) fn count_cognitive_complexity(body: &str, lang: &str) -> u32 {
    let code_lines = strip_strings_and_comments(body, lang);
    let branch_kw_owned = cog_branch_keywords_for(lang);
    let nesting_kw_owned = cog_nesting_keywords_for(lang);
    let branch_kw: Vec<&str> = branch_kw_owned.iter().map(|s| s.as_str()).collect();
    let nesting_kw: Vec<&str> = nesting_kw_owned.iter().map(|s| s.as_str()).collect();

    let mut cog = 0u32;
    let mut nesting: i32 = 0;

    for line in code_lines.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        cog += cog_branch_penalty(trimmed, &branch_kw, nesting);
        cog += cog_logic_penalty(trimmed);
        nesting = cog_update_nesting(trimmed, nesting, &nesting_kw);
    }
    cog
}

/// Parameter list node kinds recognized across languages.
const PARAM_LIST_KINDS: &[&str] = &[
    "parameters", "formal_parameters", "parameter_list",
    "function_type_parameters", "type_parameters",
];

/// Check if a parameter node represents self/this (should be excluded from count).
fn is_self_or_this(param: tree_sitter::Node, content: &[u8]) -> bool {
    let pk = param.kind();
    if pk == "self_parameter" || pk == "self" {
        return true;
    }
    if let Ok(text) = param.utf8_text(content) {
        let t = text.trim();
        matches!(t, "self" | "&self" | "&mut self" | "this")
    } else {
        false
    }
}

/// Check if a node kind represents a countable parameter.
fn is_parameter_kind(kind: &str) -> bool {
    matches!(kind,
        "parameter" | "formal_parameter"
        | "simple_parameter" | "typed_parameter"
        | "default_parameter" | "typed_default_parameter"
        | "identifier" | "required_parameter"
        | "optional_parameter" | "rest_parameter"
        | "spread_parameter" | "variadic_parameter"
        | "keyword_argument" | "list_splat_pattern"
        | "dictionary_splat_pattern"
    )
}

/// Count parameters in a parameter list node, excluding self/this.
fn count_params_in_list(param_list: tree_sitter::Node, content: &[u8]) -> u32 {
    let mut count = 0u32;
    for j in 0..param_list.named_child_count() {
        let param = param_list.named_child(j).unwrap();
        if is_self_or_this(param, content) { continue; }
        if is_parameter_kind(param.kind()) {
            count += 1;
        }
    }
    count
}

/// Count function parameters from a tree-sitter node, excluding self/this.
pub(crate) fn count_parameters(node: tree_sitter::Node, content: &[u8]) -> u32 {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if PARAM_LIST_KINDS.contains(&child.kind()) {
            return count_params_in_list(child, content);
        }
    }
    0
}

/// Compute a normalized body hash for duplication detection.
/// Strips whitespace and comments, then hashes the result.
pub(crate) fn hash_body(body: &str, lang: &str) -> u64 {
    let stripped = strip_strings_and_comments(body, lang);
    // Normalize: remove all whitespace for content-only comparison
    let normalized: String = stripped.chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if normalized.len() < 20 {
        // Too short to be meaningful duplication
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    hasher.finish()
}

