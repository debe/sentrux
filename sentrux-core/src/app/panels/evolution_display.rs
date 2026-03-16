//! Evolution metrics display — churn, bus factor, hotspots, change coupling.
//!
//! Renders the EvolutionReport data in the metrics panel.
//! Uses continuous [0,1] scores with smooth color gradient.

use crate::metrics::evo::EvolutionReport;
use super::ThemeConfig;
use super::ui_helpers::score_color;

/// Draw the evolution section in the metrics panel.
pub(crate) fn draw_evolution_section(ui: &mut egui::Ui, report: &EvolutionReport, tc: &ThemeConfig) {
    let row_h = 13.0;
    let font = egui::FontId::monospace(9.0);

    // Section header
    ui.label(
        egui::RichText::new("GIT STATS")
            .monospace()
            .size(9.0)
            .color(tc.section_label),
    );
    ui.add_space(2.0);

    // Raw data rows — no score, just facts from git history
    let commits_tooltip = format!("Total commits analyzed (last {} days)", report.lookback_days);
    let metrics: Vec<(&str, String, f64, &str)> = vec![
        ("churn", format!("{} files", report.churn.len()), -1.0,
         "Files with changes in lookback window"),
        ("bus factor", format!("{} solo", (report.single_author_ratio * report.churn.len() as f64).round() as u32), -1.0,
         "Files with only one author — bus factor risk"),
        ("commits", format!("{}", report.commits_analyzed), -1.0,
         commits_tooltip.as_str()),
    ];

    for (label, value, score, tooltip) in &metrics {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        let cy = rect.center().y;
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, cy),
            egui::Align2::LEFT_CENTER,
            label,
            font.clone(),
            tc.text_secondary,
        );
        if *score >= 0.0 {
            let c = score_color(*score);
            ui.painter().text(
                egui::pos2(rect.right() - 4.0, cy),
                egui::Align2::RIGHT_CENTER,
                format!("{}", (score * 10000.0).round() as u32),
                font.clone(),
                c,
            );
            ui.painter().text(
                egui::pos2(rect.right() - 36.0, cy),
                egui::Align2::RIGHT_CENTER,
                value,
                font.clone(),
                tc.text_secondary,
            );
        } else {
            ui.painter().text(
                egui::pos2(rect.right() - 4.0, cy),
                egui::Align2::RIGHT_CENTER,
                value,
                font.clone(),
                tc.text_secondary,
            );
        }
        if resp.hovered() {
            resp.on_hover_text(egui::RichText::new(*tooltip).monospace().size(9.0));
        }
    }

    // Temporal hotspots
    draw_hotspots(ui, report, tc, row_h);

    // Change coupling pairs
    draw_coupling(ui, report, tc, row_h);

    // Bus factor details (risky single-author files)
    draw_bus_factor(ui, report, tc, row_h);
}

fn draw_hotspots(ui: &mut egui::Ui, report: &EvolutionReport, _tc: &ThemeConfig, row_h: f32) {
    if report.hotspots.is_empty() { return; }
    let color = egui::Color32::from_rgb(200, 140, 80);
    ui.add_space(3.0);
    ui.label(egui::RichText::new("HOTSPOTS (churn x complexity)").monospace().size(8.0).color(color));
    for hs in report.hotspots.iter().take(5) {
        let name = hs.file.rsplit('/').next().unwrap_or(&hs.file);
        let text = format!("  {} ({}x{}={})", name, hs.churn_count, hs.max_complexity, hs.risk_score);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        if resp.hovered() {
            resp.on_hover_text(egui::RichText::new(&hs.file).monospace().size(10.0));
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER, &text,
            egui::FontId::monospace(8.0), color);
    }
    if report.hotspots.len() > 5 {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("  +{} more", report.hotspots.len() - 5),
            egui::FontId::monospace(8.0), egui::Color32::from_rgb(140, 140, 140));
    }
}

fn draw_coupling(ui: &mut egui::Ui, report: &EvolutionReport, _tc: &ThemeConfig, row_h: f32) {
    if report.coupling_pairs.is_empty() { return; }
    let color = egui::Color32::from_rgb(140, 180, 200);
    ui.add_space(3.0);
    ui.label(egui::RichText::new("CHANGE COUPLING (co-change)").monospace().size(8.0).color(color));
    for pair in report.coupling_pairs.iter().take(5) {
        let a = pair.file_a.rsplit('/').next().unwrap_or(&pair.file_a);
        let b = pair.file_b.rsplit('/').next().unwrap_or(&pair.file_b);
        let text = format!("  {} <> {} ({} J:{})", a, b, pair.co_change_count, (pair.coupling_strength * 10000.0).round() as u32);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        if resp.hovered() {
            resp.on_hover_text(
                egui::RichText::new(format!("{} <> {}", pair.file_a, pair.file_b)).monospace().size(10.0)
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER, &text,
            egui::FontId::monospace(8.0), color);
    }
    if report.coupling_pairs.len() > 5 {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("  +{} more pairs", report.coupling_pairs.len() - 5),
            egui::FontId::monospace(8.0), egui::Color32::from_rgb(140, 140, 140));
    }
}

fn draw_bus_factor(ui: &mut egui::Ui, report: &EvolutionReport, _tc: &ThemeConfig, row_h: f32) {
    let mut single_author_files: Vec<(&str, &str)> = report.authors.iter()
        .filter(|(_, info)| info.author_count == 1)
        .map(|(path, info)| (path.as_str(), info.primary_author.as_str()))
        .collect();
    single_author_files.sort_unstable_by_key(|(path, _)| *path);
    single_author_files.truncate(5);

    if single_author_files.is_empty() { return; }

    let color = egui::Color32::from_rgb(200, 160, 200);
    ui.add_space(3.0);
    ui.label(egui::RichText::new("BUS FACTOR RISK (single author)").monospace().size(8.0).color(color));
    for (path, author) in single_author_files.iter().take(5) {
        let name = path.rsplit('/').next().unwrap_or(path);
        let text = format!("  {} ({})", name, author);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        if resp.hovered() {
            resp.on_hover_text(egui::RichText::new(*path).monospace().size(10.0));
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER, &text,
            egui::FontId::monospace(8.0), color);
    }
}
