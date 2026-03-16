//! DSM panel — renders the Design Structure Matrix in a side panel.
//!
//! Shows the NxN dependency matrix with:
//! - Color-coded cells (below diagonal = green, above = red)
//! - Level grouping with separator lines
//! - Stats summary at top
//! - Click to select a file in the main treemap

use crate::metrics::dsm::{self, DesignStructureMatrix, DsmStats};
use crate::app::state::AppState;

/// Draw the DSM panel (left side).
/// Returns true if a file was clicked (to highlight in treemap).
pub fn draw_dsm_panel(ctx: &egui::Context, state: &mut AppState) -> bool {
    let mut clicked = false;
    let tc = state.theme_config.clone();

    egui::SidePanel::left("dsm_panel")
        .default_width(400.0)
        .min_width(250.0)
        .max_width(600.0)
        .frame(
            egui::Frame::NONE
                .fill(tc.canvas_bg)
                .inner_margin(egui::Margin::same(4))
                .stroke(egui::Stroke::new(1.0, tc.section_border)),
        )
        .show(ctx, |ui| {
            draw_dsm_header(ui, state, &tc);
            ui.separator();
            if let Some(c) = draw_dsm_body(ui, state, &tc) {
                clicked = c;
            }
        });

    clicked
}

/// DSM panel header with title and close button.
fn draw_dsm_header(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tc: &crate::core::settings::ThemeConfig,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(
            egui::RichText::new("┌ DSM")
                .monospace()
                .size(10.0)
                .color(tc.section_label),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = ui.add(
                egui::Button::new(
                    egui::RichText::new("×")
                        .monospace()
                        .size(11.0)
                        .color(tc.text_secondary),
                )
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE),
            );
            if close.clicked() {
                state.dsm_panel_open = false;
            }
            close.on_hover_cursor(egui::CursorIcon::PointingHand);
        });
    });
}

/// DSM body: rebuild cache, show stats and matrix. Returns Some(true) if clicked.
fn draw_dsm_body(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tc: &crate::core::settings::ThemeConfig,
) -> Option<bool> {
    if state.snapshot.is_none() {
        ui.label(
            egui::RichText::new("Scan a project first")
                .monospace()
                .size(9.0)
                .color(tc.text_secondary),
        );
        return None;
    }
    // Key DSM cache by snapshot content fingerprint instead of pointer identity.
    // Pointer identity can produce false cache hits if the allocator reuses an
    // address after a dropped Arc. [H9 fix]
    let snap = state.snapshot.as_ref().unwrap();
    let snap_key = snap.import_graph.len() as u64 * 1_000_003
        + snap.total_files as u64 * 7
        + snap.total_lines as u64;
    let needs_rebuild = state.dsm_cache.as_ref().is_none_or(|(v, _, _)| *v != snap_key);
    if needs_rebuild {
        let m = dsm::build_dsm(&snap.import_graph);
        let s = dsm::compute_stats(&m);
        state.dsm_cache = Some((snap_key, m, s));
    }
    let (_, ref matrix, ref stats) = state.dsm_cache.as_ref().unwrap();
    let size = matrix.size;
    let total_files = matrix.total_files;
    let dropped_level_range = matrix.dropped_level_range;

    draw_stats(ui, stats, tc, total_files, dropped_level_range);
    ui.separator();

    if size == 0 {
        ui.label(
            egui::RichText::new("No import edges to display")
                .monospace()
                .size(9.0)
                .color(tc.text_secondary),
        );
        return None;
    }
    let selected = state.selected_path.clone();
    let click_result = egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| draw_matrix(ui, matrix, tc, selected.as_deref()))
        .inner;
    if let Some(path) = click_result {
        state.selected_path = Some(path);
        return Some(true);
    }
    None
}

fn draw_stats(
    ui: &mut egui::Ui,
    stats: &DsmStats,
    tc: &crate::core::settings::ThemeConfig,
    total_files: usize,
    dropped_level_range: Option<(u32, u32)>,
) {
    let mono = |s: &str| egui::RichText::new(s).monospace().size(9.0);
    draw_stats_file_counts(ui, stats, tc, total_files, dropped_level_range, &mono);
    draw_stats_direction_row(ui, stats, &mono);
    ui.label(mono(&format!("Propagation: {}", (stats.propagation_cost * 10000.0).round() as u32)).color(tc.text_secondary));
    draw_stats_clusters(ui, stats, tc, &mono);
}

/// File count, density, and truncation indicator lines.
fn draw_stats_file_counts(
    ui: &mut egui::Ui,
    stats: &DsmStats,
    tc: &crate::core::settings::ThemeConfig,
    total_files: usize,
    dropped_level_range: Option<(u32, u32)>,
    mono: &dyn Fn(&str) -> egui::RichText,
) {
    if total_files > stats.size {
        ui.label(mono(&format!("Files: {} of {}  Edges: {}", stats.size, total_files, stats.edge_count)).color(tc.text_primary));
        ui.label(mono(&format!("(sampled {} of {} — metrics approximate)", stats.size, total_files))
            .color(egui::Color32::from_rgb(220, 170, 80)));
        if let Some((lo, hi)) = dropped_level_range {
            ui.label(mono(&format!("(levels L{}–L{} omitted)", lo, hi))
                .color(egui::Color32::from_rgb(180, 150, 80)));
        } else {
            ui.label(mono("(middle-level files omitted)")
                .color(egui::Color32::from_rgb(180, 150, 80)));
        }
    } else {
        ui.label(mono(&format!("Files: {}  Edges: {}", stats.size, stats.edge_count)).color(tc.text_primary));
    }
    ui.label(mono(&format!("Density: {}", (stats.density * 10000.0).round() as u32)).color(tc.text_secondary));
}

/// Below/above diagonal and same-level edge count row.
fn draw_stats_direction_row(
    ui: &mut egui::Ui,
    stats: &DsmStats,
    mono: &dyn Fn(&str) -> egui::RichText,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(
            mono(&format!("▼ {}", stats.below_diagonal))
                .color(egui::Color32::from_rgb(100, 200, 100)),
        );
        if stats.above_diagonal > 0 {
            ui.label(
                mono(&format!("▲ {}", stats.above_diagonal))
                    .color(egui::Color32::from_rgb(220, 100, 100)),
            );
        } else {
            ui.label(
                mono("▲ 0")
                    .color(egui::Color32::from_rgb(100, 200, 100)),
            );
        }
        if stats.same_level > 0 {
            ui.label(
                mono(&format!("↔ {}", stats.same_level))
                    .color(egui::Color32::from_rgb(100, 160, 160)),
            );
        }
    });
}

/// Cluster summary lines.
fn draw_stats_clusters(
    ui: &mut egui::Ui,
    stats: &DsmStats,
    tc: &crate::core::settings::ThemeConfig,
    mono: &dyn Fn(&str) -> egui::RichText,
) {
    if !stats.clusters.is_empty() {
        ui.add_space(2.0);
        ui.label(mono(&format!("Clusters: {}", stats.clusters.len())).color(tc.text_primary));
        for (i, c) in stats.clusters.iter().take(3).enumerate() {
            ui.label(
                mono(&format!("  #{}: {} files, {} internal edges (L{})", i + 1, c.files.len(), c.internal_edges, c.level))
                    .color(tc.text_secondary),
            );
        }
    }
}

/// Shared draw context for DSM matrix rendering — bundles the parameters
/// that every matrix drawing helper needs (reduces draw_row_label: 9->4,
/// cell_color: 8->4, draw_matrix_chrome: 8->3).
struct DsmDrawCtx<'a> {
    tc: &'a crate::core::settings::ThemeConfig,
    label_width: f32,
    cell_size: f32,
    display_size: usize,
    colors: DsmColors,
    hover_row: Option<usize>,
    hover_col: Option<usize>,
    selected_row: Option<usize>,
}

/// Constant colors used by DSM cell rendering.
struct DsmColors {
    diag: egui::Color32,
    below: egui::Color32,
    above: egui::Color32,
    same_level: egui::Color32,
    hover: egui::Color32,
    level_break: egui::Color32,
}

impl DsmColors {
    fn new() -> Self {
        Self {
            diag: egui::Color32::from_rgb(80, 80, 100),
            below: egui::Color32::from_rgb(50, 140, 80),
            above: egui::Color32::from_rgb(180, 60, 60),
            same_level: egui::Color32::from_rgb(80, 120, 120),
            hover: egui::Color32::from_rgb(100, 100, 140),
            level_break: egui::Color32::from_rgb(60, 60, 80),
        }
    }
}

/// Detect which cell the mouse is hovering over. Returns (row, col) indices.
fn detect_hover(
    hover_pos: Option<egui::Pos2>,
    origin: egui::Pos2,
    label_width: f32,
    cell_size: f32,
    display_size: usize,
) -> (Option<usize>, Option<usize>) {
    let mut hover_row: Option<usize> = None;
    let mut hover_col: Option<usize> = None;
    if let Some(pos) = hover_pos {
        let rel_x = pos.x - origin.x - label_width;
        let rel_y = pos.y - origin.y - cell_size;
        if rel_x >= 0.0 && rel_y >= 0.0 {
            let col = (rel_x / cell_size) as usize;
            let row = (rel_y / cell_size) as usize;
            if col < display_size && row < display_size {
                hover_row = Some(row);
                hover_col = Some(col);
            }
        }
    }
    (hover_row, hover_col)
}

/// Pick the direction color based on row/col levels.
#[inline]
fn direction_color(rl: u32, cl: u32, c: &DsmColors) -> egui::Color32 {
    if rl > cl { c.below } else if rl < cl { c.above } else { c.same_level }
}

/// Compute the color for a single DSM cell.
fn cell_color(
    row: usize, col: usize, row_levels: &[u32],
    matrix: &DesignStructureMatrix, dctx: &DsmDrawCtx<'_>,
) -> Option<egui::Color32> {
    if row == col {
        return Some(dctx.colors.diag);
    }

    let has_edge = matrix.matrix[row][col];
    let is_hovered = dctx.hover_row == Some(row) || dctx.hover_col == Some(col);

    if has_edge {
        Some(direction_color(row_levels[row], row_levels[col], &dctx.colors))
    } else if is_hovered {
        Some(dctx.colors.hover.linear_multiply(0.3))
    } else if dctx.selected_row.is_some_and(|sr| row == sr || col == sr) {
        Some(egui::Color32::from_rgba_unmultiplied(100, 100, 180, 25))
    } else {
        None
    }
}

/// Draw column index labels and level break lines on the DSM grid.
fn draw_matrix_chrome(
    painter: &egui::Painter, origin: egui::Pos2,
    matrix: &DesignStructureMatrix, dctx: &DsmDrawCtx<'_>,
) {
    let label_width = dctx.label_width;
    let cell_size = dctx.cell_size;
    let display_size = dctx.display_size;

    // Column index labels (abbreviated)
    for col in 0..display_size {
        if col % 5 == 0 {
            let x = origin.x + label_width + (col as f32 * cell_size);
            let y = origin.y;
            painter.text(
                egui::pos2(x + cell_size * 0.5, y + cell_size * 0.5),
                egui::Align2::CENTER_CENTER,
                format!("{col}"),
                egui::FontId::monospace(6.0),
                dctx.tc.text_secondary,
            );
        }
    }

    // Level break lines
    for &brk in &matrix.level_breaks {
        if brk < display_size {
            let x = origin.x + label_width + (brk as f32 * cell_size);
            let y_start = origin.y + cell_size;
            let y_end = origin.y + cell_size + (display_size as f32 * cell_size);
            painter.line_segment(
                [egui::pos2(x, y_start), egui::pos2(x, y_end)],
                egui::Stroke::new(0.5, dctx.colors.level_break),
            );
            let x_start = origin.x + label_width;
            let x_end = origin.x + label_width + (display_size as f32 * cell_size);
            let y = origin.y + cell_size + (brk as f32 * cell_size);
            painter.line_segment(
                [egui::pos2(x_start, y), egui::pos2(x_end, y)],
                egui::Stroke::new(0.5, dctx.colors.level_break),
            );
        }
    }
}

/// Draw row labels for the DSM matrix.
fn draw_row_label(
    painter: &egui::Painter, origin: egui::Pos2,
    row: usize, label: &str, dctx: &DsmDrawCtx<'_>,
) {
    let y = origin.y + dctx.cell_size + (row as f32 * dctx.cell_size);
    let short = if label.len() > 18 {
        let start = label.len() - 18;
        let start = label.ceil_char_boundary(start);
        &label[start..]
    } else {
        label
    };

    let label_color = if dctx.hover_row == Some(row) || dctx.selected_row == Some(row) {
        dctx.tc.text_primary
    } else {
        dctx.tc.text_secondary
    };

    painter.text(
        egui::pos2(origin.x + dctx.label_width - 4.0, y + dctx.cell_size * 0.5),
        egui::Align2::RIGHT_CENTER,
        short,
        egui::FontId::monospace(7.0),
        label_color,
    );
}

/// Build tooltip text for a hovered DSM cell.
fn hover_tooltip(matrix: &DesignStructureMatrix, row: usize, col: usize) -> String {
    let from = &matrix.files[row];
    let to = &matrix.files[col];
    let has_edge = matrix.matrix[row][col];
    let level_from = matrix.levels.get(from).copied().unwrap_or(0);
    let level_to = matrix.levels.get(to).copied().unwrap_or(0);

    if row == col {
        format!("{from} (L{level_from})")
    } else if has_edge {
        let dir = if level_from > level_to {
            "↓ correct"
        } else if level_from < level_to {
            "↑ INVERSION"
        } else {
            "↔ same level"
        };
        format!("{from} → {to}\nL{level_from} → L{level_to} ({dir})")
    } else {
        format!("{from} ⊘ {to}\nNo dependency")
    }
}

/// Draw the DSM matrix grid. Returns Some(path) if a file row was clicked.
fn draw_matrix(
    ui: &mut egui::Ui,
    matrix: &DesignStructureMatrix,
    tc: &crate::core::settings::ThemeConfig,
    selected_path: Option<&str>,
) -> Option<String> {
    let display_size = matrix.size.min(200);
    let cell_size = 8.0_f32;
    let label_width = 120.0_f32;

    let selected_row: Option<usize> = selected_path.and_then(|sp| {
        matrix.files.iter().take(display_size).position(|f| f == sp)
    });

    let total_w = label_width + (display_size as f32 * cell_size) + 8.0;
    let total_h = (display_size as f32 * cell_size) + cell_size + 8.0;

    let (response, painter) = ui.allocate_painter(
        egui::vec2(total_w, total_h),
        egui::Sense::click(),
    );
    let origin = response.rect.min;

    let (hover_row, hover_col) = detect_hover(
        response.hover_pos(), origin, label_width, cell_size, display_size,
    );

    let dctx = DsmDrawCtx {
        tc,
        label_width,
        cell_size,
        display_size,
        colors: DsmColors::new(),
        hover_row,
        hover_col,
        selected_row,
    };

    draw_matrix_chrome(&painter, origin, matrix, &dctx);

    // Pre-compute per-row levels for O(1) lookup during cell coloring.
    let row_levels: Vec<u32> = (0..display_size)
        .map(|i| matrix.levels.get(&matrix.files[i]).copied().unwrap_or(0))
        .collect();

    // Pre-compute per-row edge counts to skip empty rows that have no
    // hover/selection crosshair.  Reduces 40k paint calls to ~edge_count
    // for typical sparse matrices.
    let has_crosshair = hover_row.is_some() || hover_col.is_some() || selected_row.is_some();
    let row_has_edges: Vec<bool> = if has_crosshair {
        // When crosshair is active, we may need to draw highlight on any row
        vec![true; display_size]
    } else {
        (0..display_size)
            .map(|r| (0..display_size).any(|c| matrix.matrix[r][c] || r == c))
            .collect()
    };

    // Draw rows: labels + cells
    for row in 0..display_size {
        draw_row_label(&painter, origin, row, &matrix.files[row], &dctx);

        // Skip rows with no edges and no crosshair interaction
        if !row_has_edges[row] {
            continue;
        }

        let y = origin.y + cell_size + (row as f32 * cell_size);
        for col in 0..display_size {
            if let Some(color) = cell_color(row, col, &row_levels, matrix, &dctx) {
                let x = origin.x + label_width + (col as f32 * cell_size);
                let rect = egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(cell_size - 0.5, cell_size - 0.5),
                );
                painter.rect_filled(rect, 0.0, color);
            }
        }
    }

    // Handle click -> return clicked file path
    let mut click_result: Option<String> = None;
    if response.clicked() {
        if let Some(row) = hover_row {
            if row < matrix.files.len() {
                click_result = Some(matrix.files[row].clone());
            }
        }
    }

    // Tooltip on hover
    if let (Some(row), Some(col)) = (hover_row, hover_col) {
        if row < matrix.files.len() && col < matrix.files.len() {
            let tip = hover_tooltip(matrix, row, col);
            response.on_hover_text(tip);
        }
    }

    if display_size < matrix.size {
        ui.label(
            egui::RichText::new(format!("({} more files not shown)", matrix.size - display_size))
                .monospace()
                .size(8.0)
                .color(tc.text_secondary),
        );
    }

    click_result
}
