//! Metrics panel — always-visible left panel showing all analysis results.
//!
//! Displays the 11-dimension health report, architecture metrics,
//! evolution metrics (churn/bus factor/hotspots/coupling), test gaps,
//! rules check, and what-if simulation for the selected file.

use super::AppState;
use crate::license;
use super::activity_panel::draw_sep;
use super::health_display::draw_health_section;
use super::evolution_display::draw_evolution_section;
use super::rules_display::draw_rules_section;
use super::whatif_display::draw_whatif_section;
use super::ThemeConfig;

/// Draw the metrics panel (left side) showing all analysis results.
/// Always visible when a snapshot exists — no toggle.
pub fn draw_metrics_panel(ctx: &egui::Context, state: &mut AppState) {
    let tc = state.theme_config.clone();

    egui::SidePanel::left("metrics_panel")
        .default_width(200.0)
        .min_width(160.0)
        .max_width(280.0)
        .frame(
            egui::Frame::NONE
                .fill(tc.canvas_bg)
                .inner_margin(egui::Margin::same(4))
                .stroke(egui::Stroke::new(1.0, tc.section_border)),
        )
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("┌ Sentrux v{}", env!("CARGO_PKG_VERSION")))
                    .monospace()
                    .size(10.0)
                    .color(tc.section_label),
            );
            draw_sep(ui, &tc, 2.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    draw_metrics_sections(ui, state, &tc);
                });
        });
}

/// Render all metric sections inside the scroll area.
fn draw_metrics_sections(ui: &mut egui::Ui, state: &mut AppState, tc: &ThemeConfig) {
    let tier = license::current_tier();

    // Unified quality panel — ONE panel with 3 categories
    if let Some(report) = &state.health_report {
        draw_health_section(ui, report, tc);
        draw_sep(ui, tc, 4.0);
    }

    // Evolution: free shows grades only, pro shows full details
    if let Some(evo) = &state.evolution_report {
        if tier.is_pro() {
            draw_evolution_section(ui, evo, tc);
        } else {
            draw_evolution_summary(ui, evo, tc);
        }
        draw_sep(ui, tc, 4.0);
    }

    // Rules: always free (rule count limited in MCP, GUI shows all for local use)
    if let Some(rules) = &state.rule_check_result {
        draw_rules_section(ui, rules, tc);
        draw_sep(ui, tc, 4.0);
    }

    // What-if: only shown for pro users, hidden entirely for free
    if let (Some(sel), Some(snap)) = (&state.selected_path, &state.snapshot) {
        if tier.is_pro() {
            let sel_clone = sel.clone();
            let snap_clone = snap.clone();
            draw_whatif_section(ui, &sel_clone, &snap_clone, &mut state.whatif_cache, tc);
        }
    }

    if state.health_report.is_none() && state.arch_report.is_none() {
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new("  (scan a project)")
                .monospace()
                .size(10.0)
                .color(tc.text_secondary),
        );
    }
}

/// Draw evolution summary for free tier — scores only, no file-level details.
fn draw_evolution_summary(ui: &mut egui::Ui, report: &crate::metrics::evo::EvolutionReport, tc: &ThemeConfig) {
    use super::ui_helpers::score_color;
    let font = egui::FontId::monospace(9.0);
    let row_h = 13.0;

    ui.label(
        egui::RichText::new("GIT STATS")
            .monospace()
            .size(9.0)
            .color(tc.section_label),
    );
    ui.add_space(2.0);

    let metrics: Vec<(&str, String)> = vec![
        ("churn", format!("{} files", report.churn.len())),
        ("bus factor", format!("{} solo", (report.single_author_ratio * report.churn.len() as f64).round() as u32)),
        ("commits", format!("{}", report.commits_analyzed)),
    ];
    for (label, value) in &metrics {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        let cy = rect.center().y;
        ui.painter().text(egui::pos2(rect.left() + 4.0, cy), egui::Align2::LEFT_CENTER, label, font.clone(), tc.text_secondary);
        ui.painter().text(egui::pos2(rect.right() - 4.0, cy), egui::Align2::RIGHT_CENTER, value, font.clone(), tc.text_secondary);
    }

}
