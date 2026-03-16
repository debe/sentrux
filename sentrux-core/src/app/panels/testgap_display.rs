//! Test gap display — shows coverage ratio, grade, and riskiest untested files.
//!
//! Renders the test-gap analysis results in the metrics panel. Displays
//! overall coverage ratio, letter grade, and a ranked list of the riskiest
//! untested production files (sorted by lines of code descending).
//! Key function: `draw_testgap_section` paints the test-gap UI into an egui frame.

use crate::metrics::testgap::TestGapReport;
use crate::core::settings::ThemeConfig;
use super::ui_helpers::score_color;

/// Draw the test gap section in the metrics panel.
pub(crate) fn draw_testgap_section(ui: &mut egui::Ui, report: &TestGapReport, tc: &ThemeConfig) {
    let row_h = 13.0;
    let font = egui::FontId::monospace(9.0);

    // Section header
    ui.label(
        egui::RichText::new("TEST COVERAGE")
            .monospace()
            .size(9.0)
            .color(tc.section_label),
    );
    ui.add_space(2.0);

    // Overall score
    let sc = score_color(report.coverage_score);
    let (grade_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(grade_rect.left() + 4.0, grade_rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("Coverage: {}", (report.coverage_ratio * 10000.0).round() as u32),
        egui::FontId::monospace(11.0),
        sc,
    );
    ui.add_space(2.0);

    // Summary stats
    let stats: Vec<(&str, String, &str)> = vec![
        ("source files", format!("{}", report.source_files),
         "Non-test source files detected"),
        ("test files", format!("{}", report.test_files),
         "Test files detected by naming convention"),
        ("tested", format!("{}", report.tested_source_files),
         "Source files imported by at least one test"),
        ("untested", format!("{}", report.untested_source_files),
         "Source files with zero test coverage"),
    ];

    for (label, value, tooltip) in &stats {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        let cy = rect.center().y;
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, cy),
            egui::Align2::LEFT_CENTER,
            label,
            font.clone(),
            tc.text_secondary,
        );
        ui.painter().text(
            egui::pos2(rect.right() - 4.0, cy),
            egui::Align2::RIGHT_CENTER,
            value,
            font.clone(),
            tc.text_secondary,
        );
        if resp.hovered() {
            resp.on_hover_text(egui::RichText::new(*tooltip).monospace().size(9.0));
        }
    }

    // Riskiest untested files
    draw_gaps(ui, report, tc, row_h);
}

fn draw_gaps(ui: &mut egui::Ui, report: &TestGapReport, _tc: &ThemeConfig, row_h: f32) {
    if report.gaps.is_empty() { return; }
    let color = egui::Color32::from_rgb(200, 100, 100);
    ui.add_space(3.0);
    ui.label(egui::RichText::new("RISKIEST UNTESTED").monospace().size(8.0).color(color));
    for gap in report.gaps.iter().take(2) {
        let name = gap.file.rsplit('/').next().unwrap_or(&gap.file);
        let text = format!("  {} cc:{} fi:{} r:{}", name, gap.max_complexity, gap.fan_in, gap.risk_score);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        if resp.hovered() {
            resp.on_hover_text(
                egui::RichText::new(format!("{} [{}]\nComplexity: {}, Fan-in: {}, Risk: {}",
                    gap.file, gap.lang, gap.max_complexity, gap.fan_in, gap.risk_score
                )).monospace().size(9.0)
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER, &text,
            egui::FontId::monospace(8.0), color);
    }
    if report.gaps.len() > 2 {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("  +{} more untested", report.gaps.len() - 2),
            egui::FontId::monospace(8.0), egui::Color32::from_rgb(140, 140, 140));
    }
}
