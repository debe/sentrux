//! Entry-point detection and execution depth computation.
//!
//! Detects application entry points (main functions, HTTP handlers, CLI commands)
//! by inspecting file names, function signatures, and language conventions.
//! Computes execution depth via BFS over the import graph from entry points.

use super::lang_registry;
use crate::core::types::{EntryPoint, FileNode, ImportEdge};
use std::collections::{BTreeSet, HashMap, VecDeque};

/// BFS from entry points over the import graph to compute execution depth.
/// Uses BTreeSet for deterministic BFS order.
pub(crate) fn compute_exec_depth(
    import_edges: &[ImportEdge],
    entry_points: &[EntryPoint],
) -> HashMap<String, u32> {
    let mut exec_depth: HashMap<String, u32> = HashMap::new();
    let import_adjacency: HashMap<String, Vec<String>> = {
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for edge in import_edges {
            adj.entry(edge.from_file.clone())
                .or_default()
                .push(edge.to_file.clone());
        }
        adj
    };

    let entry_files: BTreeSet<String> = entry_points.iter().map(|e| e.file.clone()).collect();
    let mut queue: VecDeque<(String, u32)> =
        entry_files.iter().map(|f| (f.clone(), 0)).collect();

    for f in &entry_files {
        exec_depth.insert(f.clone(), 0);
    }

    while let Some((file, depth)) = queue.pop_front() {
        if let Some(deps) = import_adjacency.get(&file) {
            for dep in deps {
                if !exec_depth.contains_key(dep) {
                    exec_depth.insert(dep.clone(), depth + 1);
                    queue.push_back((dep.clone(), depth + 1));
                }
            }
        }
    }

    exec_depth
}

/// Whether a language can contain executable entry points.
/// Reads `is_executable` from the language profile (Layer 2).
/// Non-executable languages (html, css, scss, markdown, etc.) return false.
/// Unknown languages are conservatively allowed (may have entry points).
fn can_have_entry_points(lang: &str) -> bool {
    // Non-plugin languages (json, toml, yaml, xml, markdown) aren't executable
    const NON_EXECUTABLE_FALLBACKS: &[&str] = &["json", "toml", "yaml", "xml", "markdown"];
    if NON_EXECUTABLE_FALLBACKS.contains(&lang) {
        return false;
    }
    lang_registry::profile(lang).semantics.is_executable
}

/// Detect if a file is an entry point
pub(crate) fn detect_entry_points(file: &FileNode) -> Vec<EntryPoint> {
    if is_non_production_path(&file.path) {
        return Vec::new();
    }

    // Skip files whose language cannot have entry points (CSS, HTML, etc.)
    if !can_have_entry_points(&file.lang) {
        return Vec::new();
    }

    let mut entries = Vec::new();

    if is_main_entry_by_name(file) {
        entries.push(make_entry(file, "main"));
    }

    collect_sa_entry_points(file, &mut entries);

    entries
}

/// Returns true for test/example/benchmark/fixture/vendor directories.
fn is_non_production_path(path: &str) -> bool {
    let p = path.to_lowercase();
    const PREFIXES: &[&str] = &[
        "test/", "tests/", "test_",
        "example/", "examples/",
        "bench/", "benches/",
        "fixtures/", "vendor/",
    ];
    const INFIXES: &[&str] = &[
        "/test/", "/tests/",
        "/example/", "/examples/",
        "/bench/", "/benches/",
        "/fixtures/", "/vendor/",
    ];
    PREFIXES.iter().any(|pfx| p.starts_with(pfx))
        || INFIXES.iter().any(|inf| p.contains(inf))
}

/// Check if the file name matches a known main/app/server entry point pattern.
/// Uses `lang_registry::detect_lang_from_ext` for `main.*` files so that newly
/// registered languages (e.g., zig, elixir, haskell, scala) are automatically
/// recognized without maintaining a hardcoded extension list here.
fn is_main_entry_by_name(file: &FileNode) -> bool {
    let name_lower = file.name.to_lowercase();
    let path_depth = file.path.matches('/').count();
    let is_index = matches!(name_lower.as_str(), "index.ts" | "index.js" | "index.php");

    if is_index {
        return path_depth <= 1;
    }
    if name_lower.starts_with("main.") {
        // Use lang_registry to check if the extension belongs to a recognized
        // programming language, rather than maintaining a hardcoded list of every
        // `main.*` variant. This automatically supports new languages added to the
        // registry (e.g., main.zig, main.ex, main.hs, main.scala) without needing
        // to update this function.
        if let Some(ext) = name_lower.strip_prefix("main.") {
            let detected = lang_registry::detect_lang_from_ext(ext);
            return detected != "unknown" && can_have_entry_points(&detected);
        }
        return false;
    }
    // app.*/server.*/program.* only near project root (depth <= 2) [ref:daa66d13]
    // Check language profile for custom main filenames first
    let profile = lang_registry::profile(&file.lang);
    if !profile.semantics.main_filenames.is_empty() {
        return profile.semantics.main_filenames.iter().any(|mf| name_lower == mf.to_lowercase());
    }
    // Universal fallback: common app/server/program entry point patterns
    path_depth <= 2
        && matches!(
            name_lower.as_str(),
            "app.py" | "app.ts" | "app.js" | "app.rb" | "app.java"
            | "server.py" | "server.ts" | "server.js" | "server.go" | "server.rb"
            | "program.cs"
        )
}

/// Check if an entry point with the given func name already exists for this file.
fn has_entry(entries: &[EntryPoint], file_path: &str, func: &str) -> bool {
    entries.iter().any(|e| e.file == file_path && e.func == func)
}

/// Add entry point if not already present.
fn add_entry_if_new(file: &FileNode, func: &str, entries: &mut Vec<EntryPoint>) {
    if !has_entry(entries, &file.path, func) {
        entries.push(make_entry(file, func));
    }
}

/// Collect entry points from structural analysis tags and functions.
fn collect_sa_entry_points(file: &FileNode, entries: &mut Vec<EntryPoint>) {
    let sa = match &file.sa {
        Some(sa) => sa,
        None => return,
    };

    if let Some(sa_tags) = &sa.tags {
        for tag in sa_tags {
            add_entry_if_new(file, tag, entries);
        }
    }

    if let Some(fns) = &sa.functions {
        if fns.iter().any(|f| f.n == "main") {
            add_entry_if_new(file, "main", entries);
        }
    }
}

fn make_entry(file: &FileNode, func: &str) -> EntryPoint {
    EntryPoint {
        file: file.path.clone(),
        func: func.to_string(),
        lang: file.lang.clone(),
        confidence: "high".to_string(),
    }
}
