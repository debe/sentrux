//! Tree-sitter capture classification and processing helpers.
//!
//! Extracted from parser.rs to keep that module under 500 lines.
//! Contains the two-pass capture classification logic, entry-tag detection,
//! and per-match-kind processing (func def, class def, import, call).

use super::imports::{
    count_complexity_ast, count_cognitive_complexity_ast,
    count_parameters, hash_body,
    extract_base_classes,
    lang_uses_dot_separator, normalize_module_path,
};
use crate::core::types::{ClassInfo, FuncInfo};
use std::collections::HashSet;

/// Match classification for two-pass capture processing.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum MatchKind {
    FuncDef,
    ClassDef,
    Import,
    Call,
}

pub(super) struct CaptureResult<'a> {
    pub(super) match_type: Option<MatchKind>,
    pub(super) match_node: Option<tree_sitter::Node<'a>>,
    pub(super) name_text: Option<String>,
    pub(super) class_kind: Option<&'a str>,
    pub(super) import_module_text: Option<String>,
    pub(super) import_node: Option<tree_sitter::Node<'a>>,
    pub(super) call_line: u32,
}

/// Set the result to a class definition with the given kind.
fn set_class_def<'a>(r: &mut CaptureResult<'a>, node: tree_sitter::Node<'a>, kind: &'static str) {
    r.match_type = Some(MatchKind::ClassDef);
    r.match_node = Some(node);
    r.class_kind = Some(kind);
}

/// Process a scoped call path, extracting the module portion as an import.
fn process_scoped_path(
    node: tree_sitter::Node,
    content: &[u8],
    imports: &mut Vec<String>,
    import_set: &mut HashSet<String>,
) {
    if let Ok(path_text) = node.utf8_text(content) {
        if let Some(last_sep) = path_text.rfind("::") {
            let module_part = &path_text[..last_sep];
            let normalized = normalize_module_path(module_part, false);
            if !normalized.is_empty() && import_set.insert(normalized.clone()) {
                imports.push(normalized);
            }
        }
    }
}

/// Process a single capture, updating the result accordingly.
fn process_single_capture<'a>(
    cname: &str,
    cap: &tree_sitter::QueryCapture<'a>,
    content: &[u8],
    lang: &str,
    r: &mut CaptureResult<'a>,
    imports: &mut Vec<String>,
    import_set: &mut HashSet<String>,
    tags: &mut Vec<String>,
    tag_set: &mut HashSet<String>,
) {
    match cname {
        "definition.function" | "definition.method" | "func.def" => {
            r.match_type = Some(MatchKind::FuncDef);
            r.match_node = Some(cap.node);
        }
        "definition.class" => set_class_def(r, cap.node, "class"),
        "definition.interface" => set_class_def(r, cap.node, "interface"),
        "definition.adt" => set_class_def(r, cap.node, "adt"),
        "definition.type" => set_class_def(r, cap.node, "type"),
        "class.def" => {
            r.match_type = Some(MatchKind::ClassDef);
            r.match_node = Some(cap.node);
            if r.class_kind.is_none() {
                r.class_kind = Some("class");
            }
        }
        "reference.call" | "reference.class" | "reference.send" | "call" => {
            if r.match_type.is_none() {
                r.match_type = Some(MatchKind::Call);
                r.call_line = cap.node.start_position().row as u32 + 1;
            }
        }
        "name" | "func.name" | "class.name" | "call.name" | "mod.name" => {
            r.name_text = cap.node.utf8_text(content).ok().map(|s| s.to_string());
        }
        "import" => {
            if !is_test_mod(cap.node, content, lang) {
                r.match_type = Some(MatchKind::Import);
                r.import_node = Some(cap.node);
            }
        }
        "import.module" => {
            r.import_module_text = cap.node.utf8_text(content).ok().map(|s| {
                s.trim_matches(|c: char| c == '"' || c == '\'').to_string()
            });
        }
        "call.scoped_path" => {
            process_scoped_path(cap.node, content, imports, import_set);
        }
        "entry" | "entry.point" => {
            classify_entry_tag(cap.node, content, lang, tags, tag_set);
        }
        // Ignored capture names
        "definition.module" | "definition.macro" | "definition.constant"
        | "definition.field" | "definition.property"
        | "reference.implementation" | "reference.type" | "reference.interface"
        | "doc" | "ignore" | "local.scope" | "module" => {}
        _ => {}
    }
}

pub(super) fn classify_captures<'a>(
    captures: &'a [tree_sitter::QueryCapture<'a>],
    capture_names: &[&str],
    content: &[u8],
    lang: &str,
    imports: &mut Vec<String>,
    import_set: &mut HashSet<String>,
    tags: &mut Vec<String>,
    tag_set: &mut HashSet<String>,
) -> CaptureResult<'a> {
    let mut r = CaptureResult {
        match_type: None,
        match_node: None,
        name_text: None,
        class_kind: None,
        import_module_text: None,
        import_node: None,
        call_line: 0,
    };

    for cap in captures {
        let cname = capture_names[cap.index as usize];
        process_single_capture(cname, cap, content, lang, &mut r, imports, import_set, tags, tag_set);
    }
    r
}

/// Check if an attribute node matches all test attribute patterns.
/// Generic: patterns come from plugin TOML `test_attribute_patterns`.
fn is_test_attribute(sib: tree_sitter::Node, content: &[u8], patterns: &[String]) -> bool {
    if patterns.is_empty() { return false; }
    if let Ok(text) = sib.utf8_text(content) {
        patterns.iter().all(|p| text.contains(p.as_str()))
    } else {
        false
    }
}

/// Check if a tree-sitter node is a test module declaration preceded by a test attribute.
/// Configured via test_module_kind and test_attribute_kind in plugin TOML.
/// Test modules are not production dependencies -- including them creates
/// false mutual edges that inflate upward violations.
fn is_test_mod(node: tree_sitter::Node, content: &[u8], lang: &str) -> bool {
    let profile = crate::analysis::lang_registry::profile(lang);
    let sem = &profile.semantics;
    if sem.test_module_kind.is_empty() || sem.test_attribute_kind.is_empty() {
        return false;
    }
    if node.kind() != sem.test_module_kind {
        return false;
    }
    let mut sibling = node.prev_sibling();
    while let Some(sib) = sibling {
        if sib.kind() != sem.test_attribute_kind {
            break;
        }
        if is_test_attribute(sib, content, &sem.test_attribute_patterns) {
            return true;
        }
        sibling = sib.prev_sibling();
    }
    false
}

/// Map an entry-point tag line to its canonical label.
/// Checks against entry_point_patterns from the language's plugin TOML.
/// Falls back to a universal "@main" label if any configured pattern matches.
fn entry_tag_label(tag: &str, lang: &str) -> Option<String> {
    let profile = crate::analysis::lang_registry::profile(lang);
    let patterns = &profile.semantics.entry_point_patterns;
    if patterns.is_empty() {
        return None;
    }
    for pattern in patterns {
        if tag.contains(pattern.as_str()) {
            // Use pattern as label, or "@main" for common patterns
            if pattern.contains("main") {
                return Some("@main".to_string());
            }
            return Some(pattern.clone());
        }
    }
    None
}

fn classify_entry_tag(
    node: tree_sitter::Node,
    content: &[u8],
    lang: &str,
    tags: &mut Vec<String>,
    tag_set: &mut HashSet<String>,
) {
    let text = match node.utf8_text(content) {
        Ok(t) => t,
        Err(_) => return,
    };
    let tag = text.lines().next().unwrap_or(text).trim();
    if let Some(label) = entry_tag_label(tag, lang) {
        if tag_set.insert(label.clone()) {
            tags.push(label);
        }
    }
}

/// Shared context for parsing a single file — bundles the file content and
/// language that every process_func_def / process_class_def call needs.
pub(super) struct ParseContext<'a> {
    pub content: &'a [u8],
    pub lang: &'a str,
}

pub(super) fn process_func_def(
    name: String,
    match_node: Option<tree_sitter::Node>,
    fallback_node: tree_sitter::Node,
    pctx: &ParseContext<'_>,
    functions: &mut Vec<FuncInfo>,
    func_set: &mut HashSet<(String, u32)>,
) {
    let node = match_node.unwrap_or(fallback_node);
    let sl = node.start_position().row as u32 + 1;
    if func_set.insert((name.clone(), sl)) {
        let el = node.end_position().row as u32 + 1;
        let ln = el - sl + 1;
        let body = node.utf8_text(pctx.content).unwrap_or("");
        let profile = crate::analysis::lang_registry::profile(pctx.lang);
        let (cc, cog) = if profile.semantics.complexity.is_configured() {
            // AST-based: walk tree-sitter nodes directly
            let cc = count_complexity_ast(node, pctx.content, profile);
            let cog = count_cognitive_complexity_ast(node, pctx.content, profile);
            (cc, cog)
        } else {
            // No complexity config → CC=1 (base path), COG=0
            // Languages must declare branch_nodes in plugin.toml to get complexity analysis.
            (1u32, 0u32)
        };
        let pc = count_parameters(node, pctx.content, pctx.lang);
        let bh = hash_body(body, pctx.lang);
        // Detect visibility from function text — TOML-driven via public_keywords
        let is_public = {
            let keywords = &profile.semantics.public_keywords;
            if keywords.is_empty() {
                false
            } else {
                let text = body.trim_start();
                keywords.iter().any(|kw| text.starts_with(kw.as_str()))
            }
        };
        functions.push(FuncInfo {
            n: name, sl, el, ln,
            cc: Some(cc),
            cog: Some(cog),
            pc: Some(pc),
            bh: if bh != 0 { Some(bh) } else { None },
            d: None, co: None,
            is_public,
        });
    }
}

pub(super) fn process_class_def(
    name_text: Option<String>,
    match_node: Option<tree_sitter::Node>,
    class_kind: Option<&str>,
    pctx: &ParseContext<'_>,
    classes: &mut Vec<ClassInfo>,
) {
    let name = name_text.unwrap_or_else(|| {
        match_node.map(|n| n.kind().to_string()).unwrap_or_default()
    });
    if !name.is_empty() {
        let bases = match_node.and_then(|node| extract_base_classes(node, pctx.content, pctx.lang));
        classes.push(ClassInfo {
            n: name, m: None, b: bases,
            k: class_kind.map(|s| s.to_string()),
        });
    }
}

/// Apply module name transform from plugin profile (e.g., Elixir PascalCase→snake_case).
fn apply_module_transform(module: &str, transform: &str) -> String {
    match transform {
        "pascal_to_snake" => super::lang_extractors::pascal_to_snake_path(module),
        _ => module.to_string(),
    }
}

/// Insert a normalized module path into imports if non-empty and not seen.
fn insert_normalized(raw: &str, dots_are_seps: bool, imports: &mut Vec<String>, import_set: &mut HashSet<String>) {
    let module = normalize_module_path(raw, dots_are_seps);
    if !module.is_empty() && import_set.insert(module.clone()) {
        imports.push(module);
    }
}

/// Context for processing a single import match — groups the captured fields
/// from classify_captures that are forwarded to process_import.
pub(super) struct ImportContext<'a> {
    pub import_module_text: Option<String>,
    pub name_text: Option<String>,
    pub import_node: Option<tree_sitter::Node<'a>>,
    pub match_node: Option<tree_sitter::Node<'a>>,
}

pub(super) fn process_import(
    ictx: &ImportContext<'_>,
    lang: &str,
    content: &[u8],
    imports: &mut Vec<String>,
    import_set: &mut HashSet<String>,
) {
    let profile = crate::analysis::lang_registry::profile(lang);
    let dots_are_seps = lang_uses_dot_separator(lang);
    let transform = &profile.semantics.import_ast.module_name_transform;
    if let Some(module) = &ictx.import_module_text {
        let module = apply_module_transform(module, transform);
        insert_normalized(&module, dots_are_seps, imports, import_set);
    } else if let Some(module) = &ictx.name_text {
        let module = apply_module_transform(module, transform);
        insert_normalized(&module, dots_are_seps, imports, import_set);
    } else if let Some(node) = ictx.import_node.or(ictx.match_node) {
        // Generic brace expansion: Prefix.{A, B} → Prefix.A, Prefix.B
        // Only for non-scoped_path languages — scoped_path (Rust, Java) has its
        // own use_list handling that's AST-aware (type vs module filtering).
        let strategy = &profile.semantics.import_ast.strategy;
        if strategy != "scoped_path" {
            if let Ok(text) = node.utf8_text(content) {
                let expanded = super::imports::expand_braces(text);
                if !expanded.is_empty() {
                    for raw in &expanded {
                        let module = apply_module_transform(raw, transform);
                        insert_normalized(&module, dots_are_seps, imports, import_set);
                    }
                    return;
                }
            }
        }
        // AST-based: walk tree-sitter nodes directly
        if profile.semantics.import_ast.is_configured() {
            let paths = super::ast_import_walker::extract_imports_from_ast(
                node, content, &profile.semantics.import_ast,
            );
            for raw in paths {
                insert_normalized(&raw, dots_are_seps, imports, import_set);
            }
        }
    }
}
