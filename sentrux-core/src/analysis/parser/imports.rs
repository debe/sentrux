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

// ── Base class extraction ───────────────────────────────────────────────

/// Extract base/parent class names from a class definition AST node.
///
/// Uses three strategies in order:
/// 1. Profile `base_class_node_kinds` (data-driven, covers most languages)
/// 2. Compiled `base_class_extractor` (for Python which needs special handling)
/// 3. Generic fallback (pattern-match on node kind names)
pub(crate) fn extract_base_classes(node: tree_sitter::Node, content: &[u8], lang: &str) -> Option<Vec<String>> {
    let profile = crate::analysis::lang_registry::profile(lang);
    let mut bases = Vec::new();

    if !profile.semantics.base_class_node_kinds.is_empty() {
        // Data-driven: use node kinds from plugin.toml
        let kinds: Vec<&str> = profile.semantics.base_class_node_kinds.iter().map(|s| s.as_str()).collect();
        lang_extractors::extract_bases_by_kinds(node, content, &kinds, &mut bases);
    } else {
        // Generic fallback: pattern-match on node kind substrings
        lang_extractors::extract_bases_generic(node, content, &mut bases);
    }

    if bases.is_empty() { None } else { Some(bases) }
}

// Legacy text-based complexity counting has been removed.
// All complexity analysis is now AST-based via count_complexity_ast()
// and count_cognitive_complexity_ast() using branch_nodes/logic_nodes
// from plugin.toml [semantics.complexity].

// ── AST-based complexity counting ─────────────────────────────────────
// These functions walk the tree-sitter AST directly instead of scanning text.
// They use node kinds from the language profile (plugin.toml [semantics.complexity]).

use std::collections::HashSet as CxHashSet;

/// Check if a node's operator text matches one of the logic operators.
fn is_logic_operator(node: tree_sitter::Node, content: &[u8], operators: &[String]) -> bool {
    if operators.is_empty() {
        return true; // No filter = count all logic_nodes
    }
    // Check the "operator" field first (many grammars have it)
    if let Some(op_node) = node.child_by_field_name("operator") {
        if let Ok(op_text) = op_node.utf8_text(content) {
            return operators.iter().any(|op| op == op_text.trim());
        }
    }
    // Fallback: check if any child is one of the operators
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if !child.is_named() {
                if let Ok(text) = child.utf8_text(content) {
                    if operators.iter().any(|op| op == text.trim()) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Count nesting depth of a node by walking up to the function root.
/// Only counts ancestors whose kind is in `nesting_set`.
fn nesting_depth(
    node: tree_sitter::Node,
    func_node: tree_sitter::Node,
    nesting_set: &CxHashSet<&str>,
) -> u32 {
    let mut depth = 0u32;
    let mut current = node.parent();
    let func_id = func_node.id();
    while let Some(p) = current {
        if p.id() == func_id {
            break; // Don't count beyond the function boundary
        }
        if nesting_set.contains(p.kind()) {
            depth += 1;
        }
        current = p.parent();
    }
    depth
}

/// Walk the AST subtree of a function node and compute cyclomatic complexity.
/// CC = 1 + (number of branch_nodes) + (number of logic_nodes with matching operator).
pub(crate) fn count_complexity_ast(
    func_node: tree_sitter::Node,
    content: &[u8],
    profile: &crate::analysis::plugin::profile::LanguageProfile,
) -> u32 {
    let cx = &profile.semantics.complexity;
    let branch_set: CxHashSet<&str> = cx.branch_nodes.iter().map(|s| s.as_str()).collect();
    let logic_set: CxHashSet<&str> = cx.logic_nodes.iter().map(|s| s.as_str()).collect();

    let mut cc = 1u32; // Base path
    let mut cursor = func_node.walk();

    // DFS walk of the subtree
    let mut visited_root = false;
    loop {
        if !visited_root {
            visited_root = true;
        }
        let node = cursor.node();

        if branch_set.contains(node.kind()) {
            cc += 1;
        } else if logic_set.contains(node.kind()) {
            if is_logic_operator(node, content, &cx.logic_operators) {
                cc += 1;
            }
        }

        // Descend into children
        if cursor.goto_first_child() {
            continue;
        }
        // Move to next sibling
        if cursor.goto_next_sibling() {
            continue;
        }
        // Go up and try next sibling
        loop {
            if !cursor.goto_parent() {
                // Back at root — done
                return cc;
            }
            if cursor.node().id() == func_node.id() {
                return cc;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Walk the AST subtree of a function node and compute cognitive complexity.
/// COG = sum of (1 + nesting_depth) for each branch node + 1 for each logic operator.
pub(crate) fn count_cognitive_complexity_ast(
    func_node: tree_sitter::Node,
    content: &[u8],
    profile: &crate::analysis::plugin::profile::LanguageProfile,
) -> u32 {
    let cx = &profile.semantics.complexity;
    let branch_set: CxHashSet<&str> = cx.branch_nodes.iter().map(|s| s.as_str()).collect();
    let logic_set: CxHashSet<&str> = cx.logic_nodes.iter().map(|s| s.as_str()).collect();
    let nesting_set: CxHashSet<&str> = cx.nesting_nodes.iter().map(|s| s.as_str()).collect();

    let mut cog = 0u32;
    let mut cursor = func_node.walk();

    let mut visited_root = false;
    loop {
        if !visited_root {
            visited_root = true;
        }
        let node = cursor.node();

        if branch_set.contains(node.kind()) {
            let depth = nesting_depth(node, func_node, &nesting_set);
            cog += 1 + depth;
        } else if logic_set.contains(node.kind()) {
            if is_logic_operator(node, content, &cx.logic_operators) {
                cog += 1;
            }
        }

        if cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }
        loop {
            if !cursor.goto_parent() {
                return cog;
            }
            if cursor.node().id() == func_node.id() {
                return cog;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
    }
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

