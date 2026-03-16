//! Entry-point badge rendering — small colored dots on entry-point file blocks.
//!
//! Badges are drawn in screen space (fixed pixel size regardless of zoom).
//! High-confidence entry points get the theme's badge_high color; low-confidence
//! get badge_low. Only visible when the file block is large enough on screen.

use super::{RectKind, RenderData, RenderContext};
use egui::{CornerRadius, Stroke, StrokeKind};

/// Draw entry-point badges at the top-right corner of file rects.
/// Rendered in screen space so they always appear correctly positioned
/// regardless of zoom level. Size is fixed in screen pixels (not world).
pub fn draw_badges(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    rd: &RenderData,
    ctx: &RenderContext,
) {
    let ep_set = build_entry_point_set(ctx);
    if ep_set.is_empty() {
        return;
    }

    let canvas_origin = clip_rect.min;
    let vp = &ctx.viewport;
    let badge_size = 6.0_f32;

    let inset = ctx.settings.file_rect_inset;
    for r in &rd.rects {
        if let Some(confidence) = badge_candidate(r, &ep_set, vp) {
            let screen_rect = vp.world_to_screen_rect(r.x, r.y, r.w, r.h, canvas_origin).shrink(inset);
            if screen_rect.width() >= 14.0 && screen_rect.height() >= 14.0 {
                draw_single_badge(painter, screen_rect, badge_size, confidence, ctx);
            }
        }
    }
}

/// Check if a rect is an entry-point file that is visible. Returns confidence if so.
fn badge_candidate<'a>(
    r: &crate::layout::types::LayoutRectSlim,
    ep_set: &std::collections::HashMap<&str, &'a str>,
    vp: &crate::layout::viewport::ViewportTransform,
) -> Option<&'a str> {
    if r.kind != RectKind::File { return None; }
    let confidence = *ep_set.get(r.path.as_str())?;
    if !vp.is_visible(r.x, r.y, r.w, r.h) { return None; }
    Some(confidence)
}

/// Build a map of entry-point file paths to their confidence level.
fn build_entry_point_set<'a>(ctx: &'a RenderContext) -> std::collections::HashMap<&'a str, &'a str> {
    ctx.snapshot
        .as_ref()
        .map(|snap| {
            snap.entry_points
                .iter()
                .map(|ep| (ep.file.as_str(), ep.confidence.as_str()))
                .collect()
        })
        .unwrap_or_default()
}

/// Draw a single badge dot at the top-right corner of a screen rect.
fn draw_single_badge(
    painter: &egui::Painter,
    screen_rect: egui::Rect,
    badge_size: f32,
    confidence: &str,
    ctx: &RenderContext,
) {
    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(
            screen_rect.right() - badge_size - 2.0,
            screen_rect.top() + 2.0,
        ),
        egui::vec2(badge_size, badge_size),
    );

    let tc = &ctx.theme_config;
    let fill = if confidence == "high" { tc.badge_high } else { tc.badge_low };

    painter.rect_filled(badge_rect, CornerRadius::ZERO, fill);
    painter.rect_stroke(badge_rect, CornerRadius::ZERO, Stroke::new(1.0, tc.section_border), StrokeKind::Middle);
}
