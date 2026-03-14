//! Tree-sitter capture classification and processing helpers.
//!
//! Extracted from parser.rs to keep that module under 500 lines.
//! Contains the two-pass capture classification logic, entry-tag detection,
//! and per-match-kind processing (func def, class def, import, call).

use super::imports::{
    count_complexity, count_cognitive_complexity,
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
            if !is_cfg_test_mod(cap.node, content) {
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
            classify_entry_tag(cap.node, content, tags, tag_set);
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
        process_single_capture(cname, cap, content, &mut r, imports, import_set, tags, tag_set);
    }
    r
}

/// Check if an attribute_item node contains cfg(test).
fn is_cfg_test_attribute(sib: tree_sitter::Node, content: &[u8]) -> bool {
    if let Ok(text) = sib.utf8_text(content) {
        text.contains("cfg") && text.contains("test")
    } else {
        false
    }
}

/// Check if a tree-sitter node is a `mod` declaration preceded by `#[cfg(test)]`.
/// In the Rust AST, `#[cfg(test)] mod tests;` produces:
///   attribute_item  <- sibling
///   mod_item        <- our node
/// Test modules are not production dependencies -- including them creates
/// false mutual edges (mod->test, test->mod) that inflate upward violations.
fn is_cfg_test_mod(node: tree_sitter::Node, content: &[u8]) -> bool {
    if node.kind() != "mod_item" {
        return false;
    }
    let mut sibling = node.prev_sibling();
    while let Some(sib) = sibling {
        if sib.kind() != "attribute_item" {
            break;
        }
        if is_cfg_test_attribute(sib, content) {
            return true;
        }
        sibling = sib.prev_sibling();
    }
    false
}

/// Map an entry-point tag line to its canonical label.
fn entry_tag_label(tag: &str) -> Option<&'static str> {
    if tag.contains("@main") || tag.contains("@Main") || tag.contains("@UIApplicationMain") {
        return Some("@main");
    }
    if tag.contains("__main__") {
        return Some("__main__");
    }
    if tag.contains("tokio::main") || tag.contains("actix_web::main") {
        return Some("@async_main");
    }
    if tag.contains("#[main]") {
        return Some("@main");
    }
    None
}

fn classify_entry_tag(
    node: tree_sitter::Node,
    content: &[u8],
    tags: &mut Vec<String>,
    tag_set: &mut HashSet<String>,
) {
    let text = match node.utf8_text(content) {
        Ok(t) => t,
        Err(_) => return,
    };
    let tag = text.lines().next().unwrap_or(text).trim();
    if let Some(label) = entry_tag_label(tag) {
        if tag_set.insert(label.to_string()) {
            tags.push(label.to_string());
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
            // AST-based: walk tree-sitter nodes directly (no text scanning)
            let cc = count_complexity_ast(node, pctx.content, profile);
            let cog = count_cognitive_complexity_ast(node, pctx.content, profile);
            (cc, cog)
        } else {
            // Legacy fallback: text-based keyword scanning
            let cc = count_complexity(body, pctx.lang);
            let cog = count_cognitive_complexity(body, pctx.lang);
            (cc, cog)
        };
        let pc = count_parameters(node, pctx.content);
        let bh = hash_body(body, pctx.lang);
        functions.push(FuncInfo {
            n: name, sl, el, ln,
            cc: Some(cc),
            cog: Some(cog),
            pc: Some(pc),
            bh: if bh != 0 { Some(bh) } else { None },
            d: None, co: None,
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
        if profile.semantics.import_ast.is_configured() {
            // AST-based: walk tree-sitter nodes directly
            let paths = super::ast_import_walker::extract_imports_from_ast(
                node, content, &profile.semantics.import_ast,
            );
            for raw in paths {
                insert_normalized(&raw, dots_are_seps, imports, import_set);
            }
        }
        // Languages without import_ast configured rely on @import.module
        // captures (branch 1 above). If neither is configured, no imports
        // are extracted — the plugin needs to be updated.
    }
}
