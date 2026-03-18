//! What-if simulation display — shows impact of removing/moving the selected file.
//!
//! Computes on-demand when a file is selected. The simulation is O(E) so it's
//! fast enough for the UI thread (sub-millisecond for typical projects).

use super::{Snapshot, ThemeConfig};
use crate::metrics::whatif::{self, WhatIfAction, WhatIfResult};
use std::sync::Arc;

/// Cached what-if result to avoid recomputing every frame.
/// Keyed by (selected_path, snapshot edge count + file count) to invalidate on change.
/// Previously used Arc pointer identity which could give false cache hits if the
/// allocator reuses the same address after an Arc is dropped.
pub(crate) struct WhatIfCache {
    path: String,
    snap_fingerprint: u64,
    remove_result: WhatIfResult,
}

/// Ensure the what-if cache is up-to-date for the selected path and snapshot.
fn ensure_cache(
    selected_path: &str,
    snapshot: &Arc<Snapshot>,
    cache: &mut Option<WhatIfCache>,
) {
    // Use edge count + entry point count + total files + total lines as fingerprint
    // to detect snapshot changes. More robust than Arc pointer identity (allocator reuse)
    // and catches incremental rescans that change edge targets without changing counts. [H8 fix]
    let snap_fingerprint = snapshot.import_graph.len() as u64 * 1_000_003
        + snapshot.entry_points.len() as u64
        + snapshot.total_files as u64 * 7
        + snapshot.total_lines as u64;
    let needs_compute = cache.as_ref().is_none_or(|c| {
        c.path != selected_path || c.snap_fingerprint != snap_fingerprint
    });
    if needs_compute {
        let action = WhatIfAction::RemoveFile { path: selected_path.to_string() };
        let result = whatif::simulate(&snapshot.import_graph, &snapshot.entry_points, &action);
        *cache = Some(WhatIfCache {
            path: selected_path.to_string(),
            snap_fingerprint,
            remove_result: result,
        });
    }
}

/// Draw the file name row with hover tooltip.
fn draw_file_name_row(ui: &mut egui::Ui, selected_path: &str, row_h: f32, font: &egui::FontId, tc: &ThemeConfig) {
    let name = selected_path.rsplit('/').next().unwrap_or(selected_path);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("Remove \"{}\":", name),
        font.clone(),
        tc.text_primary,
    );
    if resp.hovered() {
        resp.on_hover_text(egui::RichText::new(selected_path).monospace().size(10.0));
    }
}

/// Draw the verdict row (improved / no improvement).
fn draw_verdict_row(ui: &mut egui::Ui, r: &WhatIfResult, row_h: f32, font: &egui::FontId, tc: &ThemeConfig) {
    let (verdict, verdict_color) = if r.improved {
        ("improves architecture", tc.status_success)
    } else {
        ("no improvement", tc.status_warning)
    };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("  -> {}", verdict),
        font.clone(),
        verdict_color,
    );
}

/// Draw before/after comparison rows for changed metrics.
fn draw_comparison_rows(ui: &mut egui::Ui, r: &WhatIfResult, row_h: f32, tc: &ThemeConfig) {
    let comparisons: Vec<(&str, String, String)> = vec![
        ("score", format!("{}", (r.score_before * 10000.0).round() as u32), format!("{}", (r.score_after * 10000.0).round() as u32)),
        ("violations", format!("{}", r.upward_violations_before), format!("{}", r.upward_violations_after)),
        ("max blast", format!("{}", r.max_blast_before), format!("{}", r.max_blast_after)),
        ("max level", format!("{}", r.max_level_before), format!("{}", r.max_level_after)),
    ];
    for (label, before, after) in &comparisons {
        if before == after { continue; }
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("  {}: {} -> {}", label, before, after),
            egui::FontId::monospace(8.0),
            tc.text_secondary,
        );
    }
}

/// Draw level change rows (top 3 files affected).
fn draw_level_changes(ui: &mut egui::Ui, r: &WhatIfResult, row_h: f32, tc: &ThemeConfig) {
    if r.level_changes.is_empty() { return; }
    let color = tc.accent_coupling;
    for lc in r.level_changes.iter().take(3) {
        let name = lc.file.rsplit('/').next().unwrap_or(&lc.file);
        let arrow = if lc.level_after < lc.level_before { "↓" } else { "↑" };
        let text = format!("  {} L{} {} L{}", name, lc.level_before, arrow, lc.level_after);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        if resp.hovered() {
            resp.on_hover_text(egui::RichText::new(&lc.file).monospace().size(10.0));
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER, &text,
            egui::FontId::monospace(8.0), color);
    }
}

/// Draw what-if section for the currently selected file.
/// Returns true if the cache was updated (for repaint).
pub(crate) fn draw_whatif_section(
    ui: &mut egui::Ui,
    selected_path: &str,
    snapshot: &Arc<Snapshot>,
    cache: &mut Option<WhatIfCache>,
    tc: &ThemeConfig,
) {
    let row_h = 13.0;
    let font = egui::FontId::monospace(9.0);

    ui.label(
        egui::RichText::new("WHAT-IF (selected file)")
            .monospace()
            .size(9.0)
            .color(tc.section_label),
    );
    ui.add_space(2.0);

    ensure_cache(selected_path, snapshot, cache);

    let r = &cache.as_ref().unwrap().remove_result;
    draw_file_name_row(ui, selected_path, row_h, &font, tc);
    draw_verdict_row(ui, r, row_h, &font, tc);
    draw_comparison_rows(ui, r, row_h, tc);
    draw_level_changes(ui, r, row_h, tc);
}
