//! Rules check display — shows pass/fail status and violations.
//!
//! Renders architectural rule check results in the metrics panel using egui.
//! Each rule is shown with a pass/fail indicator and severity-colored text.
//! Violations list the offending file pairs with the rule they break.
//! Key function: `draw_rules_section` paints the rules UI into an egui frame.

use crate::metrics::rules::checks::{RuleCheckResult, Severity};
use super::ThemeConfig;

/// Draw the rules check section in the metrics panel.
pub(crate) fn draw_rules_section(ui: &mut egui::Ui, result: &RuleCheckResult, tc: &ThemeConfig) {
    let row_h = 13.0;
    let font = egui::FontId::monospace(9.0);

    // Section header
    ui.label(
        egui::RichText::new("RULES")
            .monospace()
            .size(9.0)
            .color(tc.section_label),
    );
    ui.add_space(2.0);

    // Pass/fail status
    let (status_text, status_color) = if result.passed {
        ("PASS", tc.status_success)
    } else {
        ("FAIL", tc.status_error)
    };

    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("{} ({} rules checked)", status_text, result.rules_checked),
        egui::FontId::monospace(11.0),
        status_color,
    );
    ui.add_space(2.0);

    // Violations
    if result.violations.is_empty() { return; }

    let error_count = result.violations.iter().filter(|v| v.severity == Severity::Error).count();
    let warn_count = result.violations.iter().filter(|v| v.severity == Severity::Warning).count();

    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("{} errors, {} warnings", error_count, warn_count),
        font.clone(),
        tc.text_secondary,
    );

    let error_color = tc.status_error;
    let warn_color = tc.status_warning;

    draw_violation_rows(ui, result, row_h, error_color, warn_color, tc);
}

/// Draw up to 8 violation rows, with overflow indicator.
fn draw_violation_rows(
    ui: &mut egui::Ui,
    result: &RuleCheckResult,
    row_h: f32,
    error_color: egui::Color32,
    warn_color: egui::Color32,
    tc: &ThemeConfig,
) {
    for violation in result.violations.iter().take(8) {
        let (color, prefix) = match violation.severity {
            Severity::Error => (error_color, "E"),
            Severity::Warning => (warn_color, "W"),
        };
        let text = format!("  {} {}: {}", prefix, violation.rule, truncate(&violation.message, 40));
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        if resp.hovered() {
            resp.on_hover_text(
                egui::RichText::new(&violation.message).monospace().size(9.0)
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER, &text,
            egui::FontId::monospace(8.0), color);
    }
    if result.violations.len() > 8 {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("  +{} more violations", result.violations.len() - 8),
            egui::FontId::monospace(8.0), tc.text_muted);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let boundary = s.floor_char_boundary(max);
        format!("{}...", &s[..boundary])
    }
}
