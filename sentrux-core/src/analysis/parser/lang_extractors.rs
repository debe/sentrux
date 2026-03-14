//! Base-class extraction helpers and module name transforms.
//!
//! Import extraction is now fully AST-based (ast_import_walker.rs) or
//! handled by @import.module query captures. No text-based import
//! extractors remain.
//!
//! Base class extraction: data-driven via base_class_node_kinds in plugin.toml.

// ── Module name transforms ──────────────────────────────────────────

/// Convert a dot-separated PascalCase module path to snake_case file path.
/// "Collect.Listing" → "collect/listing", "GenServer" → "gen_server"
/// Used by Elixir via `module_name_transform = "pascal_to_snake"` in plugin.toml.
pub(super) fn pascal_to_snake_path(module: &str) -> String {
    module.split('.').map(pascal_to_snake).collect::<Vec<_>>().join("/")
}

fn pascal_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                let prev = chars[i - 1];
                if prev.is_lowercase() || prev.is_ascii_digit()
                    || (prev.is_uppercase()
                        && chars.get(i + 1).is_some_and(|ch| ch.is_lowercase()))
                {
                    result.push('_');
                }
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

// ── Base-class extraction helpers ────────────────────────────────────

/// Collect base classes by matching child node kinds against a set of patterns.
/// Used by the data-driven `base_class_node_kinds` profile field.
pub(super) fn extract_bases_by_kinds(node: tree_sitter::Node, content: &[u8], kinds: &[&str], bases: &mut Vec<String>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if kinds.contains(&child.kind()) {
            collect_type_identifiers(child, content, bases);
        }
    }
}

/// Generic fallback: collect base classes from children whose kind contains inheritance keywords.
pub(super) fn extract_bases_generic(node: tree_sitter::Node, content: &[u8], bases: &mut Vec<String>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        let k = child.kind();
        if k.contains("superclass") || k.contains("extends")
            || k.contains("base_class") || k.contains("heritage")
        {
            collect_type_identifiers(child, content, bases);
        }
    }
}

fn is_type_identifier_kind(kind: &str) -> bool {
    matches!(kind, "type_identifier" | "identifier" | "constant" | "scope_resolution")
}

fn is_leaf_type_node(node: tree_sitter::Node) -> bool {
    is_type_identifier_kind(node.kind())
        && (node.child_count() == 0 || node.kind() == "scope_resolution")
}

fn is_visibility_keyword(name: &str) -> bool {
    matches!(name, "public" | "private" | "protected")
}

const MAX_TYPE_COLLECT_DEPTH: usize = 64;

fn collect_type_identifiers(node: tree_sitter::Node, content: &[u8], out: &mut Vec<String>) {
    collect_type_identifiers_inner(node, content, out, 0);
}

fn collect_type_identifiers_inner(node: tree_sitter::Node, content: &[u8], out: &mut Vec<String>, depth: usize) {
    if depth >= MAX_TYPE_COLLECT_DEPTH { return; }
    if is_leaf_type_node(node) {
        if let Ok(text) = node.utf8_text(content) {
            let name = text.trim().to_string();
            if !name.is_empty() && !is_visibility_keyword(&name) {
                out.push(name);
                return;
            }
        }
    }
    for i in 0..node.child_count() {
        collect_type_identifiers_inner(node.child(i).unwrap(), content, out, depth + 1);
    }
}
