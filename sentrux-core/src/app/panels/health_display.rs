//! Unified quality display — 6 root cause metrics + quality signal.
//!
//! No letter grades. No arbitrary categories. Just 6 fundamental
//! structural properties with continuous [0,1] scores and smooth
//! red→yellow→green color gradient.

use crate::metrics::HealthReport;
use crate::core::settings::ThemeConfig;
use super::ui_helpers::score_color;

pub(crate) fn draw_health_section(ui: &mut egui::Ui, report: &HealthReport, tc: &ThemeConfig) {
    let row_h = 13.0;
    let font = egui::FontId::monospace(9.0);

    // ── Quality Signal bar ──
    draw_quality_signal(ui, report, tc);
    ui.add_space(4.0);

    // ── 6 Root Cause Metrics ──
    let raw = &report.root_cause_raw;
    let scores = &report.root_cause_scores;

    draw_root_cause_row(ui, "modularity",  format!("Q={:.2}", raw.modularity_q),          scores.modularity,  tc, row_h, &font);
    draw_root_cause_row(ui, "acyclicity",  format!("{} cycles", raw.cycle_count),          scores.acyclicity,  tc, row_h, &font);
    draw_root_cause_row(ui, "depth",       format!("{} max", raw.max_depth),               scores.depth,       tc, row_h, &font);
    draw_root_cause_row(ui, "equality",    format!("G={:.2}", raw.complexity_gini),        scores.equality,    tc, row_h, &font);
    draw_root_cause_row(ui, "redundancy",  format!("{}", (raw.redundancy_ratio * 10000.0).round() as u32), scores.redundancy, tc, row_h, &font);

    // ── Summary ──
    ui.add_space(2.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, rect.center().y), egui::Align2::LEFT_CENTER,
        format!("edges {}/{}  files {}", report.cross_module_edges, report.total_import_edges,
            report.all_file_lines.len()),
        font.clone(), tc.text_secondary,
    );

    // ── Flagged items ──
    draw_cycles(ui, report, tc, row_h);
    draw_flagged_files(ui, report, row_h);
    draw_unstable(ui, report, row_h);
}

/// Draw the quality signal bar at the top.
fn draw_quality_signal(ui: &mut egui::Ui, report: &HealthReport, tc: &ThemeConfig) {
    ui.label(
        egui::RichText::new("CODE QUALITY")
            .monospace().size(9.0).color(tc.section_label),
    );
    ui.add_space(2.0);

    let signal = report.quality_signal;
    let color = score_color(signal);

    let (grade_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(grade_rect.left() + 4.0, grade_rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("Quality  {}", (signal * 10000.0).round() as u32),
        egui::FontId::monospace(11.0), color,
    );

    // Pixel block progress bar: ████████████░░░░░░
    let total_blocks = 20;
    let filled = (signal * total_blocks as f64).round() as usize;
    let bar_str: String = (0..total_blocks).map(|i| if i < filled { '█' } else { '░' }).collect();
    let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 12.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(bar_rect.left() + 4.0, bar_rect.center().y),
        egui::Align2::LEFT_CENTER,
        &bar_str,
        egui::FontId::monospace(9.0), color,
    );
}

/// Draw a single root cause metric row: label, raw value, score%.
fn draw_root_cause_row(
    ui: &mut egui::Ui,
    label: &str,
    raw_value: String,
    score: f64,
    tc: &ThemeConfig,
    row_h: f32,
    font: &egui::FontId,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
    let cy = rect.center().y;

    // Label
    ui.painter().text(
        egui::pos2(rect.left() + 8.0, cy), egui::Align2::LEFT_CENTER,
        label, font.clone(), tc.text_secondary,
    );

    // Score as integer 0-10000 — every point is one point
    let color = score_color(score);
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
        format!("{}", (score * 10000.0).round() as u32), font.clone(), color,
    );

    // Raw value
    if !raw_value.is_empty() {
        ui.painter().text(
            egui::pos2(rect.right() - 36.0, cy), egui::Align2::RIGHT_CENTER,
            &raw_value, font.clone(), tc.text_secondary,
        );
    }
}

fn draw_cycles(ui: &mut egui::Ui, report: &HealthReport, tc: &ThemeConfig, row_h: f32) {
    if report.circular_dep_files.is_empty() { return; }
    ui.add_space(3.0);
    let warn_color = egui::Color32::from_rgb(200, 80, 80);
    ui.label(egui::RichText::new("CYCLES").monospace().size(8.0).color(warn_color));
    for (i, cycle) in report.circular_dep_files.iter().take(2).enumerate() {
        let files_str: Vec<&str> = cycle.iter().take(3).map(|s| {
            s.rsplit('/').next().unwrap_or(s)
        }).collect();
        let suffix = if cycle.len() > 3 { format!(" +{}", cycle.len() - 3) } else { String::new() };
        let text = format!("  {}. {}{}", i + 1, files_str.join(" <> "), suffix);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER, &text,
            egui::FontId::monospace(8.0), warn_color);
    }
    if report.circular_dep_files.len() > 2 {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("  +{} more", report.circular_dep_files.len() - 2),
            egui::FontId::monospace(8.0), tc.text_secondary);
    }
}

fn draw_flagged_files(ui: &mut egui::Ui, report: &HealthReport, row_h: f32) {
    let warn_color = egui::Color32::from_rgb(200, 170, 80);
    draw_flagged_list(ui, "GOD FILES (fan-out)", &report.god_files, warn_color, row_h);
    draw_flagged_list(ui, "HOTSPOTS (fan-in)", &report.hotspot_files, warn_color, row_h);
}

fn draw_flagged_list(ui: &mut egui::Ui, title: &str, items: &[crate::metrics::FileMetric],
                     color: egui::Color32, row_h: f32) {
    if items.is_empty() { return; }
    ui.add_space(3.0);
    ui.label(egui::RichText::new(title).monospace().size(8.0).color(color));
    for item in items.iter().take(2) {
        let name = item.path.rsplit('/').next().unwrap_or(&item.path);
        let text = format!("  {} ({})", name, item.value);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        if resp.hovered() {
            resp.on_hover_text(egui::RichText::new(&item.path).monospace().size(10.0));
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER, &text,
            egui::FontId::monospace(8.0), color);
    }
}

fn draw_unstable(ui: &mut egui::Ui, report: &HealthReport, row_h: f32) {
    let unstable: Vec<_> = report.most_unstable.iter()
        .filter(|m| m.instability > 0.8).take(2).collect();
    if unstable.is_empty() { return; }
    let color = egui::Color32::from_rgb(180, 140, 200);
    ui.add_space(3.0);
    ui.label(egui::RichText::new("UNSTABLE (I>0.8)").monospace().size(8.0).color(color));
    for m in &unstable {
        let name = m.path.rsplit('/').next().unwrap_or(&m.path);
        let text = format!("  {} I:{:.2} out:{} in:{}", name, m.instability, m.fan_out, m.fan_in);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        if resp.hovered() {
            resp.on_hover_text(egui::RichText::new(&m.path).monospace().size(10.0));
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER, &text,
            egui::FontId::monospace(8.0), color);
    }
}
