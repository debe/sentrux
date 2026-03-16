//! Blueprint layout engine — grid-based layout with DAG-ordered sections.
//!
//! Unlike treemap, blueprint uses fixed cell sizes and orders sections by
//! dependency level (DAG topological sort). Produces a grid-like arrangement
//! that emphasizes architectural layers over space-filling.

use crate::layout::blueprint_dag::compute_dag_order;
use crate::layout::squarify::{squarify, SquarifyConfig, WeightedItem};
use crate::layout::types::{Anchor, LayoutCtx, LayoutRectSlim, RectKind};
use crate::layout::weight::{WeightConfig, MAX_DEPTH};
use crate::layout::LayoutConfig;
use crate::core::settings::Settings;
use crate::core::snapshot::Snapshot;
use crate::core::types::FileNode;
use std::collections::HashMap;

/// Rectangle for blueprint layout — bundles (x, y, w, h) to reduce parameter counts.
#[derive(Clone, Copy)]
struct BpRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Compute padding and header size for a blueprint section at a given depth.
/// Deeper sections get progressively less chrome to avoid eating content area.
fn bp_chrome_for_depth(w: f64, h: f64, depth: u32, s: &Settings) -> (f64, f64) {
    let frac = (s.blueprint_max_chrome_frac - depth as f64 * 0.04).max(0.08);
    let pad = (s.blueprint_section_pad)
        .min(w * frac * 0.3)
        .min(h * frac * 0.15)
        .max(1.0);
    let header = if h > 24.0 && w > 20.0 {
        (s.blueprint_section_header).min(h * frac * 0.7)
    } else {
        0.0
    };
    (pad, header)
}

/// Compute the gutter (gap between sibling sections) at a given depth.
/// Top-level sections get the largest gutter; deeper sections get less.
fn gutter_for_depth(depth: u32, s: &Settings) -> f64 {
    if depth == 0 {
        s.blueprint_gutter_top
    } else {
        (s.blueprint_gutter_base - (depth as f64 - 1.0)).max(2.0)
    }
}

/// Compute blueprint layout with DAG ordering and own content area sizing.
pub fn layout_blueprint(
    root: &FileNode,
    snapshot: &Snapshot,
    cfg: &LayoutConfig<'_>,
) -> (
    Vec<LayoutRectSlim>,
    HashMap<String, Anchor>,
    f64,
    f64,
) {
    let settings = cfg.settings;
    let wc = WeightConfig {
        size_mode: cfg.size_mode,
        scale_mode: cfg.scale_mode,
        heat_map: cfg.heat_map,
        min_child_weight: settings.min_child_weight,
        focus_mode: cfg.focus_mode,
        entry_point_files: cfg.entry_point_files,
        hidden_paths: cfg.hidden_paths,
        impact_files: cfg.impact_files,
    };
    let mut weights: HashMap<String, f64> = HashMap::new();
    crate::layout::weight::precompute_weights(root, &wc, &mut weights);

    let mut rects = Vec::new();
    let mut anchors = HashMap::new();

    let dag_order = compute_optional_dag(root, snapshot);
    let (cw, ch) = compute_content_area(root, &weights);

    let mut lctx = LayoutCtx {
        weights: &weights,
        rects: &mut rects,
        anchors: &mut anchors,
        settings,
    };
    let root_rect = BpRect {
        x: settings.blueprint_route_margin,
        y: settings.blueprint_route_margin,
        w: cw,
        h: ch,
    };
    layout_dir(root, root_rect, 0, &mut lctx, dag_order.as_ref());

    let (max_x, max_y) = compute_bounds(&rects);
    (rects, anchors, max_x + settings.blueprint_route_margin, max_y + settings.blueprint_route_margin)
}

/// Compute DAG order from snapshot graphs if any edges exist.
fn compute_optional_dag(root: &FileNode, snapshot: &Snapshot) -> Option<HashMap<String, usize>> {
    let has_graph = !snapshot.import_graph.is_empty() || !snapshot.call_graph.is_empty();
    if has_graph { Some(compute_dag_order(root, snapshot)) } else { None }
}

/// Find the bounding box (max x, max y) of all layout rects.
fn compute_bounds(rects: &[LayoutRectSlim]) -> (f64, f64) {
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;
    for r in rects {
        max_x = max_x.max(r.x + r.w);
        max_y = max_y.max(r.y + r.h);
    }
    (max_x, max_y)
}

use crate::layout::weight::get_w;

// ─── DAG order lives in blueprint_dag.rs ───────────────────────

// ─── Content area sizing ───────────────────────────────────────

fn compute_content_area(root: &FileNode, weights: &HashMap<String, f64>) -> (f64, f64) {
    let (total_dirs, max_depth, flat_files) = collect_tree_stats(root, weights);
    size_from_stats(total_dirs, max_depth, &flat_files, weights)
}

/// Accumulator for tree stats collection — reduces `count` from 6 params to 3.
struct TreeStatsAcc {
    total_dirs: u32,
    max_depth: u32,
    flat_files: Vec<String>,
}

/// Walk the tree collecting visible dir count, max depth, and visible file paths.
fn collect_tree_stats(root: &FileNode, weights: &HashMap<String, f64>) -> (u32, u32, Vec<String>) {
    let mut acc = TreeStatsAcc { total_dirs: 0, max_depth: 0, flat_files: Vec::new() };

    fn count(n: &FileNode, d: u32, acc: &mut TreeStatsAcc, weights: &HashMap<String, f64>) -> bool {
        if !n.is_dir {
            let w = weights.get(&n.path).copied().unwrap_or(0.0);
            if w > 0.0 { acc.flat_files.push(n.path.clone()); }
            return w > 0.0;
        }
        if d > acc.max_depth { acc.max_depth = d; }
        let mut has_visible = false;
        if let Some(children) = &n.children {
            for c in children {
                if count(c, d + 1, acc, weights) { has_visible = true; }
            }
        }
        if has_visible { acc.total_dirs += 1; }
        has_visible
    }
    count(root, 0, &mut acc, weights);
    (acc.total_dirs, acc.max_depth, acc.flat_files)
}

/// Compute (width, height) content area from collected tree statistics.
fn size_from_stats(total_dirs: u32, max_depth: u32, flat_files: &[String], weights: &HashMap<String, f64>) -> (f64, f64) {
    let total_weight: f64 = flat_files.iter().map(|f| weights.get(f).copied().unwrap_or(0.0)).sum();
    let visible_files = flat_files.iter().filter(|f| weights.get(*f).copied().unwrap_or(0.0) > 0.0).count();
    let avg_weight = if visible_files > 0 { total_weight / visible_files as f64 } else { 1.0 };
    let side_per_file = (avg_weight.sqrt() * 40.0).max(30.0).min(200.0);

    let nesting_overhead = max_depth as f64 * 40.0;
    let file_area = (visible_files.max(1) as f64) * side_per_file * side_per_file;
    let dir_overhead = total_dirs as f64 * 50.0 * 50.0;
    let target_area = (400.0 * 400.0_f64).max(file_area + dir_overhead);
    let aspect = 1.4;
    let base_h = (target_area / aspect).sqrt();
    let ch = base_h.max(nesting_overhead * 3.0);
    let cw = ch * aspect;
    (cw.round(), ch.round())
}

// ─── Recursive nested layout ───────────────────────────────────

/// Partition children into visible dirs and files, filtering zero-weight nodes.
fn partition_children<'a>(
    children: &'a [FileNode],
    weights: &HashMap<String, f64>,
) -> (Vec<&'a FileNode>, Vec<&'a FileNode>) {
    let mut files: Vec<&FileNode> = Vec::new();
    let mut dirs: Vec<&FileNode> = Vec::new();
    for c in children {
        if c.is_dir {
            if get_w(c, weights) > 0.0 {
                dirs.push(c);
            }
        } else if get_w(c, weights) > 0.0 {
            files.push(c);
        }
    }
    (dirs, files)
}

/// Sort dirs (DAG at depth 0, alpha otherwise) and files (alpha),
/// then build the weighted children list: dirs first, then files.
fn sort_and_build_weighted<'a>(
    dirs: &mut Vec<&'a FileNode>,
    files: &mut Vec<&'a FileNode>,
    weights: &HashMap<String, f64>,
    dag_order: Option<&HashMap<String, usize>>,
    depth: u32,
) -> Vec<(&'a FileNode, f64)> {
    if let Some(dag) = dag_order {
        if depth == 0 && !dag.is_empty() {
            dirs.sort_by(|a, b| {
                let ao = dag.get(&a.path).copied().unwrap_or(0);
                let bo = dag.get(&b.path).copied().unwrap_or(0);
                ao.cmp(&bo).then_with(|| a.name.cmp(&b.name))
            });
        } else {
            dirs.sort_by(|a, b| a.name.cmp(&b.name));
        }
    } else {
        dirs.sort_by(|a, b| a.name.cmp(&b.name));
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));

    let mut wchildren: Vec<(&FileNode, f64)> = Vec::with_capacity(dirs.len() + files.len());
    for d in dirs.iter() {
        wchildren.push((*d, get_w(*d, weights)));
    }
    for f in files.iter() {
        wchildren.push((*f, get_w(*f, weights)));
    }
    wchildren
}

/// Emit a file rect and its anchor into the output vectors.
fn emit_file_rect(
    child: &FileNode,
    r: BpRect,
    depth: u32,
    parent_path: &str,
    lctx: &mut LayoutCtx<'_>,
) {
    lctx.rects.push(LayoutRectSlim {
        path: child.path.clone(),
        x: r.x, y: r.y, w: r.w, h: r.h,
        depth,
        kind: RectKind::File,
        section_id: parent_path.to_string(),
        grid_coord: None,
        header_h: 0.0,
    });
    if !child.path.is_empty() {
        lctx.anchors.insert(
            child.path.clone(),
            Anchor {
                file_path: child.path.clone(),
                cx: r.x + r.w / 2.0,
                cy: r.y + r.h / 2.0,
                section_id: parent_path.to_string(),
                bx: r.x, by: r.y, bw: r.w, bh: r.h,
            },
        );
    }
}

fn layout_dir(
    node: &FileNode,
    r: BpRect,
    depth: u32,
    lctx: &mut LayoutCtx<'_>,
    dag_order: Option<&HashMap<String, usize>>,
) {
    if depth >= MAX_DEPTH {
        return;
    }
    let settings = lctx.settings;
    if r.w < settings.blueprint_min_rect || r.h < settings.blueprint_min_rect {
        return;
    }
    let children = match &node.children {
        Some(c) if !c.is_empty() => c,
        _ => return,
    };

    let (pad, header) = bp_chrome_for_depth(r.w, r.h, depth, settings);
    let inner = BpRect {
        x: r.x + pad,
        y: r.y + header + pad,
        w: r.w - pad * 2.0,
        h: r.h - header - pad * 2.0,
    };
    if inner.w < settings.blueprint_min_rect || inner.h < settings.blueprint_min_rect {
        return;
    }

    let (mut dirs, mut files) = partition_children(children, lctx.weights);
    if files.is_empty() && dirs.is_empty() {
        return;
    }

    emit_section_rect(node, r, depth, header, lctx.rects);
    let wchildren = sort_and_build_weighted(&mut dirs, &mut files, lctx.weights, dag_order, depth);
    place_children(&wchildren, inner, depth, &node.path, lctx, dag_order);
}

/// Emit a section (directory) rect into the output.
fn emit_section_rect(
    node: &FileNode, r: BpRect, depth: u32, header: f64,
    rects: &mut Vec<LayoutRectSlim>,
) {
    rects.push(LayoutRectSlim {
        path: node.path.clone(),
        x: r.x, y: r.y, w: r.w, h: r.h, depth,
        kind: RectKind::Section,
        section_id: node.path.clone(),
        grid_coord: None,
        header_h: header,
    });
}

/// Squarify children and recursively lay out each placed child.
fn place_children(
    wchildren: &[(&FileNode, f64)],
    inner: BpRect,
    depth: u32, parent_path: &str,
    lctx: &mut LayoutCtx<'_>,
    dag_order: Option<&HashMap<String, usize>>,
) {
    let mut items: Vec<WeightedItem> = wchildren.iter().enumerate()
        .map(|(i, (_, w))| WeightedItem { weight: *w, index: i })
        .collect();
    items.sort_by(|a, b| b.weight.total_cmp(&a.weight));

    let gutter = gutter_for_depth(depth, lctx.settings);
    let mut placed: Vec<(usize, f64, f64, f64, f64)> = Vec::new();
    let sc = SquarifyConfig {
        x: inner.x, y: inner.y, w: inner.w, h: inner.h,
        gutter,
        min_rect: lctx.settings.squarify_min_rect,
    };
    squarify(&items, &sc, |idx, cx, cy, cw, ch| {
        placed.push((idx, cx, cy, cw, ch));
    });

    for (idx, cx, cy, cw, ch) in placed {
        let (child_node, _) = wchildren[idx];
        let cr = BpRect { x: cx, y: cy, w: cw, h: ch };
        if child_node.is_dir {
            layout_dir(child_node, cr, depth + 1, lctx, dag_order);
        } else {
            emit_file_rect(child_node, cr, depth + 1, parent_path, lctx);
        }
    }
}
