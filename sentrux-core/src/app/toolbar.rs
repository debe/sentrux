//! Toolbar UI — mode selectors, filter controls, and scan progress display.
//!
//! Returns `(layout_changed, visual_changed)` so the caller knows whether
//! to trigger a re-layout or just a repaint.

use crate::layout::types::{ColorMode, EdgeFilter, FocusMode, LayoutMode, ScaleMode, SizeMode};
use crate::core::settings::Theme;
use crate::license;
use super::state::AppState;

/// Draw the toolbar panel. Returns (layout_changed, visual_changed).
/// layout_changed = size/scale/layout mode changed (needs re-layout).
/// visual_changed = color/theme/edge/focus changed (needs repaint only).
pub fn draw_toolbar(ui: &mut egui::Ui, state: &mut AppState) -> (bool, bool) {
    let mut layout_changed = false;
    let mut visual_changed = false;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        draw_open_folder(ui, state);

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(2.0);

        draw_structure_group(ui, state, &mut layout_changed);

        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);

        draw_visual_group(ui, state, &mut visual_changed);

        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);

        // Search box
        let search_response = ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .desired_width(120.0)
                .hint_text("Search files...")
                .font(egui::FontId::monospace(9.0))
        );
        if search_response.changed() {
            visual_changed = true;
        }

        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);

        draw_filter_group(ui, state, &mut layout_changed, &mut visual_changed);

        draw_scan_progress(ui, state);
    });

    (layout_changed, visual_changed)
}

/// Primary actions: Open Folder + Rescan.
fn draw_open_folder(ui: &mut egui::Ui, state: &mut AppState) {
    if ui.button("Open Folder").on_hover_text("Open a project folder to scan").clicked() {
        state.folder_picker_requested = true;
    }
    // Rescan button — manual trigger for re-scanning current project
    if state.root_path.is_some() && !state.scanning {
        if ui.button("\u{21bb}").on_hover_text("Rescan (Cmd+R)").clicked() {
            state.rescan_requested = true;
        }
    }
}

/// Structure group: Layout mode, Size mode, Scale mode combo boxes.
fn draw_structure_group(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    draw_layout_mode_combo(ui, state, layout_changed);
    draw_size_mode_combo(ui, state, layout_changed);
    draw_scale_mode_combo(ui, state, layout_changed);
}

/// Layout mode combo box (Treemap / Blueprint).
fn draw_layout_mode_combo(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    ui.label(egui::RichText::new("Layout").small().weak());
    let layout_label = match state.layout_mode {
        LayoutMode::Treemap => "Treemap",
        LayoutMode::Blueprint => "Blueprint",
    };
    egui::ComboBox::from_id_salt("layout_mode")
        .selected_text(layout_label)
        .width(80.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(&mut state.layout_mode, LayoutMode::Treemap, "Treemap")
                .changed()
            {
                *layout_changed = true;
            }
            if ui
                .selectable_value(&mut state.layout_mode, LayoutMode::Blueprint, "Blueprint")
                .changed()
            {
                *layout_changed = true;
            }
        });
}

/// Map SizeMode to its display label.
fn size_mode_label(mode: SizeMode) -> &'static str {
    match mode {
        SizeMode::Lines => "Lines",
        SizeMode::Logic => "Logic",
        SizeMode::Funcs => "Funcs",
        SizeMode::Comments => "Comments",
        SizeMode::Blanks => "Blanks",
        SizeMode::Heat => "Heat",
        SizeMode::Uniform => "Uniform",
    }
}

/// All SizeMode variants in display order.
const SIZE_MODES: &[SizeMode] = &[
    SizeMode::Lines, SizeMode::Logic, SizeMode::Funcs,
    SizeMode::Comments, SizeMode::Blanks, SizeMode::Heat,
    SizeMode::Uniform,
];

/// Size mode combo box (Lines / Logic / Funcs / ...).
fn draw_size_mode_combo(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    ui.label(egui::RichText::new("size:").small().weak());
    egui::ComboBox::from_id_salt("size_mode")
        .selected_text(size_mode_label(state.size_mode))
        .width(70.0)
        .show_ui(ui, |ui| {
            for &mode in SIZE_MODES {
                if ui.selectable_value(&mut state.size_mode, mode, size_mode_label(mode)).changed() {
                    *layout_changed = true;
                }
            }
        });
}

/// Scale mode combo box (Linear / Sqrt / Log / Smooth).
fn draw_scale_mode_combo(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    let scale_label = match state.scale_mode {
        ScaleMode::Linear => "Lin",
        ScaleMode::Sqrt => "Sqrt",
        ScaleMode::Log => "Log",
        ScaleMode::Smooth => "Smo",
    };
    egui::ComboBox::from_id_salt("scale_mode")
        .selected_text(scale_label)
        .width(50.0)
        .show_ui(ui, |ui| {
            for mode in [ScaleMode::Linear, ScaleMode::Sqrt, ScaleMode::Log, ScaleMode::Smooth] {
                let label = match mode {
                    ScaleMode::Linear => "Linear",
                    ScaleMode::Sqrt => "Sqrt",
                    ScaleMode::Log => "Log",
                    ScaleMode::Smooth => "Smooth",
                };
                if ui.selectable_value(&mut state.scale_mode, mode, label).changed() {
                    *layout_changed = true;
                }
            }
        });
}

/// Visual group: Color mode and Theme combo boxes.
fn draw_visual_group(ui: &mut egui::Ui, state: &mut AppState, visual_changed: &mut bool) {
    ui.label(egui::RichText::new("color:").small().weak());
    let color_label = state.color_mode.label();
    let tier = license::current_tier();
    let available_modes: &[ColorMode] = if tier.is_pro() { ColorMode::ALL } else { ColorMode::FREE };
    egui::ComboBox::from_id_salt("color_mode")
        .selected_text(color_label)
        .width(80.0)
        .show_ui(ui, |ui| {
            for &mode in available_modes {
                if ui.selectable_value(&mut state.color_mode, mode, mode.label()).changed() {
                    *visual_changed = true;
                }
            }
        });

    let theme_label = state.theme.label();
    egui::ComboBox::from_id_salt("theme")
        .selected_text(theme_label)
        .width(70.0)
        .show_ui(ui, |ui| {
            for &theme in Theme::ALL {
                if ui
                    .selectable_value(&mut state.theme, theme, theme.label())
                    .changed()
                {
                    state.set_theme(theme);
                    *visual_changed = true;
                }
            }
        });
}

/// Filter group: Focus mode, Edge filter, edge/DSM/activity toggles.
fn draw_filter_group(
    ui: &mut egui::Ui,
    state: &mut AppState,
    layout_changed: &mut bool,
    visual_changed: &mut bool,
) {
    draw_focus_combo(ui, state, layout_changed);
    draw_edge_filter_combo(ui, state, visual_changed);
    draw_toggle_buttons(ui, state);
}

/// Focus mode combo box (All / EntryPoints / Directory / Language).
fn draw_focus_combo(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    ui.label(egui::RichText::new("Focus").small().weak());
    let focus_label = state.focus_mode.label();
    egui::ComboBox::from_id_salt("focus_mode")
        .selected_text(focus_label)
        .width(85.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(matches!(state.focus_mode, FocusMode::All), "All Files")
                .clicked()
            {
                state.focus_mode = FocusMode::All;
                *layout_changed = true;
            }
            if ui
                .selectable_label(
                    matches!(state.focus_mode, FocusMode::EntryPoints),
                    "Entry Points",
                )
                .clicked()
            {
                state.focus_mode = FocusMode::EntryPoints;
                *layout_changed = true;
            }
            draw_focus_dir_items(ui, state, layout_changed);
            draw_focus_lang_items(ui, state, layout_changed);
        });
}

/// Directory items inside the focus combo dropdown.
fn draw_focus_dir_items(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    if state.top_dirs.is_empty() {
        return;
    }
    ui.separator();
    ui.label(egui::RichText::new("Directories").small().weak());
    let mut clicked_dir = None;
    for i in 0..state.top_dirs.len() {
        let dir = &state.top_dirs[i];
        let is_sel = matches!(&state.focus_mode, FocusMode::Directory(d) if d == dir);
        if ui.selectable_label(is_sel, dir).clicked() {
            clicked_dir = Some(i);
        }
    }
    if let Some(i) = clicked_dir {
        state.focus_mode = FocusMode::Directory(state.top_dirs[i].clone());
        *layout_changed = true;
    }
}

/// Language items inside the focus combo dropdown.
fn draw_focus_lang_items(ui: &mut egui::Ui, state: &mut AppState, layout_changed: &mut bool) {
    if state.languages.is_empty() {
        return;
    }
    ui.separator();
    ui.label(egui::RichText::new("Languages").small().weak());
    let mut clicked_lang = None;
    for i in 0..state.languages.len() {
        let lang = &state.languages[i];
        let is_sel = matches!(&state.focus_mode, FocusMode::Language(l) if l == lang);
        if ui.selectable_label(is_sel, lang).clicked() {
            clicked_lang = Some(i);
        }
    }
    if let Some(i) = clicked_lang {
        state.focus_mode = FocusMode::Language(state.languages[i].clone());
        *layout_changed = true;
    }
}

/// Edge filter combo box.
fn draw_edge_filter_combo(ui: &mut egui::Ui, state: &mut AppState, visual_changed: &mut bool) {
    let ctx_label = state.edge_filter.label();
    egui::ComboBox::from_id_salt("edge_filter")
        .selected_text(ctx_label)
        .width(75.0)
        .show_ui(ui, |ui| {
            for &filter in EdgeFilter::ALL {
                if ui.selectable_value(&mut state.edge_filter, filter, filter.label()).changed() {
                    *visual_changed = true;
                }
            }
        });
}

/// Color for an active/inactive toggle state.
fn toggle_color(active: bool, active_color: egui::Color32) -> egui::Color32 {
    if active { active_color } else { egui::Color32::from_rgb(120, 120, 120) }
}

/// Draw the show-all-edges toggle button.
fn draw_edge_toggle(ui: &mut egui::Ui, state: &mut AppState) {
    let edge_icon = if state.show_all_edges { "\u{26A1}" } else { "\u{25C7}" };
    let color = toggle_color(state.show_all_edges, egui::Color32::from_rgb(220, 180, 80));
    let edge_btn = ui.add(
        egui::Button::new(egui::RichText::new(edge_icon).monospace().color(color))
            .fill(egui::Color32::TRANSPARENT),
    );
    if edge_btn.clicked() { state.show_all_edges = !state.show_all_edges; }
    let tip = if state.show_all_edges { "Showing all edges \u{2014} click to show only on hover" }
        else { "Edges shown on hover \u{2014} click to show all" };
    edge_btn.on_hover_text(tip);
}

/// Draw the DSM panel toggle button.
fn draw_dsm_toggle(ui: &mut egui::Ui, state: &mut AppState) {
    let color = toggle_color(state.dsm_panel_open, egui::Color32::from_rgb(100, 200, 180));
    let dsm_btn = ui.add(egui::Button::new(egui::RichText::new("DSM").monospace().size(9.0).color(color)));
    if dsm_btn.on_hover_text("Design Structure Matrix").clicked() { state.dsm_panel_open = !state.dsm_panel_open; }
}

/// Draw the activity panel toggle button.
fn draw_activity_toggle(ui: &mut egui::Ui, state: &mut AppState) {
    let act_label = if state.activity_panel_open { "\u{258C}" } else { "\u{2590}" };
    let color = toggle_color(state.activity_panel_open, egui::Color32::from_rgb(220, 180, 80));
    let act_btn = ui.add(egui::Button::new(egui::RichText::new(act_label).monospace().color(color)));
    if act_btn.on_hover_text("Activity Panel").clicked() { state.activity_panel_open = !state.activity_panel_open; }
}

/// Toggle buttons for show-all-edges, DSM panel, and activity panel.
fn draw_toggle_buttons(ui: &mut egui::Ui, state: &mut AppState) {
    draw_edge_toggle(ui, state);
    draw_dsm_toggle(ui, state);
    // Activity toggle removed — right panel is always visible

    // Export button — Free: grades only, Pro: full detail
    if state.health_report.is_some() {
        let is_pro = crate::license::current_tier().is_pro();
        let tip = if is_pro { "Export full report (Pro)" } else { "Export grade summary" };
        if ui.button("\u{2913}").on_hover_text(tip).clicked() {
            if let Some(report) = &state.health_report {
                let d = &report.dimensions;
                let summary = if is_pro {
                    // Pro: include counts, ratios, edge counts
                    serde_json::json!({
                        "grade": report.grade.to_string(),
                        "files": state.snapshot.as_ref().map(|s| s.total_files).unwrap_or(0),
                        "lines": state.snapshot.as_ref().map(|s| s.total_lines).unwrap_or(0),
                        "dimensions": {
                            "cycles": { "value": report.circular_dep_count, "grade": d.cycles.to_string() },
                            "coupling": { "value": format!("{:.2}", report.coupling_score), "grade": d.coupling.to_string() },
                            "entropy": { "value": format!("{:.2}", report.entropy), "grade": d.entropy.to_string() },
                            "cohesion": { "value": report.avg_cohesion.map(|c| format!("{:.2}", c)).unwrap_or("n/a".into()), "grade": d.cohesion.map(|c| c.to_string()).unwrap_or("-".into()) },
                            "depth": { "value": report.max_depth, "grade": d.depth.to_string() },
                            "complex_fn": { "count": report.complex_functions.len(), "ratio": format!("{:.2}", report.complex_fn_ratio), "grade": d.complex_fn.to_string() },
                            "dead_code": { "count": report.dead_functions.len(), "ratio": format!("{:.2}", report.dead_code_ratio), "grade": d.dead_code.to_string() },
                        },
                        "import_edges": report.total_import_edges,
                        "cross_module_edges": report.cross_module_edges,
                    })
                } else {
                    // Free: grades only, no counts or ratios
                    serde_json::json!({
                        "grade": report.grade.to_string(),
                        "dimensions": {
                            "cycles": d.cycles.to_string(),
                            "coupling": d.coupling.to_string(),
                            "entropy": d.entropy.to_string(),
                            "cohesion": d.cohesion.map(|c| c.to_string()).unwrap_or("-".into()),
                            "depth": d.depth.to_string(),
                            "complex_fn": d.complex_fn.to_string(),
                            "dead_code": d.dead_code.to_string(),
                        },
                    })
                };
                let json = serde_json::to_string_pretty(&summary).unwrap_or_default();
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("sentrux-report.json")
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    let _ = std::fs::write(&path, &json);
                }
            }
        }
    }
}

/// Scan progress spinner and percentage indicator.
fn draw_scan_progress(ui: &mut egui::Ui, state: &AppState) {
    if state.scanning {
        ui.add_space(4.0);
        // Rotating block chars: ▖▘▝▗
        let frames = ['▖', '▘', '▝', '▗'];
        let idx = ((ui.input(|i| i.time) * 6.0) as usize) % frames.len();
        ui.label(egui::RichText::new(frames[idx].to_string()).monospace());
        ui.label(
            egui::RichText::new(format!("{}  {}%", state.scan_step, state.scan_pct))
                .small()
                .weak(),
        );
    }
}
