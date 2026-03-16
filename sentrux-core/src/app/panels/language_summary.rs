//! Language & plugin summary — shows per-language breakdown after scan.
//!
//! Displays which language plugins were loaded, which were used in the scan,
//! and per-language statistics: files, functions, lines, import edges.

use crate::analysis::lang_registry;
use crate::core::snapshot::Snapshot;
use crate::core::settings::ThemeConfig;
use std::collections::HashMap;
use std::sync::Arc;

/// Per-language statistics aggregated from the snapshot.
pub(crate) struct LangStat {
    pub files: u32,
    pub lines: u32,
    pub funcs: u32,
    pub import_edges: u32,
}

/// Aggregate per-language stats from snapshot. Called once per scan, cached.
pub(crate) fn compute_lang_stats(snap: &Snapshot) -> Vec<(String, LangStat)> {
    let files = crate::core::snapshot::flatten_files_ref(&snap.root);
    let mut stats: HashMap<String, LangStat> = HashMap::new();

    for file in &files {
        if file.lang.is_empty() || file.lang == "unknown" {
            continue;
        }
        let stat = stats.entry(file.lang.clone()).or_insert(LangStat {
            files: 0, lines: 0, funcs: 0, import_edges: 0,
        });
        stat.files += 1;
        stat.lines += file.lines;
        stat.funcs += file.funcs;
    }

    // Count import edges per source language
    for edge in &snap.import_graph {
        let ext = edge.from_file.rsplit('.').next().unwrap_or("");
        let lang = lang_registry::detect_lang_from_ext(ext);
        if let Some(stat) = stats.get_mut(&lang) {
            stat.import_edges += 1;
        }
    }

    let mut sorted: Vec<(String, LangStat)> = stats.into_iter().collect();
    sorted.sort_by(|a, b| b.1.files.cmp(&a.1.files).then_with(|| a.0.cmp(&b.0)));
    sorted
}

/// Draw the language & plugin summary section.
/// `lang_stats` should be pre-computed and cached (call `compute_lang_stats` once per scan).
pub(crate) fn draw_language_summary(
    ui: &mut egui::Ui,
    snap: &Arc<Snapshot>,
    lang_stats: &[(String, LangStat)],
    tc: &ThemeConfig,
) {
    let row_h = 13.0;
    let font = egui::FontId::monospace(9.0);
    let font_small = egui::FontId::monospace(8.0);

    // Header
    ui.label(
        egui::RichText::new("LANGUAGES")
            .monospace().size(9.0).color(tc.section_label),
    );
    ui.add_space(2.0);

    // Plugin count
    let total_plugins = lang_registry::plugin_count();
    let langs_used = lang_stats.len();

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_h), egui::Sense::hover(),
    );
    let cy = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, cy), egui::Align2::LEFT_CENTER,
        "plugins loaded", font.clone(), tc.text_secondary,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
        format!("{}", total_plugins), font.clone(), tc.text_primary,
    );

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_h), egui::Sense::hover(),
    );
    let cy = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, cy), egui::Align2::LEFT_CENTER,
        "languages in project", font.clone(), tc.text_secondary,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
        format!("{}", langs_used), font.clone(), tc.text_primary,
    );

    // Graph edge totals
    let total_imports = snap.import_graph.len();
    let total_calls = snap.call_graph.len();
    let total_inherit = snap.inherit_graph.len();

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_h), egui::Sense::hover(),
    );
    let cy = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, cy), egui::Align2::LEFT_CENTER,
        "import edges", font.clone(), tc.text_secondary,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
        format!("{}", total_imports), font.clone(), tc.text_primary,
    );

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_h), egui::Sense::hover(),
    );
    let cy = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, cy), egui::Align2::LEFT_CENTER,
        "call edges", font.clone(), tc.text_secondary,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
        format!("{}", total_calls), font.clone(), tc.text_primary,
    );

    if total_inherit > 0 {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_h), egui::Sense::hover(),
        );
        let cy = rect.center().y;
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, cy), egui::Align2::LEFT_CENTER,
            "inherit edges", font.clone(), tc.text_secondary,
        );
        ui.painter().text(
            egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
            format!("{}", total_inherit), font.clone(), tc.text_primary,
        );
    }

    ui.add_space(4.0);

    // Per-language breakdown
    for (lang, stat) in lang_stats {
        let profile = lang_registry::profile(lang);
        let color = egui::Color32::from_rgb(
            profile.color_rgb[0], profile.color_rgb[1], profile.color_rgb[2],
        );

        // Language name row with color dot
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_h + 1.0), egui::Sense::hover(),
        );
        let cy = rect.center().y;

        // Color square
        ui.painter().rect_filled(
            egui::Rect::from_center_size(egui::pos2(rect.left() + 7.0, cy), egui::vec2(5.0, 5.0)),
            0.0, color,
        );

        // Language name
        ui.painter().text(
            egui::pos2(rect.left() + 14.0, cy), egui::Align2::LEFT_CENTER,
            lang, font.clone(), tc.text_primary,
        );

        // File count
        ui.painter().text(
            egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
            format!("{} files", stat.files), font_small.clone(), tc.text_secondary,
        );

        // Detail row: funcs, lines, edges
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_h - 1.0), egui::Sense::hover(),
        );
        let cy = rect.center().y;
        let detail = if stat.import_edges > 0 {
            format!("{} functions  {} lines  {} imports", stat.funcs, stat.lines, stat.import_edges)
        } else {
            format!("{} functions  {} lines", stat.funcs, stat.lines)
        };
        ui.painter().text(
            egui::pos2(rect.left() + 14.0, cy), egui::Align2::LEFT_CENTER,
            detail, font_small.clone(),
            egui::Color32::from_rgb(90, 90, 100),
        );
    }

    // Show failed plugins if any
    let failed = lang_registry::failed_plugins();
    if !failed.is_empty() {
        ui.add_space(4.0);
        ui.painter().text(
            egui::pos2(4.0, ui.cursor().top()), egui::Align2::LEFT_TOP,
            format!("{} plugin(s) failed", failed.len()),
            font_small.clone(), egui::Color32::from_rgb(180, 80, 60),
        );
    }
}
