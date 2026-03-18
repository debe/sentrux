//! Language & plugin summary — shows per-language breakdown after scan.
//!
//! Displays which language plugins were loaded, which were used in the scan,
//! and per-language statistics: files, functions, lines, import edges.

use crate::analysis::lang_registry;
use super::{Snapshot, ThemeConfig};
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

/// Draw a single label-value row at standard height.
fn draw_stat_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    font: &egui::FontId,
    tc: &ThemeConfig,
) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 13.0), egui::Sense::hover(),
    );
    let cy = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, cy), egui::Align2::LEFT_CENTER,
        label, font.clone(), tc.text_secondary,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
        value, font.clone(), tc.text_primary,
    );
}

/// Draw summary rows: plugin count, language count, edge totals.
fn draw_summary_rows(
    ui: &mut egui::Ui,
    snap: &Arc<Snapshot>,
    lang_count: usize,
    font: &egui::FontId,
    tc: &ThemeConfig,
) {
    draw_stat_row(ui, "plugins loaded", &format!("{}", lang_registry::plugin_count()), font, tc);
    draw_stat_row(ui, "languages in project", &format!("{}", lang_count), font, tc);
    draw_stat_row(ui, "import edges", &format!("{}", snap.import_graph.len()), font, tc);
    draw_stat_row(ui, "call edges", &format!("{}", snap.call_graph.len()), font, tc);
    if !snap.inherit_graph.is_empty() {
        draw_stat_row(ui, "inherit edges", &format!("{}", snap.inherit_graph.len()), font, tc);
    }
}

/// Draw a single language row: color dot, name, file count, detail line.
fn draw_lang_row(
    ui: &mut egui::Ui,
    lang: &str,
    stat: &LangStat,
    font: &egui::FontId,
    font_small: &egui::FontId,
    tc: &ThemeConfig,
) {
    let profile = lang_registry::profile(lang);
    let color = super::ui_helpers::lang_profile_color(&profile);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 14.0), egui::Sense::hover(),
    );
    let cy = rect.center().y;
    ui.painter().circle_filled(egui::pos2(rect.left() + 7.0, cy), 3.0, color);
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, cy), egui::Align2::LEFT_CENTER,
        lang, font.clone(), tc.text_primary,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
        format!("{} files", stat.files), font_small.clone(), tc.text_secondary,
    );

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 12.0), egui::Sense::hover(),
    );
    let cy = rect.center().y;
    let detail = if stat.import_edges > 0 {
        format!("{} functions  {} lines  {} imports", stat.funcs, stat.lines, stat.import_edges)
    } else {
        format!("{} functions  {} lines", stat.funcs, stat.lines)
    };
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, cy), egui::Align2::LEFT_CENTER,
        detail, font_small.clone(), tc.text_secondary.linear_multiply(0.7),
    );
}

/// Draw the language & plugin summary section.
/// `lang_stats` should be pre-computed and cached (call `compute_lang_stats` once per scan).
pub(crate) fn draw_language_summary(
    ui: &mut egui::Ui,
    snap: &Arc<Snapshot>,
    lang_stats: &[(String, LangStat)],
    tc: &ThemeConfig,
) {
    let font = egui::FontId::monospace(9.0);
    let font_small = egui::FontId::monospace(8.0);

    ui.label(egui::RichText::new("LANGUAGES").monospace().size(9.0).color(tc.section_label));
    ui.add_space(2.0);

    draw_summary_rows(ui, snap, lang_stats.len(), &font, tc);
    ui.add_space(4.0);

    for (lang, stat) in lang_stats {
        draw_lang_row(ui, lang, stat, &font, &font_small, tc);
    }

    let failed = lang_registry::failed_plugins();
    if !failed.is_empty() {
        ui.add_space(4.0);
        ui.painter().text(
            egui::pos2(4.0, ui.cursor().top()), egui::Align2::LEFT_TOP,
            format!("{} plugin(s) failed", failed.len()),
            font_small, tc.accent_plugin_error,
        );
    }
}
