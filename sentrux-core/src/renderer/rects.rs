//! Rectangle rendering — draws file blocks and directory sections.
//!
//! Handles color mode selection, connectivity dimming (unconnected files fade),
//! hover/selection highlighting, header strips for directory sections, and
//! zoom-proportional text labels with monospace font.

use crate::layout::types::{ColorMode, LayoutRectSlim, RectKind, RenderData};
use crate::layout::viewport::ViewportTransform;
use super::colors;
use crate::core::heat;
use super::RenderContext;
use crate::layout::types::EdgeFilter;
use egui::{Color32, CornerRadius, Stroke, StrokeKind};
use std::collections::HashSet;

/// sRGB → linear (WCAG formula).
#[inline]
fn lin(c: u8) -> f32 {
    let s = c as f32 / 255.0;
    if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
}

/// Compute relative luminance (WCAG 2.0).
#[inline]
fn luminance(r: u8, g: u8, b: u8) -> f32 {
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}


/// Adaptive high-contrast text color for any background.
///
/// Instead of harsh black/white, produces elegant tinted text:
/// - On dark backgrounds: warm off-white (slightly tinted toward bg hue)
/// - On light backgrounds: deep charcoal (not pure black — softer on eyes)
/// - WCAG 4.5:1 minimum contrast guaranteed
///
/// The tinting makes text feel "part of" the block's color scheme rather than
/// fighting against it — same principle Apple/Google use in their design systems.
#[inline]
fn contrast_text_color(bg: Color32, _light: Color32, _dark: Color32) -> Color32 {
    let [r, g, b, _] = bg.to_array();
    let bg_lum = luminance(r, g, b);

    if bg_lum > 0.18 {
        // Light background → deep charcoal text (not pure black)
        // Tint slightly toward the bg color for harmony
        let tr = 15 + (r / 12);
        let tg = 15 + (g / 12);
        let tb = 18 + (b / 12);
        Color32::from_rgb(tr, tg, tb)
    } else {
        // Dark background → warm off-white (not pure white)
        // Slightly desaturate toward the bg hue for elegance
        let base: u8 = 220;
        let tr = base + (r / 20).min(35);
        let tg = base + (g / 20).min(35);
        let tb = base + (b / 20).min(35);
        Color32::from_rgb(tr, tg, tb)
    }
}

/// Adaptive color for secondary/stats text — lower contrast than label, but still readable.
/// WCAG 3:1 minimum for large text / secondary information.
#[inline]
fn contrast_secondary_color(bg: Color32) -> Color32 {
    let [r, g, b, _] = bg.to_array();
    let bg_lum = luminance(r, g, b);

    if bg_lum > 0.18 {
        // Light bg → medium gray with slight tint
        let tr = 80 + (r / 10);
        let tg = 80 + (g / 10);
        let tb = 85 + (b / 10);
        Color32::from_rgb(tr, tg, tb)
    } else {
        // Dark bg → dimmed warm gray
        let base: u8 = 150;
        let tr = base + (r / 16).min(30);
        let tg = base + (g / 16).min(30);
        let tb = base + (b / 16).min(30);
        Color32::from_rgb(tr, tg, tb)
    }
}

/// Bundles common drawing parameters to reduce function argument counts.
struct DrawCtx<'a> {
    painter: &'a egui::Painter,
    tc: &'a crate::core::settings::ThemeConfig,
    fs: f32,
    cw: f32,
    px: f32,
    py: f32,
}

/// Draw all layout rectangles of a given kind (file or section) onto the painter.
pub fn draw_rects(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    rd: &RenderData,
    ctx: &RenderContext,
    kind: RectKind,
    lod_full: bool,
) {
    let canvas_origin = clip_rect.min;
    let vp = &ctx.viewport;
    let tc = &ctx.theme_config;

    // Pre-build connectivity set for hover/selected file — dim unconnected files.
    let connected_files: Option<HashSet<&str>> = build_connected_set(rd, ctx, kind, lod_full);

    // ONE global font size for ALL text. Scales with zoom.
    let fs = (ctx.settings.font_scale * 72.0 * vp.scale as f32).clamp(4.0, 40.0);
    let cw = fs * 0.62; // monospace char width
    let px = fs * 0.25;
    let py = fs * 0.15;

    let dctx = DrawCtx { painter, tc, fs, cw, px, py };

    for r in &rd.rects {
        if r.kind != kind {
            continue;
        }

        // Viewport culling
        if !vp.is_visible(r.x, r.y, r.w, r.h) {
            continue;
        }

        let screen_rect = vp.world_to_screen_rect(r.x, r.y, r.w, r.h, canvas_origin);

        // Skip sub-pixel rects
        if screen_rect.width() < 1.0 || screen_rect.height() < 1.0 {
            continue;
        }

        match kind {
            RectKind::Section | RectKind::Root => {
                draw_section_rect(&dctx, screen_rect, r, ctx, vp, lod_full);
            }
            RectKind::File => {
                draw_file_rect(&dctx, screen_rect, r, ctx, &connected_files, lod_full);
            }
        }
    }
}

/// Build set of files connected to the active (hovered/selected) file via edges.
/// Returns None if no file is active or not in file-level full-LOD mode.
fn build_connected_set<'a>(
    rd: &'a RenderData,
    ctx: &'a RenderContext,
    kind: RectKind,
    lod_full: bool,
) -> Option<HashSet<&'a str>> {
    if kind != RectKind::File || !lod_full {
        return None;
    }
    let active_file = ctx.selected_path.or(ctx.hovered_path);
    active_file.map(|af| {
        let mut set = HashSet::new();
        set.insert(af);
        let adj = &rd.edge_adjacency;
        let edge_type = match ctx.edge_filter {
            EdgeFilter::All => "all",
            EdgeFilter::Imports => "import",
            EdgeFilter::Calls => "call",
            EdgeFilter::Inherit => "inherit",
        };
        for neighbor in adj.connected(af, edge_type) {
            set.insert(neighbor);
        }
        set
    })
}

/// Render a section/root rectangle: background, border, header strip, and label.
fn draw_section_rect(
    dctx: &DrawCtx,
    screen_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
    vp: &ViewportTransform,
    lod_full: bool,
) {
    let bg = dctx.tc.section_color(r.depth);
    dctx.painter.rect_filled(screen_rect, CornerRadius::ZERO, bg);

    if screen_rect.width() > 10.0 {
        dctx.painter.rect_stroke(
            screen_rect,
            CornerRadius::ZERO,
            Stroke::new(1.0, dctx.tc.section_border),
            StrokeKind::Middle,
        );
    }

    let strip_h = vp.ws(r.header_h);
    if lod_full && strip_h > 4.0 && screen_rect.width() > 20.0 {
        draw_section_header(dctx, screen_rect, r, ctx, strip_h);
    }
}

/// Render the header strip and label text for a section rectangle.
fn draw_section_header(
    dctx: &DrawCtx,
    screen_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
    strip_h: f32,
) {
    let strip = egui::Rect::from_min_size(
        screen_rect.left_top(),
        egui::vec2(screen_rect.width(), strip_h),
    );
    dctx.painter.rect_filled(strip, CornerRadius::ZERO, dctx.tc.header_strip_bg);

    if dctx.fs + dctx.py >= strip_h {
        return;
    }

    let label = if r.path.is_empty() || r.path == "/" {
        ctx.root_path.unwrap_or("/").to_string()
    } else {
        let dirname = r.path.rsplit('/').next().unwrap_or(&r.path);
        format!("./{}/", dirname)
    };

    let max_chars = ((screen_rect.width() - dctx.px * 2.0) / dctx.cw).max(0.0) as usize;
    let display = if max_chars < 3 {
        ""
    } else if label.chars().count() > max_chars {
        &label[..label.floor_char_boundary(max_chars)]
    } else {
        &label
    };
    if !display.is_empty() {
        dctx.painter.text(
            egui::pos2(screen_rect.left() + dctx.px, screen_rect.top() + dctx.py),
            egui::Align2::LEFT_TOP,
            display,
            egui::FontId::monospace(dctx.fs),
            dctx.tc.section_label,
        );
    }
}

/// Compute the final display color for a file rect, applying spotlight dimming.
fn file_display_color(
    ctx: &RenderContext,
    path: &str,
    connected_files: &Option<HashSet<&str>>,
    lod_full: bool,
) -> Color32 {
    let base_color = file_color(ctx, path);
    if !lod_full {
        return base_color;
    }

    // Search highlighting: when search is active, highlight matches, dim others
    if !ctx.search_query.is_empty() {
        let filename = path.rsplit('/').next().unwrap_or(path);
        let query_lower = ctx.search_query.to_lowercase();
        let matches = filename.to_lowercase().contains(&query_lower)
            || path.to_lowercase().contains(&query_lower);
        if matches {
            // Brighten matching files
            let [r, g, b, _] = base_color.to_array();
            return Color32::from_rgb(
                (r as f32 + (255.0 - r as f32) * 0.35) as u8,
                (g as f32 + (255.0 - g as f32) * 0.35) as u8,
                (b as f32 + (255.0 - b as f32) * 0.35) as u8,
            );
        } else {
            // Heavily dim non-matching files
            let [r, g, b, _] = base_color.to_array();
            return Color32::from_rgb(r / 4, g / 4, b / 4);
        }
    }

    let has_spotlight = connected_files.is_some();
    let is_spotlit = connected_files.as_ref().is_some_and(|c| c.contains(path));
    if is_spotlit {
        if ctx.color_mode == ColorMode::Monochrome {
            ctx.theme_config.file_surface_spotlit
        } else {
            let [r, g, b, _] = base_color.to_array();
            let factor = 0.25_f32;
            Color32::from_rgb(
                (r as f32 + (255.0 - r as f32) * factor) as u8,
                (g as f32 + (255.0 - g as f32) * factor) as u8,
                (b as f32 + (255.0 - b as f32) * factor) as u8,
            )
        }
    } else if has_spotlight {
        let [r, g, b, _] = base_color.to_array();
        Color32::from_rgb(r / 2, g / 2, b / 2)
    } else {
        base_color
    }
}

/// Render a file rectangle: fill, border, hover/selected highlights, and text.
fn draw_file_rect(
    dctx: &DrawCtx,
    screen_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
    connected_files: &Option<HashSet<&str>>,
    lod_full: bool,
) {
    let color = file_display_color(ctx, &r.path, connected_files, lod_full);
    let s = &ctx.settings;
    let inset_rect = screen_rect.shrink(s.file_rect_inset);
    dctx.painter.rect_filled(inset_rect, CornerRadius::ZERO, color);

    if lod_full {
        draw_file_borders(&dctx, screen_rect, inset_rect, r, ctx);

        if inset_rect.width() > dctx.cw * 2.0 && inset_rect.height() > dctx.fs + dctx.py * 2.0 {
            draw_file_text(dctx, inset_rect, r, ctx, color);
        }
    }
}

/// Draw border, hover highlight, and selected highlight for a file rect.
fn draw_file_borders(
    dctx: &DrawCtx,
    screen_rect: egui::Rect,
    inset_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
) {
    // Git status border: muted colored border for changed files
    let border_color = ctx.file_index.get(r.path.as_str())
        .filter(|entry| !entry.gs.is_empty())
        .map(|entry| super::colors::git_color(&entry.gs))
        .unwrap_or(dctx.tc.file_border);
    dctx.painter.rect_stroke(
        inset_rect,
        CornerRadius::ZERO,
        Stroke::new(if border_color != dctx.tc.file_border { 1.5 } else { 1.0 }, border_color),
        StrokeKind::Middle,
    );

    if ctx.hovered_path == Some(r.path.as_str()) {
        dctx.painter.rect_stroke(
            screen_rect, CornerRadius::ZERO,
            Stroke::new(1.0, dctx.tc.hover_stroke),
            StrokeKind::Outside,
        );
    }

    if ctx.selected_path == Some(r.path.as_str()) {
        dctx.painter.rect_stroke(
            screen_rect, CornerRadius::ZERO,
            Stroke::new(2.0, dctx.tc.selected_stroke),
            StrokeKind::Outside,
        );
    }
}

/// Draw file name and stats line text inside a file rect.
/// Uses adaptive text color based on background luminance for readability.
fn draw_file_text(
    dctx: &DrawCtx,
    inset_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
    bg_color: Color32,
) {
    let name = r.path.rsplit('/').next().unwrap_or(&r.path);
    let display_name = truncate_to_fit(name, inset_rect.width(), dctx.cw, dctx.px, 2);

    if display_name.is_empty() {
        return;
    }

    let label_color = contrast_text_color(bg_color, dctx.tc.file_label, Color32::from_rgb(20, 20, 25));
    let stats_color = contrast_secondary_color(bg_color);

    let text_x = inset_rect.left() + dctx.px;
    let text_y = inset_rect.top() + dctx.py;
    let name_bottom = dctx.painter.text(
        egui::pos2(text_x, text_y),
        egui::Align2::LEFT_TOP,
        display_name,
        egui::FontId::monospace(dctx.fs),
        label_color,
    ).max.y;

    draw_stats_line(dctx, inset_rect, r, ctx, text_x, name_bottom, stats_color);
}

/// Truncate a string to fit within `width` given padding and char width.
/// Returns empty str if fewer than `min_chars` fit.
fn truncate_to_fit(s: &str, width: f32, cw: f32, px: f32, min_chars: usize) -> &str {
    let max_chars = ((width - px * 2.0) / cw).max(0.0) as usize;
    if max_chars < min_chars {
        ""
    } else if s.chars().count() > max_chars {
        &s[..s.floor_char_boundary(max_chars)]
    } else {
        s
    }
}

/// Draw the stats line below the file name if there is room.
fn draw_stats_line(
    dctx: &DrawCtx,
    inset_rect: egui::Rect,
    r: &LayoutRectSlim,
    ctx: &RenderContext,
    text_x: f32,
    name_bottom: f32,
    stats_color: Color32,
) {
    let gap = dctx.fs * 0.1;
    if name_bottom + gap + dctx.fs >= inset_rect.bottom() - dctx.py {
        return;
    }
    if let Some(entry) = ctx.file_index.get(r.path.as_str()) {
        let sl = &entry.stats_line;
        let stat_display = truncate_to_fit(sl.as_str(), inset_rect.width(), dctx.cw, dctx.px, 0);
        dctx.painter.text(
            egui::pos2(text_x, name_bottom + gap),
            egui::Align2::LEFT_TOP,
            stat_display,
            egui::FontId::monospace(dctx.fs),
            stats_color,
        );
    }
}

/// Compute file color based on current color mode. Used by both main canvas and minimap.
pub fn file_color(ctx: &RenderContext, path: &str) -> Color32 {
    match ctx.color_mode {
        ColorMode::Monochrome => ctx.theme_config.file_surface,
        ColorMode::Language => color_by_language(ctx, path),
        ColorMode::Heat => color_by_heat(ctx, path),
        ColorMode::Age => color_by_age(ctx, path),
        ColorMode::Churn => color_by_churn(ctx, path),
        ColorMode::Risk => color_by_risk(ctx, path),
        ColorMode::Git => color_by_git(ctx, path),
        ColorMode::ExecDepth => color_by_exec_depth(ctx, path),
        ColorMode::BlastRadius => color_by_blast_radius(ctx, path),
    }
}

fn color_by_language(ctx: &RenderContext, path: &str) -> Color32 {
    let lang = ctx
        .file_index
        .get(path)
        .map(|e| e.lang.as_str())
        .unwrap_or("unknown");
    colors::language_color(lang)
}

fn color_by_heat(ctx: &RenderContext, path: &str) -> Color32 {
    let h = ctx.heat.get_heat(path, ctx.frame_instant, ctx.settings.heat_half_life);
    if h > 0.01 {
        heat::heat_color(h)
    } else {
        Color32::from_rgb(50, 50, 55)
    }
}

/// Age mode: color by file mtime — newer = brighter. O(1) via file_index.
fn color_by_age(ctx: &RenderContext, path: &str) -> Color32 {
    let mtime = ctx.file_index.get(path).map(|e| e.mtime).filter(|&m| m > 0.0);
    match mtime {
        Some(mt) => {
            let now = ctx.frame_now_secs;
            let age_days = ((now - mt) / 86400.0).max(0.0);
            let t = (age_days / 365.0).min(1.0) as f32; // 0=new, 1=1yr+
            let r = (100.0 + (1.0 - t) * 155.0) as u8;
            let g = (180.0 * (1.0 - t)) as u8;
            let b = (60.0 + t * 140.0) as u8;
            Color32::from_rgb(r, g, b)
        }
        None => Color32::from_rgb(70, 70, 70),
    }
}

/// Churn: estimate from git status + function count.
fn color_by_churn(ctx: &RenderContext, path: &str) -> Color32 {
    let entry = ctx.file_index.get(path);
    let gs = entry.map(|e| e.gs.as_str()).unwrap_or("");
    let funcs = entry.map(|e| e.funcs).unwrap_or(0);
    let churn = match gs {
        "M" | "MM" => 0.7 + (funcs as f64 * 0.01).min(0.3),
        "A" => 0.5,
        _ => (funcs as f64 * 0.005).min(0.4),
    };
    let t = churn.min(1.0) as f32;
    let r = (80.0 + t * 175.0) as u8;
    let g = (180.0 - t * 120.0) as u8;
    let b = (80.0 - t * 40.0) as u8;
    Color32::from_rgb(r, g, b)
}

/// Risk: high complexity + modified = high risk.
fn color_by_risk(ctx: &RenderContext, path: &str) -> Color32 {
    let entry = ctx.file_index.get(path);
    let funcs = entry.map(|e| e.funcs).unwrap_or(0);
    let lines = entry.map(|e| e.lines).unwrap_or(0);
    let gs = entry.map(|e| e.gs.as_str()).unwrap_or("");
    let complexity = (funcs as f64 * 0.1 + lines as f64 * 0.001).min(1.0);
    let modified = if matches!(gs, "M" | "MM") { 0.5 } else { 0.0 };
    let risk = (complexity + modified).min(1.0);
    let t = risk as f32;
    let r = (60.0 + t * 195.0) as u8;
    let g = (200.0 - t * 160.0) as u8;
    let b = (60.0 - t * 30.0) as u8;
    Color32::from_rgb(r, g, b)
}

fn color_by_git(ctx: &RenderContext, path: &str) -> Color32 {
    let gs = ctx
        .file_index
        .get(path)
        .map(|e| e.gs.as_str())
        .unwrap_or("");
    colors::git_color(gs)
}

fn color_by_exec_depth(ctx: &RenderContext, path: &str) -> Color32 {
    let depth = ctx
        .snapshot
        .as_ref()
        .and_then(|s| s.exec_depth.get(path))
        .copied()
        .unwrap_or(u32::MAX);
    if depth == u32::MAX {
        Color32::from_rgb(50, 50, 50)
    } else {
        colors::exec_depth_color(depth)
    }
}

fn color_by_blast_radius(ctx: &RenderContext, path: &str) -> Color32 {
    let (radius, max_radius) = ctx
        .arch_report
        .as_ref()
        .map(|a| {
            let r = a.blast_radius.get(path).copied().unwrap_or(0);
            (r, a.max_blast_radius)
        })
        .unwrap_or((0, 0));
    colors::blast_radius_color(radius, max_radius)
}
