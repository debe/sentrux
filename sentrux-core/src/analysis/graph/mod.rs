//! Cross-file graph construction — import, call, and inheritance edges.
//!
//! Takes a flat list of parsed `FileNode` references and builds three
//! dependency graphs plus entry-point detection and execution depth.
//! Import resolution uses oxc_resolver for JS/TS and suffix-index for others.

use super::entry_points::{compute_exec_depth, detect_entry_points};
use super::resolver::suffix::resolve_path_imports_ref;
use crate::core::types::{CallEdge, EntryPoint, FileNode, ImportEdge, InheritEdge};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Interface for building cross-file dependency graphs from parsed file nodes.
/// Enables alternative implementations for testing or incremental graph updates.
pub trait GraphBuilder {
    /// Build all dependency graphs from a set of parsed files.
    fn build(&self, files: &[&FileNode], scan_root: Option<&Path>, max_call_targets: usize) -> GraphResult;
}

/// Result of `build_graphs`: all three dependency graphs plus entry points.
/// Replaces the fragile 5-tuple return type with named fields.
pub struct GraphResult {
    /// File-to-file import edges (resolved from import/require statements)
    pub import_edges: Vec<ImportEdge>,
    /// Function-to-function call edges (restricted to imported files)
    pub call_edges: Vec<CallEdge>,
    /// Class inheritance edges (child extends/implements parent)
    pub inherit_edges: Vec<InheritEdge>,
    /// Detected application entry points (main, handlers, CLI commands)
    pub entry_points: Vec<EntryPoint>,
    /// BFS distance from entry points (0 = entry point, higher = deeper)
    pub exec_depth: HashMap<String, u32>,
}

/// Build cross-file graphs from a flat list of file references with structural analysis.
/// Zero-copy: accepts `&[&FileNode]` from `flatten_files_ref` to avoid cloning the tree.
///
/// Import edges come from two tiers:
///   Tier 1: oxc_resolver for JS/TS (sync, <100ms)
///   Tier 2: suffix-index + file-path join for everything else (sync, <10ms)
///
/// `scan_root` enables path resolution. Without it, no import edges are produced.
pub fn build_graphs(
    files: &[&FileNode],
    scan_root: Option<&Path>,
    max_call_targets: usize,
) -> GraphResult {
    let t0 = std::time::Instant::now();

    let (lang_map, func_map, class_map) = build_lookup_maps(files);
    let t_maps = t0.elapsed();

    let mut import_edges = resolve_path_imports_ref(files, scan_root);

    // Merge jdeps bytecode-level edges for Java projects
    if let Some(root) = scan_root {
        if let Some(jdeps_result) = super::resolver::jdeps::resolve_jdeps_edges(root, files, &[]) {
            let before = import_edges.len();
            import_edges.extend(jdeps_result.edges);
            crate::debug_log!(
                "[graph] jdeps: +{} edges ({} unmapped)",
                import_edges.len() - before,
                jdeps_result.unmapped_count
            );
        }
    }

    let t_imports = t0.elapsed();

    // Dedup BEFORE building target index to avoid wasted allocations
    dedup_import_edges(&mut import_edges);

    let import_targets = build_import_target_index(&import_edges);
    let call_edges = compute_call_edges(files, &lang_map, &func_map, &class_map, &import_targets, max_call_targets);
    let inherit_edges = compute_inherit_edges(files, &lang_map, &class_map, &import_targets);
    let entry_points = collect_entry_points(files);
    let exec_depth = compute_exec_depth(&import_edges, &entry_points);

    log_build_graphs_timing(files.len(), &t0, t_maps, t_imports, &import_edges, &call_edges, &inherit_edges);

    GraphResult { import_edges, call_edges, inherit_edges, entry_points, exec_depth }
}

/// Build lookup maps for language, functions, and classes from file nodes.
/// Zero-copy: borrows from the files slice which outlives these maps.
/// Lookup maps: (lang_map, func_map, class_map).
type LookupMaps<'a> = (HashMap<&'a str, &'a str>, HashMap<&'a str, Vec<&'a str>>, HashMap<&'a str, Vec<&'a str>>);

/// Index functions and classes from a file's structural analysis into lookup maps.
fn index_file_symbols<'a>(
    file: &'a FileNode,
    func_map: &mut HashMap<&'a str, Vec<&'a str>>,
    class_map: &mut HashMap<&'a str, Vec<&'a str>>,
) {
    let sa = match &file.sa {
        Some(sa) => sa,
        None => return,
    };
    if let Some(fns) = &sa.functions {
        for f in fns {
            func_map.entry(f.n.as_str()).or_default().push(&file.path);
        }
    }
    if let Some(classes) = &sa.cls {
        for c in classes {
            class_map.entry(c.n.as_str()).or_default().push(&file.path);
        }
    }
}

fn build_lookup_maps<'a>(
    files: &[&'a FileNode],
) -> LookupMaps<'a> {
    let mut lang_map: HashMap<&str, &str> = HashMap::new();
    let mut func_map: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut class_map: HashMap<&str, Vec<&str>> = HashMap::new();

    for file in files {
        if file.is_dir {
            continue;
        }
        lang_map.insert(&file.path, &file.lang);
        index_file_symbols(file, &mut func_map, &mut class_map);
    }
    (lang_map, func_map, class_map)
}

/// Build a from_file → {to_file} index from import edges.
fn build_import_target_index(import_edges: &[ImportEdge]) -> HashMap<&str, HashSet<&str>> {
    let mut m: HashMap<&str, HashSet<&str>> = HashMap::new();
    for edge in import_edges {
        m.entry(edge.from_file.as_str())
            .or_default()
            .insert(edge.to_file.as_str());
    }
    m
}

/// Dedup import edges: two specifiers can resolve to the same target. [ref:daa66d13]
fn dedup_import_edges(import_edges: &mut Vec<ImportEdge>) {
    let mut seen: HashSet<(&str, &str)> = HashSet::with_capacity(import_edges.len());
    // Safety: we borrow from `import_edges` elements which are not moved/dropped by `retain`
    // until after the `seen` set is done being used. We use raw pointers to work around
    // the borrow checker since `retain` takes `&mut self` but we need shared refs to elements.
    // Instead, use a two-pass approach: mark indices to keep, then retain.
    let mut keep = vec![false; import_edges.len()];
    for (i, e) in import_edges.iter().enumerate() {
        if seen.insert((e.from_file.as_str(), e.to_file.as_str())) {
            keep[i] = true;
        }
    }
    let mut idx = 0;
    import_edges.retain(|_| {
        let k = keep[idx];
        idx += 1;
        k
    });
}

/// Detect entry points across all non-directory files.
fn collect_entry_points(files: &[&FileNode]) -> Vec<EntryPoint> {
    let mut entry_points = Vec::new();
    for &file in files {
        if !file.is_dir {
            entry_points.extend(detect_entry_points(file));
        }
    }
    entry_points
}

/// Log timing breakdown for build_graphs.
fn log_build_graphs_timing(
    file_count: usize,
    t0: &std::time::Instant,
    t_maps: std::time::Duration,
    t_imports: std::time::Duration,
    import_edges: &[ImportEdge],
    call_edges: &[CallEdge],
    inherit_edges: &[InheritEdge],
) {
    let t_total = t0.elapsed();
    eprintln!(
        "[build_graphs] {} files | maps {:.1}ms, imports {:.1}ms, calls+inherit {:.1}ms, total {:.1}ms | {} import, {} call, {} inherit edges",
        file_count,
        t_maps.as_secs_f64() * 1000.0,
        (t_imports - t_maps).as_secs_f64() * 1000.0,
        (t_total - t_imports).as_secs_f64() * 1000.0,
        t_total.as_secs_f64() * 1000.0,
        import_edges.len(),
        call_edges.len(),
        inherit_edges.len(),
    );
}

/// Resolve a single call to target files, filtering by language and import relationship.
fn resolve_call_targets<'a>(
    call_name: &str,
    file_path: &str,
    src_lang: &str,
    func_map: &HashMap<&'a str, Vec<&'a str>>,
    lang_map: &HashMap<&'a str, &'a str>,
    imported_files: Option<&HashSet<&'a str>>,
    max_call_targets: usize,
    implicit_module: bool,
) -> Vec<&'a str> {
    let targets = match func_map.get(call_name) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let same_lang: Vec<&str> = targets
        .iter()
        .filter(|t| {
            **t != file_path
                && lang_map.get(*t).copied().unwrap_or("") == src_lang
                && (implicit_module || imported_files.is_some_and(|imp| imp.contains(*t)))
        })
        .copied()
        .collect();
    if same_lang.len() <= max_call_targets { same_lang } else { Vec::new() }
}

/// Compute call edges between files connected by import edges.
/// Only emits edges where the caller imports the callee (proof of intent).
fn compute_call_edges<'a>(
    files: &[&'a FileNode],
    lang_map: &HashMap<&'a str, &'a str>,
    func_map: &HashMap<&'a str, Vec<&'a str>>,
    class_map: &HashMap<&'a str, Vec<&'a str>>,
    import_targets: &HashMap<&'a str, HashSet<&'a str>>,
    max_call_targets: usize,
) -> Vec<CallEdge> {
    let mut all_edges: Vec<CallEdge> = files
        .par_iter()
        .filter(|file| !file.is_dir)
        .flat_map(|file| {
            let mut edges = Vec::new();
            let src_lang = lang_map.get(file.path.as_str()).copied().unwrap_or("");
            if src_lang.is_empty() {
                return edges;
            }
            let imported_files = import_targets.get(file.path.as_str());
            // Check if this language has implicit module visibility (e.g., Swift)
            let profile = crate::analysis::lang_registry::profile(src_lang);
            let implicit = profile.semantics.project.implicit_module;
            let sa = match &file.sa {
                Some(sa) => sa,
                None => return edges,
            };

            let mut emit_call = |from_func: &str, call_name: &str| {
                // Match against function names
                let mut targets = resolve_call_targets(
                    call_name, &file.path, src_lang, func_map, lang_map, imported_files, max_call_targets, implicit,
                );
                // Also match against class/type names — type references are dependencies too
                if targets.is_empty() {
                    targets = resolve_call_targets(
                        call_name, &file.path, src_lang, class_map, lang_map, imported_files, max_call_targets, implicit,
                    );
                }
                for target_file in targets {
                    edges.push(CallEdge {
                        from_file: file.path.clone(),
                        from_func: from_func.to_string(),
                        to_file: target_file.to_string(),
                        to_func: call_name.to_string(),
                    });
                }
            };

            if let Some(fns) = &sa.functions {
                for func in fns {
                    for call_name in func.co.iter().flatten() {
                        emit_call(&func.n, call_name);
                    }
                }
            }
            for call_name in sa.co.iter().flatten() {
                emit_call("", call_name);
            }
            edges
        })
        .collect();
    // Sort for deterministic output regardless of par_iter ordering. [H2 fix]
    all_edges.sort_unstable_by(|a, b| {
        a.from_file.cmp(&b.from_file)
            .then_with(|| a.from_func.cmp(&b.from_func))
            .then_with(|| a.to_file.cmp(&b.to_file))
            .then_with(|| a.to_func.cmp(&b.to_func))
    });
    all_edges
}

/// Check if a parent file is a valid inheritance target: different file,
/// same language, and imported by the child file.
fn is_valid_inherit_target(
    parent_file: &str,
    child_path: &str,
    src_lang: &str,
    lang_map: &HashMap<&str, &str>,
    imported_files: Option<&HashSet<&str>>,
) -> bool {
    parent_file != child_path
        && lang_map.get(parent_file).copied().unwrap_or("") == src_lang
        && imported_files.is_some_and(|imp| imp.contains(parent_file))
}

/// Lookup context for resolving inheritance targets — groups the shared
/// maps and per-file context that every class in the file needs.
struct InheritLookup<'a, 'b> {
    file_path: &'a str,
    src_lang: &'a str,
    lang_map: &'b HashMap<&'a str, &'a str>,
    class_map: &'b HashMap<&'a str, Vec<&'a str>>,
    imported_files: Option<&'b HashSet<&'a str>>,
}

/// Collect inheritance edges for a single class's base classes.
fn collect_class_inherit_edges(
    lk: &InheritLookup<'_, '_>,
    cls: &crate::core::types::ClassInfo,
    edges: &mut Vec<InheritEdge>,
) {
    let bases = match &cls.b {
        Some(b) => b,
        None => return,
    };
    for base_name in bases {
        let parent_files = match lk.class_map.get(base_name.as_str()) {
            Some(pf) => pf,
            None => continue,
        };
        for &parent_file in parent_files {
            if is_valid_inherit_target(parent_file, lk.file_path, lk.src_lang, lk.lang_map, lk.imported_files) {
                edges.push(InheritEdge {
                    child_file: lk.file_path.to_string(),
                    child_class: cls.n.clone(),
                    parent_file: parent_file.to_string(),
                    parent_class: base_name.clone(),
                });
            }
        }
    }
}

/// Compute inheritance edges between files connected by import edges.
/// Only emits edges where the child file imports the parent file.
fn compute_inherit_edges<'a>(
    files: &[&'a FileNode],
    lang_map: &HashMap<&'a str, &'a str>,
    class_map: &HashMap<&'a str, Vec<&'a str>>,
    import_targets: &HashMap<&'a str, HashSet<&'a str>>,
) -> Vec<InheritEdge> {
    let mut all_edges: Vec<InheritEdge> = files
        .par_iter()
        .filter(|file| !file.is_dir)
        .flat_map(|file| {
            let mut edges = Vec::new();
            let src_lang = lang_map.get(file.path.as_str()).copied().unwrap_or("");
            if src_lang.is_empty() {
                return edges;
            }
            let imported_files = import_targets.get(file.path.as_str());
            let classes = match file.sa.as_ref().and_then(|sa| sa.cls.as_ref()) {
                Some(c) => c,
                None => return edges,
            };
            let lk = InheritLookup {
                file_path: &file.path,
                src_lang,
                lang_map,
                class_map,
                imported_files,
            };
            for cls in classes {
                collect_class_inherit_edges(&lk, cls, &mut edges);
            }
            edges
        })
        .collect();
    // Sort for deterministic output regardless of par_iter ordering. [H2 fix]
    all_edges.sort_unstable_by(|a, b| {
        a.child_file.cmp(&b.child_file)
            .then_with(|| a.child_class.cmp(&b.child_class))
            .then_with(|| a.parent_file.cmp(&b.parent_file))
            .then_with(|| a.parent_class.cmp(&b.parent_class))
    });
    all_edges
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests2;
