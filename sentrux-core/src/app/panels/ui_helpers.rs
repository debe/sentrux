//! Shared UI drawing helpers to eliminate duplication across display panels.
//!
//! Provides core primitives:
//! - `score_color`: continuous color from [0,1] score (delegates to color_utils)
//! - `draw_flagged_list`: titled item list with hover tooltips and "+N more" overflow

// Re-export color computation functions from color_utils
pub(crate) use crate::app::color_utils::lang_profile_color;
pub(crate) use crate::app::color_utils::score_color;
pub(crate) use crate::app::color_utils::score_color_for_theme;
pub(crate) use crate::app::color_utils::score_color_themed;

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_flagged_list<T, F, H>(
    ui: &mut egui::Ui,
    title: &str,
    items: &[T],
    color: egui::Color32,
    row_h: f32,
    max_items: usize,
    format_fn: F,
    hover_fn: H,
    tc: &crate::core::settings::ThemeConfig,
) where
    F: Fn(&T) -> String,
    H: Fn(&T) -> String,
{
    if items.is_empty() {
        return;
    }
    ui.add_space(3.0);
    ui.label(egui::RichText::new(title).monospace().size(8.0).color(color));
    for item in items.iter().take(max_items) {
        let text = format_fn(item);
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        if resp.hovered() {
            resp.on_hover_text(egui::RichText::new(hover_fn(item)).monospace().size(10.0));
        }
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &text,
            egui::FontId::monospace(8.0),
            color,
        );
    }
    let remaining = items.len().saturating_sub(max_items);
    if remaining > 0 {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("  +{} more", remaining),
            egui::FontId::monospace(8.0),
            tc.text_muted,
        );
    }
}
