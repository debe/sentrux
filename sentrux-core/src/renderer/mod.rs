//! Rendering pipeline — draws the treemap/blueprint visualization.
//!
//! The renderer is purely functional: it takes a `RenderContext` (read-only
//! snapshot of app state) and paints onto an egui `Painter`. No mutation,
//! no side effects. Sub-modules handle specific visual layers.

pub mod badges;
pub mod colors;
pub mod edge_routing;
pub mod edges;
pub mod heat_overlay;
pub mod minimap;
pub mod rects;

// Re-exports for sub-modules — import from super:: instead of cross-module
pub(crate) use crate::core::snapshot::Snapshot;
pub(crate) use crate::core::types::FileIndexEntry;
pub(crate) use crate::layout::types::{
    ColorMode, EdgeFilter, EdgePath, LayoutRectSlim, RectKind, RenderData,
};
pub(crate) use crate::metrics::arch::ArchReport;
pub(crate) use crate::core::heat::{self, HeatTracker};
pub(crate) use crate::core::settings::{Settings, ThemeConfig};
pub(crate) use crate::layout::viewport::ViewportTransform;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Interface for rendering a visualization frame onto a painter.
/// Enables alternative rendering backends or testing without a real GPU.
pub trait Renderer {
    /// Draw a single frame of the visualization.
    fn render(&self, painter: &egui::Painter, clip_rect: egui::Rect, ctx: &RenderContext);
}

/// Everything the renderer needs to draw a frame.
/// Constructed by the app layer from AppState — renderer never imports AppState.
/// This breaks the renderer → app circular dependency.
pub struct RenderContext<'a> {
    /// Pre-computed layout rectangles and edge paths (None before first layout)
    pub render_data: Option<&'a RenderData>,
    /// Current viewport transform for world/screen coordinate conversion
    pub viewport: &'a ViewportTransform,
    /// Active theme colors
    pub theme_config: &'a ThemeConfig,
    /// User-tunable rendering parameters
    pub settings: &'a Settings,
    /// Per-file metadata for labels and color modes
    pub file_index: &'a HashMap<String, FileIndexEntry>,
    /// Active color mode (determines how file blocks are colored)
    pub color_mode: ColorMode,
    /// Currently selected file path (for spotlight edges)
    pub selected_path: Option<&'a str>,
    /// Currently hovered file path (for tooltip and edge preview)
    pub hovered_path: Option<&'a str>,
    /// Which edge types to display
    pub edge_filter: EdgeFilter,
    /// Whether to show all edges or only spotlight edges
    pub show_all_edges: bool,
    /// Full snapshot for entry-point and exec-depth lookups
    pub snapshot: Option<&'a Arc<Snapshot>>,
    /// Architecture report for blast-radius color mode
    pub arch_report: Option<&'a ArchReport>,
    /// Heat tracker for ripple animations and heat color mode
    pub heat: &'a HeatTracker,
    /// Monotonic timestamp for this frame (used for heat/ripple queries)
    pub frame_instant: Instant,
    /// Unix epoch seconds for this frame (used for age color mode)
    pub frame_now_secs: f64,
    /// Animation time in seconds since app start
    pub anim_time: f64,
    /// Whether the user is actively panning/zooming (reduced LOD)
    pub interacting: bool,
    /// Absolute path of the scan root (for status bar display)
    pub root_path: Option<&'a str>,
    /// Search query — matching files are highlighted, non-matching dimmed
    pub search_query: &'a str,
}

/// Orchestrate a single frame of rendering onto the canvas painter.
pub fn render_frame(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    ctx: &RenderContext,
) {
    let rd = match ctx.render_data {
        Some(rd) => rd,
        None => return,
    };

    let lod_full = !ctx.interacting;

    // Draw root rect first — outermost background
    rects::draw_rects(painter, clip_rect, rd, ctx, RectKind::Root, lod_full);

    // Draw section rects (directories) — they form the background
    rects::draw_rects(painter, clip_rect, rd, ctx, RectKind::Section, lod_full);

    // Draw file rects
    rects::draw_rects(painter, clip_rect, rd, ctx, RectKind::File, lod_full);

    // Draw heat overlays (ripples + activity trail glow)
    if lod_full && ctx.heat.is_active() {
        heat_overlay::draw_heat_overlays(painter, clip_rect, rd, ctx);
    }

    // Draw edges on top of blocks
    if lod_full && (ctx.show_all_edges || ctx.selected_path.is_some() || ctx.hovered_path.is_some()) {
        edges::draw_edges(painter, clip_rect, rd, ctx);
    }

    // Badges on top of everything
    if lod_full {
        badges::draw_badges(painter, clip_rect, rd, ctx);
    }

    // Minimap always visible — keeps orientation during pan/zoom
    minimap::draw_minimap(painter, clip_rect, rd, ctx);
}
