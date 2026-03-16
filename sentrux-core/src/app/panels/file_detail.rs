//! File detail panel — shows metrics, imports, importers, functions for the selected file.
//!
//! Replaces the generic ACTIVITY panel when a file is selected.
//! Shows everything a developer needs to understand a single file's role.

use crate::app::state::AppState;
use crate::core::settings::ThemeConfig;
use crate::core::snapshot::Snapshot;
use std::sync::Arc;

/// Draw file detail section for the selected file.
pub(crate) fn draw_file_detail(
    ui: &mut egui::Ui,
    state: &AppState,
    snap: &Arc<Snapshot>,
    tc: &ThemeConfig,
) {
    let selected = match &state.selected_path {
        Some(p) => p,
        None => {
            ui.label(
                egui::RichText::new("Click a file to see details")
                    .monospace().size(9.0).color(tc.text_secondary),
            );
            return;
        }
    };

    let font = egui::FontId::monospace(9.0);
    let font_small = egui::FontId::monospace(8.0);
    let row_h = 13.0;

    // File name + path
    let filename = selected.rsplit('/').next().unwrap_or(selected);
    ui.label(
        egui::RichText::new(filename)
            .monospace().size(11.0).color(tc.text_primary).strong(),
    );
    ui.label(
        egui::RichText::new(selected.as_str())
            .monospace().size(8.0).color(tc.text_secondary),
    );

    // Language + line count from file_index
    if let Some(entry) = state.file_index.get(selected.as_str()) {
        let lang_text = format!("{} \u{00b7} {} lines \u{00b7} {} functions",
            entry.lang, entry.lines, entry.funcs);
        let profile = crate::analysis::lang_registry::profile(&entry.lang);
        let color = egui::Color32::from_rgb(
            profile.color_rgb[0], profile.color_rgb[1], profile.color_rgb[2],
        );
        ui.horizontal(|ui| {
            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 10.0), egui::Sense::hover());
            ui.painter().circle_filled(dot_rect.center(), 3.0, color);
            ui.label(egui::RichText::new(lang_text).monospace().size(8.0).color(tc.text_secondary));
        });
    }

    ui.add_space(6.0);

    let is_pro = crate::license::current_tier().is_pro();
    if !is_pro {
        return;
    }

    draw_metrics_section(ui, state, snap, selected, tc, row_h, &font);

    let imports: Vec<&str> = snap.import_graph.iter()
        .filter(|e| e.from_file == *selected)
        .map(|e| e.to_file.as_str())
        .collect();
    draw_edge_list(ui, &imports, "IMPORTS", "\u{2192}", tc, row_h, &font_small);

    let importers: Vec<&str> = snap.import_graph.iter()
        .filter(|e| e.to_file == *selected)
        .map(|e| e.from_file.as_str())
        .collect();
    draw_edge_list(ui, &importers, "IMPORTED BY", "\u{2190}", tc, row_h, &font_small);

    draw_functions_section(ui, state, snap, selected, tc, row_h, &font_small);
}

fn draw_metrics_section(
    ui: &mut egui::Ui,
    state: &AppState,
    snap: &Arc<Snapshot>,
    selected: &str,
    tc: &ThemeConfig,
    row_h: f32,
    font: &egui::FontId,
) {
    if state.file_index.get(selected).is_none() {
        return;
    }
    draw_section_header(ui, "METRICS", tc);

    let fan_out = snap.import_graph.iter()
        .filter(|e| e.from_file == selected).count();
    let fan_in = snap.import_graph.iter()
        .filter(|e| e.to_file == selected).count();

    draw_metric_row(ui, "fan-out", &format!("{} imports", fan_out), tc, row_h, font);
    draw_metric_row(ui, "fan-in", &format!("{} importers", fan_in), tc, row_h, font);

    if let Some(arch) = &state.arch_report {
        if let Some(&blast) = arch.blast_radius.get(selected) {
            draw_metric_row(ui, "blast radius", &format!("{} files", blast), tc, row_h, font);
        }
    }
    ui.add_space(6.0);
}

fn draw_edge_list(
    ui: &mut egui::Ui,
    edges: &[&str],
    title: &str,
    arrow: &str,
    tc: &ThemeConfig,
    row_h: f32,
    font_small: &egui::FontId,
) {
    if edges.is_empty() {
        return;
    }
    draw_section_header(ui, &format!("{} ({})", title, edges.len()), tc);
    for edge in edges.iter().take(20) {
        let short = edge.rsplit('/').next().unwrap_or(edge);
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_h), egui::Sense::click(),
        );
        if response.hovered() {
            ui.painter().rect_filled(rect, 2.0, tc.section_border);
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{} {}", arrow, short),
            font_small.clone(), tc.text_secondary,
        );
        if response.hovered() {
            response.on_hover_text(*edge);
        }
    }
    if edges.len() > 20 {
        ui.label(egui::RichText::new(format!("  +{} more", edges.len() - 20))
            .monospace().size(8.0).color(tc.text_secondary));
    }
    ui.add_space(6.0);
}

fn draw_functions_section(
    ui: &mut egui::Ui,
    state: &AppState,
    snap: &Arc<Snapshot>,
    selected: &str,
    tc: &ThemeConfig,
    row_h: f32,
    font_small: &egui::FontId,
) {
    if state.file_index.get(selected).is_none() {
        return;
    }
    let files = crate::core::snapshot::flatten_files_ref(&snap.root);
    let file_node = match files.iter().find(|f| f.path == selected) {
        Some(f) => f,
        None => return,
    };
    let funcs = match file_node.sa.as_ref().and_then(|sa| sa.functions.as_ref()) {
        Some(f) => f,
        None => return,
    };
    let mut sorted_funcs: Vec<_> = funcs.iter().collect();
    sorted_funcs.sort_by(|a, b| b.cc.unwrap_or(0).cmp(&a.cc.unwrap_or(0)));
    if sorted_funcs.is_empty() {
        return;
    }

    draw_section_header(ui, &format!("FUNCTIONS ({})", sorted_funcs.len()), tc);
    for f in sorted_funcs.iter().take(15) {
        let cc = f.cc.unwrap_or(0);
        let cc_color = if cc > 15 {
            egui::Color32::from_rgb(203, 75, 22)
        } else {
            tc.text_secondary
        };
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_h), egui::Sense::hover(),
        );
        let cy = rect.center().y;
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, cy),
            egui::Align2::LEFT_CENTER,
            &f.n, font_small.clone(), tc.text_secondary,
        );
        ui.painter().text(
            egui::pos2(rect.right() - 4.0, cy),
            egui::Align2::RIGHT_CENTER,
            format!("CC={}", cc), font_small.clone(), cc_color,
        );
    }
}

fn draw_section_header(ui: &mut egui::Ui, text: &str, tc: &ThemeConfig) {
    ui.label(
        egui::RichText::new(text)
            .monospace().size(9.0).color(tc.section_label),
    );
    ui.add_space(2.0);
}

fn draw_metric_row(ui: &mut egui::Ui, label: &str, value: &str, tc: &ThemeConfig, row_h: f32, font: &egui::FontId) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_h), egui::Sense::hover(),
    );
    let cy = rect.center().y;
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, cy),
        egui::Align2::LEFT_CENTER,
        label, font.clone(), tc.text_secondary,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, cy),
        egui::Align2::RIGHT_CENTER,
        value, font.clone(), tc.text_primary,
    );
}
