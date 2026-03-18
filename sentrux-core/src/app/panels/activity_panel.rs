//! Activity panel — live file change feed and top connections.
//!
//! Right-side panel showing recent watcher events and top-connected files.
//! Health and architecture grades moved to the always-visible metrics panel (left).

use super::{ThemeConfig, AppState};
use egui::{CursorIcon, Sense};
use std::collections::HashMap;

/// Draw a horizontal separator line with configurable top spacing.
pub(crate) fn draw_sep(ui: &mut egui::Ui, tc: &ThemeConfig, top: f32) {
    ui.add_space(top);
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [egui::pos2(r.left(), r.top()), egui::pos2(r.right(), r.top())],
        egui::Stroke::new(1.0, tc.section_border),
    );
    ui.add_space(3.0);
}


/// Draw the top connections section. Returns true if a file was clicked.
fn draw_top_connections(
    ui: &mut egui::Ui,
    top: &[(String, usize)],
    state: &mut AppState,
    tc: &ThemeConfig,
) -> bool {
    if top.is_empty() {
        return false;
    }
    ui.label(
        egui::RichText::new("TOP CONNECTIONS")
            .monospace()
            .size(9.0)
            .color(tc.section_label),
    );
    ui.add_space(2.0);
    let top_clicked_path = draw_connection_rows(ui, top, state, tc);
    let clicked = apply_click_selection(state, top_clicked_path);
    draw_sep(ui, tc, 4.0);
    ui.label(
        egui::RichText::new("ACTIVITY")
            .monospace()
            .size(9.0)
            .color(tc.section_label),
    );
    ui.add_space(2.0);
    clicked
}

/// Render each connection row and return the clicked path if any.
fn draw_connection_rows(
    ui: &mut egui::Ui,
    top: &[(String, usize)],
    state: &AppState,
    tc: &ThemeConfig,
) -> Option<String> {
    let mut top_clicked_path: Option<String> = None;
    for (path, count) in top {
        let filename = path.rsplit('/').next().unwrap_or(path);
        let is_selected = state.selected_path.as_deref() == Some(path.as_str());
        let (row_rect, row_resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 14.0),
            Sense::click(),
        );
        let hovered = row_resp.hovered();
        if hovered || is_selected {
            let bg = if is_selected { tc.file_surface_spotlit } else { tc.file_surface };
            ui.painter().rect_filled(row_rect, egui::CornerRadius::ZERO, bg);
        }
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if row_resp.clicked() {
            top_clicked_path = Some(path.clone());
        }
        row_resp.on_hover_text(egui::RichText::new(path.as_str()).monospace().size(10.0));
        let left = row_rect.left() + 4.0;
        let cy = row_rect.center().y;
        ui.painter().text(
            egui::pos2(left, cy),
            egui::Align2::LEFT_CENTER,
            format!("{:>3}", count),
            egui::FontId::monospace(9.0),
            tc.text_secondary,
        );
        let name_color = if is_selected { tc.selected_stroke } else { tc.file_label };
        ui.painter().text(
            egui::pos2(left + 28.0, cy),
            egui::Align2::LEFT_CENTER,
            filename,
            egui::FontId::monospace(9.0),
            name_color,
        );
    }
    top_clicked_path
}

/// Apply click selection toggle logic. Returns true if selection changed.
fn apply_click_selection(state: &mut AppState, clicked_path: Option<String>) -> bool {
    if let Some(path) = clicked_path {
        if state.selected_path.as_deref() == Some(path.as_str()) {
            state.selected_path = None;
        } else {
            state.selected_path = Some(path);
        }
        true
    } else {
        false
    }
}

/// Shared drawing resources for activity rows — avoids passing 3 extra params per row.
struct ActivityDrawCtx {
    now: std::time::Instant,
    font10: egui::FontId,
    font9: egui::FontId,
}

/// Draw the scrollable activity log. Returns true if a file was clicked.
fn draw_activity_scroll(ui: &mut egui::Ui, state: &mut AppState, tc: &ThemeConfig) -> bool {
    let mut clicked = false;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        let adctx = ActivityDrawCtx {
            now: std::time::Instant::now(),
            font10: egui::FontId::monospace(10.0),
            font9: egui::FontId::monospace(9.0),
        };
        let mut clicked_path: Option<String> = None;
        for i in 0..state.recent_activity.len() {
            if let Some(p) = draw_activity_row(ui, state, tc, i, &adctx) {
                clicked_path = Some(p);
            }
        }
        clicked = apply_click_selection(state, clicked_path);
    });
    clicked
}

/// Format age in seconds to a human-readable short string.
fn format_age(age_secs: u64) -> String {
    if age_secs < 2 { "now".into() }
    else if age_secs < 60 { format!("{}s", age_secs) }
    else if age_secs < 3600 { format!("{}m", age_secs / 60) }
    else { format!("{}h", age_secs / 3600) }
}

/// Map activity kind string to its display character and color.
fn kind_indicator(kind: &str, tc: &ThemeConfig, fallback: egui::Color32) -> (&'static str, egui::Color32) {
    match kind {
        "create" => ("+", tc.diff_added),
        "remove" => ("-", tc.diff_removed),
        "modify" => ("~", tc.diff_modified),
        _ => ("?", fallback),
    }
}

/// Build delta display parts (lines changed, functions changed).
fn build_delta_parts(lines_delta: i32, funcs_delta: i32, tc: &ThemeConfig) -> Vec<(String, egui::Color32)> {
    let mut parts = Vec::new();
    if lines_delta > 0 {
        parts.push((format!("+{}", lines_delta), tc.diff_added));
    } else if lines_delta < 0 {
        parts.push((format!("{}", lines_delta), tc.diff_removed));
    }
    if funcs_delta > 0 {
        parts.push((format!("+{} functions", funcs_delta), tc.diff_added));
    } else if funcs_delta < 0 {
        parts.push((format!("{}  functions", funcs_delta), tc.diff_removed));
    }
    parts
}

/// Draw delta labels right-aligned before the age string.
fn draw_delta_labels(
    ui: &mut egui::Ui,
    parts: &[(String, egui::Color32)],
    right_edge: f32,
    cy: f32,
    age_width: f32,
    font: &egui::FontId,
) {
    let mut dx = right_edge - 4.0 - age_width;
    for (text, color) in parts.iter().rev() {
        let galley = ui.painter().layout_no_wrap(text.clone(), font.clone(), *color);
        let w = galley.size().x + 4.0;
        dx -= w;
        ui.painter().galley(egui::pos2(dx, cy - galley.size().y / 2.0), galley, *color);
    }
}

/// Render a single activity row. Returns clicked path if this row was clicked.
fn draw_activity_row(
    ui: &mut egui::Ui,
    state: &AppState,
    tc: &ThemeConfig,
    idx: usize,
    adctx: &ActivityDrawCtx,
) -> Option<String> {
    let entry = &state.recent_activity[idx];
    let age_str = format_age(adctx.now.duration_since(entry.time).as_secs());
    let (kind_char, kind_color) = kind_indicator(&entry.kind, tc, tc.text_secondary);
    let filename = entry.path.rsplit('/').next().unwrap_or(&entry.path);
    let is_selected = state.selected_path.as_deref() == Some(&entry.path);
    let rh = 16.0;
    let (rr, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), rh), Sense::click());
    if resp.hovered() || is_selected {
        let bg = if is_selected { tc.file_surface_spotlit } else { tc.file_surface };
        ui.painter().rect_filled(rr, egui::CornerRadius::ZERO, bg);
    }
    if is_selected {
        let bar = egui::Rect::from_min_size(rr.left_top(), egui::vec2(2.0, rh));
        ui.painter().rect_filled(bar, egui::CornerRadius::ZERO, tc.selected_stroke);
    }
    if resp.hovered() { ui.ctx().set_cursor_icon(CursorIcon::PointingHand); }
    let result = if resp.clicked() { Some(entry.path.clone()) } else { None };
    resp.on_hover_text(egui::RichText::new(&entry.path).monospace().size(10.0));
    let left = rr.left() + 4.0;
    let cy = rr.center().y;
    ui.painter().text(egui::pos2(left, cy), egui::Align2::LEFT_CENTER, kind_char, adctx.font10.clone(), kind_color);
    let nc = if is_selected { tc.selected_stroke } else { tc.file_label };
    ui.painter().text(egui::pos2(left + 14.0, cy), egui::Align2::LEFT_CENTER, filename, adctx.font10.clone(), nc);

    let delta_parts = build_delta_parts(entry.lines_delta, entry.funcs_delta, tc);
    draw_delta_labels(ui, &delta_parts, rr.right(), cy, 24.0, &adctx.font9);

    ui.painter().text(egui::pos2(rr.right() - 4.0, cy), egui::Align2::RIGHT_CENTER, &age_str, adctx.font9.clone(), tc.text_secondary);
    result
}

/// Draw the activity panel (right side) showing recent file events.
/// Terminal pixel style: monospace, sharp corners, cool blue-gray palette,
/// no rounded elements, discrete spacing. [ref: TERMINAL_PIXEL_STYLE_GUIDE.md]
/// Returns true if a file was clicked (selected_path changed).
/// Rebuild the top connections cache if stale (mutates state), then return
/// nothing — callers should borrow from `state.top_connections_cache` directly
/// to avoid cloning every frame.
fn ensure_top_connections_cache(state: &mut AppState) {
    let ver = state.rendered_version;
    let filter_key = state.edge_filter as u8;
    let needs_rebuild = state.top_connections_cache.as_ref().is_none_or(|(v, f, _)| *v != ver || *f != filter_key);
    if needs_rebuild {
        let computed = top_connected_files(state, 10);
        state.top_connections_cache = Some((ver, filter_key, computed));
    }
}

/// Draw the update availability indicator at the top of the panel.
fn draw_update_indicator(ui: &mut egui::Ui, tc: &ThemeConfig) {
    if let Some(latest) = crate::app::update_check::available_update() {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("  v{} available", latest))
                    .monospace().size(9.0)
                    .color(tc.diff_added),
            ).on_hover_text("brew upgrade sentrux");
        });
        draw_sep(ui, tc, 2.0);
        ui.ctx().request_repaint();
    }
}

/// Draw the file detail header with deselect button, and the detail content.
fn draw_file_detail_section(ui: &mut egui::Ui, state: &mut AppState, tc: &ThemeConfig) {
    if state.selected_path.is_none() { return; }
    draw_sep(ui, tc, 4.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("┌ FILE DETAIL")
            .monospace().size(10.0).color(tc.section_label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let deselect = ui.add(
                egui::Button::new(egui::RichText::new("×")
                    .monospace().size(11.0).color(tc.text_secondary))
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE));
            if deselect.clicked() { state.selected_path = None; }
            deselect.on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text("Deselect file");
        });
    });
    draw_sep(ui, tc, 2.0);
    if let Some(snap) = &state.snapshot {
        let snap_clone = snap.clone();
        super::file_detail::draw_file_detail(ui, state, &snap_clone, tc);
    }
}

pub fn draw_activity_panel(ctx: &egui::Context, state: &mut AppState) -> bool {
    let mut clicked = false;
    let tc = state.theme_config.clone();

    egui::SidePanel::right("activity_panel")
        .default_width(200.0)
        .min_width(140.0)
        .max_width(320.0)
        .frame(egui::Frame::NONE
            .fill(tc.canvas_bg)
            .inner_margin(egui::Margin::same(4))
            .stroke(egui::Stroke::new(1.0, tc.section_border)))
        .show(ctx, |ui| {
            draw_update_indicator(ui, &tc);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(snap) = &state.snapshot {
                        super::language_summary::draw_language_summary(
                            ui, snap, &state.lang_stats, &tc);
                        draw_sep(ui, &tc, 4.0);
                    }

                    ensure_top_connections_cache(state);
                    let top_cache = state.top_connections_cache.take();
                    let top: &[(String, usize)] = match &top_cache {
                        Some((_, _, v)) => v.as_slice(),
                        None => &[],
                    };
                    if draw_top_connections(ui, top, state, &tc) { clicked = true; }
                    state.top_connections_cache = top_cache;

                    if !state.recent_activity.is_empty() {
                        if draw_activity_scroll(ui, state, &tc) { clicked = true; }
                    }

                    draw_file_detail_section(ui, state, &tc);
                });
        });

    clicked
}


/// Compute top N most-connected files from render_data edge paths.
/// Respects the active edge_filter so counts match what's visible on canvas.
/// Previously counted from raw snapshot graphs, ignoring edge_filter and
/// focus_mode — showing files not visible on canvas. [ref:4e8f1175]
pub fn top_connected_files(state: &AppState, n: usize) -> Vec<(String, usize)> {
    let rd = match &state.render_data {
        Some(rd) => rd,
        None => return Vec::new(),
    };
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for ep in &rd.edge_paths {
        if !state.edge_filter.accepts(&ep.edge_type) {
            continue;
        }
        *counts.entry(ep.from_file.as_str()).or_default() += 1;
        *counts.entry(ep.to_file.as_str()).or_default() += 1;
    }
    let mut sorted: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    sorted.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    sorted.truncate(n);
    sorted
}
