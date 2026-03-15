//! Health report display — renders the 15-dimension A-F grade card.
//!
//! Draws the composite grade plus per-dimension rows (cycles, coupling,
//! complexity, cohesion, entropy, etc.) with color-coded letter grades.
//! Tooltip on each row explains the source metric and its thresholds.

use crate::metrics::HealthReport;
use crate::core::settings::ThemeConfig;
use super::ui_helpers::dim_grade_color;

pub(crate) fn draw_health_section(ui: &mut egui::Ui, report: &HealthReport, tc: &ThemeConfig) {
    let row_h = 13.0;
    let font = egui::FontId::monospace(9.0);

    // Section header + overall grade
    draw_health_header(ui, report, tc);

    // Dimension metric rows with grades
    let metrics = build_health_metrics(report);
    draw_health_metric_rows(ui, &metrics, tc, row_h, &font);

    // Cross-module info row (no grade)
    draw_health_cross_mod_row(ui, report, tc, row_h, &font);

    draw_health_cycles(ui, report, tc, row_h);
    draw_health_flagged_files(ui, report, tc, row_h);
    draw_health_unstable(ui, report, tc, row_h);
}

/// Draw the CODE HEALTH section header and overall grade.
fn draw_health_header(ui: &mut egui::Ui, report: &HealthReport, tc: &ThemeConfig) {
    ui.label(
        egui::RichText::new("CODE HEALTH")
            .monospace()
            .size(9.0)
            .color(tc.section_label),
    );
    ui.add_space(2.0);

    let grade_color = dim_grade_color(report.grade, tc);
    let (grade_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(grade_rect.left() + 4.0, grade_rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("Grade: {}", report.grade),
        egui::FontId::monospace(11.0),
        grade_color,
    );
    ui.add_space(2.0);
}

/// Build the list of health metric tuples: (label, value, grade, tooltip).
fn build_health_metrics(report: &HealthReport) -> Vec<(&'static str, String, char, &'static str)> {
    let d = &report.dimensions;
    vec![
        ("cycles", format!("{}", report.circular_dep_count), d.cycles,
         "Martin 2003 ADP | 0=A | circular dependency count"),
        ("complex functions", format!("{} ({:.0}%)", report.complex_functions.len(), report.complex_fn_ratio * 100.0), d.complex_fn,
         "McCabe 1976 | lower=better | functions with CC>15"),
        ("coupling", format!("{:.0}%", report.coupling_score * 100.0), d.coupling,
         "Constantine & Yourdon | lower=better | cross-module/total edges"),
        ("entropy", format!("{:.2}", report.entropy), d.entropy,
         "Shannon 1948 | lower=better | edge distribution disorder"),
        ("cohesion", match report.avg_cohesion {
            Some(c) => format!("{:.0}%", c * 100.0),
            None => "n/a".to_string(),
        }, d.cohesion.unwrap_or('-'),
         "Constantine & Yourdon | higher=better | intra-module connectivity"),
        ("depth", format!("{}", report.max_depth), d.depth,
         "Lakos 1996 | lower=better | longest dependency chain"),
        ("god files", format!("{} ({:.0}%)", report.god_files.len(), report.god_file_ratio * 100.0), d.god_files,
         "Martin | lower=better | files with fan-out >15"),
        ("hotspots", format!("{} ({:.0}%)", report.hotspot_files.len(), report.hotspot_ratio * 100.0), d.hotspots,
         "Martin | lower=better | files with fan-in >20"),
        ("long functions", format!("{} ({:.0}%)", report.long_functions.len(), report.long_fn_ratio * 100.0), d.long_fn,
         "Industry | lower=better | functions >50 lines"),
        ("comments", match report.comment_ratio {
            Some(r) => format!("{:.0}%", r * 100.0),
            None => "n/a".to_string(),
        }, d.comment.unwrap_or('-'),
         "Language-aware | Rust/Go 5-10%, Java/C++ 15-25%"),
        ("large files", format!("{} ({:.0}%)", report.large_file_count, report.large_file_ratio * 100.0), d.file_size,
         "Industry | lower=better | files >500 lines"),
        ("cognitive complexity", format!("{} ({:.0}%)", report.cog_complex_functions.len(), report.cog_complex_ratio * 100.0), d.cog_complex,
         "SonarSource 2016 | lower=better | functions with cognitive complexity >15"),
        ("duplication", format!("{} groups ({:.0}%)", report.duplicate_groups.len(), report.duplication_ratio * 100.0), d.duplication,
         "SonarSource | lower=better | duplicate function bodies"),
        ("dead code", format!("{} ({:.0}%)", report.dead_functions.len(), report.dead_code_ratio * 100.0), d.dead_code,
         "Lower=better | unreferenced functions"),
        ("high parameters", format!("{} ({:.0}%)", report.high_param_functions.len(), report.high_param_ratio * 100.0), d.high_params,
         "Industry | lower=better | functions with >4 parameters"),
    ]
}

/// Draw each health metric row with label, value, grade letter, and tooltip.
fn draw_health_metric_rows(
    ui: &mut egui::Ui,
    metrics: &[(&str, String, char, &str)],
    tc: &ThemeConfig,
    row_h: f32,
    font: &egui::FontId,
) {
    for (label, value, grade, tooltip) in metrics {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        let cy = rect.center().y;
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, cy), egui::Align2::LEFT_CENTER,
            label, font.clone(), tc.text_secondary,
        );
        let g_color = dim_grade_color(*grade, tc);
        ui.painter().text(
            egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER,
            format!("{}", grade), font.clone(), g_color,
        );
        ui.painter().text(
            egui::pos2(rect.right() - 24.0, cy), egui::Align2::RIGHT_CENTER,
            value, font.clone(), tc.text_secondary,
        );
        if resp.hovered() {
            resp.on_hover_text(egui::RichText::new(*tooltip).monospace().size(9.0));
        }
    }
}

/// Draw the cross-module edges info row.
fn draw_health_cross_mod_row(ui: &mut egui::Ui, report: &HealthReport, tc: &ThemeConfig, row_h: f32, font: &egui::FontId) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, rect.center().y), egui::Align2::LEFT_CENTER,
        "cross-module", font.clone(), tc.text_secondary,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 4.0, rect.center().y), egui::Align2::RIGHT_CENTER,
        format!("{}/{}", report.cross_module_edges, report.total_import_edges),
        font.clone(), tc.text_secondary,
    );
}

fn draw_health_cycles(ui: &mut egui::Ui, report: &HealthReport, tc: &ThemeConfig, row_h: f32) {
    if report.circular_dep_files.is_empty() { return; }
    ui.add_space(3.0);
    ui.label(egui::RichText::new("CYCLES").monospace().size(8.0)
        .color(egui::Color32::from_rgb(200, 80, 80)));
    let warn_color = egui::Color32::from_rgb(200, 100, 100);
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

fn draw_health_flagged_files(ui: &mut egui::Ui, report: &HealthReport, _tc: &ThemeConfig, row_h: f32) {
    let warn_color = egui::Color32::from_rgb(200, 170, 80);
    draw_flagged_list(ui, "GOD FILES (fan-out)", &report.god_files, warn_color, row_h);
    draw_flagged_list(ui, "HOTSPOTS (fan-in)", &report.hotspot_files, warn_color, row_h);
}

fn draw_health_unstable(ui: &mut egui::Ui, report: &HealthReport, _tc: &ThemeConfig, row_h: f32) {
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
