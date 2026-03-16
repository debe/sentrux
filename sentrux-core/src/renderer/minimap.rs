//! Minimap overlay — small navigation thumbnail in the bottom-right corner.
//!
//! Renders a scaled-down view of the entire layout with language-colored file
//! blocks and a viewport indicator rectangle. Clicking the minimap pans the
//! main canvas. Always uses language colors regardless of the main color mode.

use super::{RectKind, RenderData, Settings, RenderContext, colors};
use egui::{CornerRadius, Rect, Stroke, StrokeKind};

/// Bundles the uniform scale and centering offsets for content → minimap mapping.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MinimapTransform {
    /// Uniform scale factor from content to minimap pixels
    pub scale: f64,
    /// Horizontal centering offset within the minimap
    pub offset_x: f64,
    /// Vertical centering offset within the minimap
    pub offset_y: f64,
}

/// Compute the minimap screen rect given the canvas clip rect.
pub fn minimap_rect(clip_rect: Rect, settings: &Settings) -> Rect {
    Rect::from_min_size(
        egui::pos2(
            clip_rect.right() - settings.minimap_w - settings.minimap_pad,
            clip_rect.bottom() - settings.minimap_h - settings.minimap_pad,
        ),
        egui::vec2(settings.minimap_w, settings.minimap_h),
    )
}

/// Compute uniform scale and centering offsets for content → minimap mapping.
fn minimap_transform(mm_w: f32, mm_h: f32, cw: f64, ch: f64) -> MinimapTransform {
    let scale_x = mm_w as f64 / cw;
    let scale_y = mm_h as f64 / ch;
    let scale = scale_x.min(scale_y);
    let offset_x = ((mm_w as f64 - cw * scale) / 2.0).max(0.0);
    let offset_y = ((mm_h as f64 - ch * scale) / 2.0).max(0.0);
    MinimapTransform { scale, offset_x, offset_y }
}

/// Draw scaled file rects on the minimap using language colors.
fn draw_minimap_files(
    painter: &egui::Painter,
    mm_rect: Rect,
    rd: &RenderData,
    ctx: &RenderContext,
    transform: &MinimapTransform,
) {
    let tc = &ctx.theme_config;
    for r in &rd.rects {
        if r.kind != RectKind::File { continue; }
        let mx = mm_rect.left() + transform.offset_x as f32 + (r.x * transform.scale) as f32;
        let my = mm_rect.top() + transform.offset_y as f32 + (r.y * transform.scale) as f32;
        let mw = (r.w * transform.scale).max(1.0) as f32;
        let mh = (r.h * transform.scale).max(1.0) as f32;

        let color = {
            let lang = ctx.file_index.get(&r.path)
                .map(|e| e.lang.as_str())
                .unwrap_or("unknown");
            colors::language_color(lang)
        };
        let rect = Rect::from_min_size(egui::pos2(mx, my), egui::vec2(mw, mh));
        let inset = if mw > 3.0 && mh > 3.0 { rect.shrink(1.0) } else { rect };
        painter.rect_filled(inset, CornerRadius::ZERO, color);
        if mw > 3.0 && mh > 3.0 {
            painter.rect_stroke(inset, CornerRadius::ZERO, Stroke::new(1.0, tc.minimap_bg), StrokeKind::Middle);
        }
    }
}

/// Draw the viewport indicator rectangle, clamped to minimap bounds.
fn draw_viewport_indicator(
    painter: &egui::Painter,
    mm_rect: Rect,
    ctx: &RenderContext,
    transform: &MinimapTransform,
) {
    let vp = &ctx.viewport;
    if vp.scale <= 0.0 { return; }
    let vx = mm_rect.left() + transform.offset_x as f32 + (vp.offset_x * transform.scale) as f32;
    let vy = mm_rect.top() + transform.offset_y as f32 + (vp.offset_y * transform.scale) as f32;
    let vw = (vp.canvas_w / vp.scale * transform.scale) as f32;
    let vh = (vp.canvas_h / vp.scale * transform.scale) as f32;
    let viewport_rect = Rect::from_min_size(egui::pos2(vx, vy), egui::vec2(vw, vh));
    let clamped = viewport_rect.intersect(mm_rect);
    let indicator = if clamped.width() > 0.0 && clamped.height() > 0.0 {
        clamped
    } else {
        let cx = vx.clamp(mm_rect.left(), mm_rect.right() - 2.0);
        let cy = vy.clamp(mm_rect.top(), mm_rect.bottom() - 2.0);
        Rect::from_min_size(egui::pos2(cx, cy), egui::vec2(2.0, 2.0))
    };
    painter.rect_stroke(indicator, CornerRadius::ZERO, Stroke::new(1.0, ctx.theme_config.minimap_viewport), StrokeKind::Middle);
}

/// Draw minimap in bottom-right corner. Pixel-sharp, no rounded corners.
pub fn draw_minimap(
    painter: &egui::Painter,
    clip_rect: Rect,
    rd: &RenderData,
    ctx: &RenderContext,
) {
    if rd.content_width <= 0.0 || rd.content_height <= 0.0 { return; }

    let s = &ctx.settings;
    let mm_rect = minimap_rect(clip_rect, s);
    let tc = &ctx.theme_config;
    painter.rect_filled(mm_rect, CornerRadius::ZERO, tc.minimap_bg);
    painter.rect_stroke(mm_rect, CornerRadius::ZERO, Stroke::new(1.0, tc.minimap_border), StrokeKind::Middle);

    let transform = minimap_transform(s.minimap_w, s.minimap_h, rd.content_width, rd.content_height);
    draw_minimap_files(painter, mm_rect, rd, ctx, &transform);
    draw_viewport_indicator(painter, mm_rect, ctx, &transform);
}

/// Convert a click position on the minimap to a world-space center point.
/// Returns None if the click is outside the minimap.
pub fn minimap_click_to_world(
    click_pos: egui::Pos2,
    clip_rect: Rect,
    rd: &RenderData,
    settings: &Settings,
) -> Option<(f64, f64)> {
    let mm = minimap_rect(clip_rect, settings);
    if !mm.contains(click_pos) {
        return None;
    }

    if rd.content_width <= 0.0 || rd.content_height <= 0.0 {
        return None;
    }

    let transform = minimap_transform(settings.minimap_w, settings.minimap_h, rd.content_width, rd.content_height);

    let world_x = ((click_pos.x as f64 - mm.left() as f64 - transform.offset_x) / transform.scale)
        .clamp(0.0, rd.content_width);
    let world_y = ((click_pos.y as f64 - mm.top() as f64 - transform.offset_y) / transform.scale)
        .clamp(0.0, rd.content_height);
    Some((world_x, world_y))
}
