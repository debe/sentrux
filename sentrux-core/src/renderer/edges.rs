//! Edge rendering — draws dependency polylines with arrowheads and dash animation.
//!
//! Supports viewport culling, edge-type filtering, spotlight mode (show only
//! edges for hovered/selected file), and animated dash offset for selected edges.

use super::{EdgePath, RenderData, ViewportTransform, Settings, RenderContext};
use super::edge_routing::draw_dashed_polyline;
use egui::{Color32, Stroke};

/// Returns true if the edge's world-space AABB intersects the viewport.
fn edge_visible(ep: &EdgePath, vp: &ViewportTransform) -> bool {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in &ep.pts {
        if p.x < min_x { min_x = p.x; }
        if p.y < min_y { min_y = p.y; }
        if p.x > max_x { max_x = p.x; }
        if p.y > max_y { max_y = p.y; }
    }
    vp.is_visible(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Returns true if the edge passes the spotlight filter.
/// When show_all_edges is off, only edges connected to the active file pass.
fn passes_spotlight(ep: &EdgePath, ctx: &RenderContext) -> bool {
    if ctx.show_all_edges {
        return true;
    }
    let active_file = ctx.selected_path
        .or(ctx.hovered_path);
    match active_file {
        Some(f) if ep.from_file == f || ep.to_file == f => true,
        Some(_) => false,
        None => false,
    }
}

/// Shared drawing parameters for rendering edges — groups settings and animation state.
struct EdgeDrawParams<'a> {
    painter: &'a egui::Painter,
    settings: &'a Settings,
    anim_time: f64,
}

/// Draw the dashed polyline for a single edge with the correct dash pattern.
fn draw_edge_line(
    edp: &EdgeDrawParams<'_>,
    screen_pts: &[egui::Pos2],
    stroke: Stroke,
    edge_type: &str,
) {
    // Each edge type: distinct dash pattern (visually obvious at any zoom)
    //   import = solid line (no gaps)
    //   call   = dashed  — — —
    //   inherit = dotted  · · ·
    let s = edp.settings;
    let (dash, gap) = match edge_type {
        "import" => (100.0_f32, 0.0_f32),
        "call" => (s.dash_len, s.dash_gap),
        _ => (2.0_f32, s.dash_gap),
    };
    let cycle = dash + gap;
    // f64 modulo avoids f32 precision loss after ~4.5h of animation.
    let offset = ((edp.anim_time * s.dash_anim_speed as f64) % (cycle as f64)) as f32;
    draw_dashed_polyline(edp.painter, screen_pts, stroke, offset, dash, gap);
}

/// Draw source bar and target stick markers at polyline endpoints.
fn draw_edge_endpoints(
    painter: &egui::Painter,
    screen_pts: &[egui::Pos2],
    from_side: char,
    color: Color32,
) {
    if screen_pts.len() < 2 {
        return;
    }
    let start = screen_pts[0];
    let end = screen_pts[screen_pts.len() - 1];

    // Source bar: perpendicular to exit border side.
    let (bar_w, bar_h) = match from_side {
        'l' | 'r' => (2.0_f32, 8.0_f32),
        _ => (8.0_f32, 2.0_f32),
    };
    painter.rect_filled(
        egui::Rect::from_center_size(start, egui::vec2(bar_w, bar_h)),
        egui::CornerRadius::ZERO, color,
    );

    // Target stick: parallel to line direction entering block.
    let prev = screen_pts[screen_pts.len() - 2];
    let tdx = (end.x - prev.x).abs();
    let tdy = (end.y - prev.y).abs();
    let (tw, th) = if tdx >= tdy {
        (8.0_f32, 2.0_f32)
    } else {
        (2.0_f32, 8.0_f32)
    };
    painter.rect_filled(
        egui::Rect::from_center_size(end, egui::vec2(tw, th)),
        egui::CornerRadius::ZERO, color,
    );
}

/// Draw edge polylines with arrowheads.
/// When spotlight is active, only draws edges connected to the selected file.
/// Supports animated dash offset for selected edges.
pub fn draw_edges(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    rd: &RenderData,
    ctx: &RenderContext,
) {
    let canvas_origin = clip_rect.min;
    let vp = &ctx.viewport;
    let edp = EdgeDrawParams {
        painter,
        settings: ctx.settings,
        anim_time: ctx.anim_time,
    };

    for ep in &rd.edge_paths {
        if ep.pts.len() < 2 { continue; }
        if !ctx.edge_filter.accepts(&ep.edge_type) { continue; }
        if !edge_visible(ep, vp) { continue; }
        if !passes_spotlight(ep, ctx) { continue; }

        let draw_alpha = ep.alpha.clamp(0.0, 1.0);
        let a = (draw_alpha * 255.0) as u8;
        // Use from_rgba_unmultiplied and let egui handle premultiplication
        // to avoid integer truncation bias. [M11 fix]
        let color = Color32::from_rgba_unmultiplied(ep.r, ep.g, ep.b, a);
        let stroke = Stroke::new(ep.line_w as f32, color);

        let screen_pts: Vec<egui::Pos2> = ep.pts.iter()
            .map(|p| egui::pos2(
                canvas_origin.x + vp.wx(p.x),
                canvas_origin.y + vp.wy(p.y),
            ))
            .collect();

        draw_edge_line(&edp, &screen_pts, stroke, &ep.edge_type);
        draw_edge_endpoints(painter, &screen_pts, ep.from_side, color);
    }
}
