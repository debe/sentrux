//! Breadcrumb navigation for drill-down into directory subtrees.
//!
//! Renders a clickable path chain ("Root / src / layout") that lets the user
//! navigate back to any ancestor level. Returns whether the drill stack changed
//! so the caller can trigger a re-layout.

use super::state::AppState;

/// Render the breadcrumb buttons, returning the clicked action index.
fn render_breadcrumb_buttons(ui: &mut egui::Ui, drill_stack: &[String]) -> Option<usize> {
    let mut clicked_action: Option<usize> = None;
    ui.horizontal(|ui| {
        if ui.small_button("Root").clicked() {
            clicked_action = Some(usize::MAX);
        }
        for i in 0..drill_stack.len() {
            ui.label("/");
            let name = drill_stack[i].rsplit('/').next().unwrap_or(&drill_stack[i]);
            if ui.small_button(name).clicked() {
                clicked_action = Some(i);
            }
        }
    });
    clicked_action
}

/// Apply a breadcrumb click action to the drill stack. Returns true if changed.
fn apply_breadcrumb_action(drill_stack: &mut Vec<String>, action: Option<usize>) -> bool {
    match action {
        Some(usize::MAX) => { drill_stack.clear(); true }
        Some(i) if i + 1 < drill_stack.len() => { drill_stack.truncate(i + 1); true }
        _ => false,
    }
}

/// Draw drill-down breadcrumb navigation. Returns true if drill_stack changed.
/// Always shows root path; shows drill stack when drilled into subdirectories.
pub fn draw_breadcrumb(ui: &mut egui::Ui, state: &mut AppState) -> bool {
    if state.drill_stack.is_empty() {
        // Show root path as non-clickable breadcrumb
        if let Some(root) = &state.root_path {
            let name = root.rsplit('/').next().unwrap_or(root);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(name).monospace().size(10.0).weak());
                let tc = crate::core::settings::ThemeConfig::from_theme(state.theme);
                ui.label(egui::RichText::new("(double-click a directory to drill in)").monospace().size(8.0).color(
                    tc.text_muted
                ));
            });
        }
        return false;
    }
    let action = render_breadcrumb_buttons(ui, &state.drill_stack);
    apply_breadcrumb_action(&mut state.drill_stack, action)
}
