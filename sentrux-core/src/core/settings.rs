//! Centralized application settings — all tunable parameters in one place.
//!
//! Previously scattered across 15+ files as magic numbers. Each parameter
//! carries inline documentation explaining its purpose and typical range.
//! Cloned per layout/render request so worker threads read consistent values.

/// User-tunable parameters, grouped by category.
/// Previously hardcoded across 15+ files — now centralized and exposed in a settings panel.
#[derive(Clone)]
pub struct Settings {
    // ── Edge colors (RGB) ──
    /// Import edge color (blue by default — "uses")
    pub import_color: (u8, u8, u8),
    /// Call edge color (orange by default — "calls")
    pub call_color: (u8, u8, u8),
    /// Inherit edge color (green by default — "inherits")
    pub inherit_color: (u8, u8, u8),

    // ── Edge rendering ──
    /// Base opacity for edges with 1 connection
    pub edge_alpha_base: f64,
    /// Maximum opacity cap for edges with many connections
    pub edge_alpha_max: f64,
    /// Base line width for edges with 1 connection
    pub edge_line_w_base: f64,
    /// Maximum line width cap
    pub edge_line_w_max: f64,
    /// Dash length in pixels (for call/inherit edges)
    pub dash_len: f32,
    /// Gap between dashes in pixels
    pub dash_gap: f32,
    /// Dash animation speed in pixels per second
    pub dash_anim_speed: f32,

    // ── Edge routing ──
    /// Minimum world-space distance to draw an edge
    pub min_edge_len: f64,
    /// Lane separation width for parallel edges
    pub lane_width: f64,
    /// Padding from rect border for edge start/end points
    pub edge_border_pad: f64,
    /// Alignment threshold: if centers differ by less than this on X or Y,
    /// route as a straight line (0 bends). Previously hardcoded as 3.0.
    pub edge_align_threshold: f64,
    /// Lane offset threshold: below this, treat as zero lane offset.
    /// Previously hardcoded as 0.5.
    pub edge_lane_threshold: f64,

    // ── Layout: Treemap ──
    /// Padding inside directory sections (world units)
    pub treemap_dir_pad: f64,
    /// Header height for directory labels (world units)
    pub treemap_dir_header: f64,
    /// Minimum dimension for file rectangles (smaller = hidden)
    pub treemap_min_rect: f64,
    /// Gutter between top-level sibling sections (depth 0)
    pub treemap_gutter_root: f64,
    /// Gutter between sibling sections at depth >= 1
    pub treemap_gutter_inner: f64,

    // ── Layout: Blueprint ──
    /// Padding inside blueprint sections
    pub blueprint_section_pad: f64,
    /// Header height for blueprint section labels
    pub blueprint_section_header: f64,
    /// Minimum rect dimension in blueprint mode
    pub blueprint_min_rect: f64,
    /// Base gutter between sections (decreases with depth)
    pub blueprint_gutter_base: f64,
    /// Gutter for top-level sections
    pub blueprint_gutter_top: f64,
    /// Margin reserved for edge routing outside file blocks
    pub blueprint_route_margin: f64,

    // ── Font sizes ──
    /// Scale factor for zoom-proportional text (0.05 = tiny, 0.35 = large)
    pub font_scale: f32,
    /// UI scale factor for panel/toolbar text (1.0 = default 13px body, 1.5 = large)
    pub ui_scale: f32,

    // ── Viewport ──
    /// Minimum zoom level (prevents zooming out too far)
    pub zoom_min: f64,
    /// Maximum zoom level (prevents zooming in too far)
    pub zoom_max: f64,
    /// Zoom multiplier per scroll wheel tick
    pub zoom_scroll_factor: f64,
    /// Padding when fitting content to viewport (world units)
    pub fit_content_padding: f64,

    // ── Minimap ──
    /// Minimap width in screen pixels
    pub minimap_w: f32,
    /// Minimap height in screen pixels
    pub minimap_h: f32,
    /// Minimap padding from canvas edge in screen pixels
    pub minimap_pad: f32,

    // ── Animation / Heat ──
    /// Heat exponential decay half-life in seconds
    pub heat_half_life: f64,
    /// Duration of the ripple border animation in seconds
    pub ripple_duration: f64,
    /// Maximum age of trail entries before pruning (seconds)
    pub trail_max_age: f64,
    /// Radius of trail dots in screen pixels
    pub trail_dot_radius: f32,

    // ── Rect rendering ──
    /// Inset applied to file rects for visual separation (screen pixels)
    pub file_rect_inset: f32,

    // ── Graph analysis ──
    /// Maximum call targets per function (limits ambiguous resolution)
    pub max_call_targets: usize,

    // ── Squarify ──
    /// Minimum rect dimension for the squarify algorithm
    pub squarify_min_rect: f64,
    /// Minimum weight floor ensuring every file gets visible area
    pub min_child_weight: f64,

    // ── Scanner limits ──
    /// Maximum file size to include in scan (kilobytes)
    pub max_file_size_kb: u64,
    /// Maximum file size to attempt tree-sitter parsing (kilobytes)
    pub max_parse_size_kb: usize,

    // ── Chrome fractions ──
    /// Max fraction of treemap section area used for padding/header
    pub treemap_max_chrome_frac: f64,
    /// Max fraction of blueprint section area used for padding/header
    pub blueprint_max_chrome_frac: f64,

    // ── Timing / Debounce ──
    /// Debounce window for accumulating file changes before rescan (ms)
    pub file_change_debounce_ms: u64,
    /// Debounce window for the filesystem watcher (ms)
    pub watcher_debounce_ms: u64,
    /// Interval between heat animation repaints (ms)
    pub heat_repaint_ms: u64,

    // ── Font loading ──
    /// Whether to load CJK (Chinese/Japanese/Korean) fallback fonts at startup.
    /// Disable to save 10-30MB of memory when CJK text is not needed.
    pub load_cjk_fonts: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Distinct edge colors — must be visually separable
            import_color: (100, 160, 220),  // blue — "uses"
            call_color: (220, 160, 80),     // orange — "calls"
            inherit_color: (140, 210, 140), // green — "inherits"

            edge_alpha_base: 0.12,
            edge_alpha_max: 0.7,
            edge_line_w_base: 1.0,
            edge_line_w_max: 2.0,
            dash_len: 5.0,
            dash_gap: 4.0,
            dash_anim_speed: 25.0,

            min_edge_len: 3.0,
            lane_width: 4.0,
            edge_border_pad: 1.5,
            edge_align_threshold: 3.0,
            edge_lane_threshold: 0.5,

            treemap_dir_pad: 6.0,
            treemap_dir_header: 14.0,
            treemap_min_rect: 8.0,
            treemap_gutter_root: 6.0,
            treemap_gutter_inner: 2.0,

            blueprint_section_pad: 6.0,
            blueprint_section_header: 20.0,
            blueprint_min_rect: 4.0,
            blueprint_gutter_base: 4.0,
            blueprint_gutter_top: 14.0,
            blueprint_route_margin: 40.0,

            font_scale: 0.10,
            ui_scale: 1.0,

            zoom_min: 0.05,
            zoom_max: 50.0,
            zoom_scroll_factor: 1.1,
            fit_content_padding: 20.0,

            minimap_w: 160.0,
            minimap_h: 120.0,
            minimap_pad: 10.0,

            heat_half_life: 5.0,
            ripple_duration: 0.6,
            trail_max_age: 30.0,
            trail_dot_radius: 3.0,

            file_rect_inset: 2.0,   // gap between blocks so selection borders are visible

            max_call_targets: 5,

            squarify_min_rect: 8.0,
            min_child_weight: 4.0,

            max_file_size_kb: 2048,
            max_parse_size_kb: 512,

            treemap_max_chrome_frac: 0.25,
            blueprint_max_chrome_frac: 0.25,

            file_change_debounce_ms: 500,
            watcher_debounce_ms: 300,
            heat_repaint_ms: 50,

            load_cjk_fonts: true,
        }
    }
}

impl Settings {
    /// Reset all values to defaults
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Clamp values to valid ranges to prevent division-by-zero and logic errors. [M4 fix]
    pub fn sanitize(&mut self) {
        self.heat_half_life = self.heat_half_life.max(0.001);
        self.ripple_duration = self.ripple_duration.max(0.001);
        self.trail_max_age = self.trail_max_age.max(0.1);
        self.zoom_min = self.zoom_min.max(0.001);
        self.zoom_max = self.zoom_max.max(self.zoom_min + 0.01);
        self.edge_alpha_base = self.edge_alpha_base.clamp(0.0, 1.0);
        self.edge_alpha_max = self.edge_alpha_max.clamp(self.edge_alpha_base, 1.0);
        self.ui_scale = self.ui_scale.clamp(0.5, 3.0);
    }

    /// Create a HeatConfig from current settings
    pub fn heat_config(&self) -> crate::core::heat::HeatConfig {
        crate::core::heat::HeatConfig {
            half_life: self.heat_half_life,
            ripple_duration: self.ripple_duration,
            trail_max_age: self.trail_max_age,
        }
    }
}

// ── Visual theme system ─────────────────────────────────────────────────
//
// Color presets for the entire UI. Each `Theme` variant maps to a
// `ThemeConfig` containing all theme-dependent colors (canvas, sections,
// files, text, minimap, badges). Renderers read `ThemeConfig` fields
// instead of hard-coding colors.

use egui::Color32;

/// Visual theme preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Calm theme: muted blue-grey palette, low contrast
    Calm,
    /// Dark theme: neutral dark background, high-contrast text
    Dark,
    /// Light theme: bright background, dark text
    Light,
    /// Midnight theme: deep blue-black palette, cool accent colors
    Midnight,
    /// Solarized theme: Ethan Schoonover's solarized dark palette
    Solarized,
}

impl Theme {
    /// All available themes in display order.
    pub const ALL: &'static [Theme] = &[
        Theme::Calm,
        Theme::Dark,
        Theme::Light,
        Theme::Midnight,
        Theme::Solarized,
    ];

    /// Human-readable display label for this theme variant.
    pub fn label(self) -> &'static str {
        match self {
            Theme::Calm => "Calm",
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Midnight => "Midnight",
            Theme::Solarized => "Solarized",
        }
    }
}

/// All theme-dependent colors live here. Renderers call `ThemeConfig::from(theme)`
/// and read fields instead of hard-coding colors.
#[derive(Debug, Clone)]
pub struct ThemeConfig {
    /// Canvas background fill
    pub canvas_bg: Color32,
    /// Section (directory) base color and step per depth
    pub section_base: u8,
    /// Brightness step per depth level for nested sections
    pub section_step: u8,
    /// True if section colors lighten with depth (dark themes)
    pub section_is_dark: bool,
    /// Section border
    pub section_border: Color32,
    /// Section label text
    pub section_label: Color32,
    /// File label text
    pub file_label: Color32,
    /// Hover stroke
    pub hover_stroke: Color32,
    /// Selected stroke
    pub selected_stroke: Color32,
    /// Minimap background
    pub minimap_bg: Color32,
    /// Minimap border
    pub minimap_border: Color32,
    /// Minimap viewport indicator
    pub minimap_viewport: Color32,
    /// Toolbar/status bar text
    pub text_primary: Color32,
    /// Secondary toolbar/status bar text
    pub text_secondary: Color32,
    /// Header strip background (darkest layer, behind section labels)
    pub header_strip_bg: Color32,
    /// File block surface color
    pub file_surface: Color32,
    /// File block surface when spotlit
    pub file_surface_spotlit: Color32,
    /// File block border
    pub file_border: Color32,
    /// Entry-point badge colors
    pub badge_high: Color32,
    /// Entry-point badge color for low-confidence detections
    pub badge_low: Color32,

    // ── Semantic colors (WCAG AA ≥4.5:1 against canvas_bg) ──
    /// Success/pass/improved state (green)
    pub status_success: Color32,
    /// Error/fail/violation state (red)
    pub status_error: Color32,
    /// Warning/caution/degraded state (amber/yellow)
    pub status_warning: Color32,
    /// Inversion/architectural issue (orange)
    pub status_inversion: Color32,
    /// Informational/neutral accent (blue)
    pub accent_info: Color32,
    /// Unstable files indicator (purple/magenta)
    pub accent_unstable: Color32,
    /// Hotspot indicator (warm orange)
    pub accent_hotspot: Color32,
    /// Coupling indicator (cool blue)
    pub accent_coupling: Color32,
    /// Bus factor indicator (purple)
    pub accent_bus_factor: Color32,
    /// Plugin error indicator (dark red/orange)
    pub accent_plugin_error: Color32,
    /// High complexity indicator (orange)
    pub accent_high_complexity: Color32,
    /// Diff: added/new (green)
    pub diff_added: Color32,
    /// Diff: removed/deleted (red)
    pub diff_removed: Color32,
    /// Diff: modified (blue)
    pub diff_modified: Color32,
    /// Overflow/muted text ("+N more", inactive)
    pub text_muted: Color32,
    /// Toggle button: inactive state
    pub toggle_inactive: Color32,
    /// Toggle button: edge filter active
    pub toggle_edge: Color32,
    /// Toggle button: DSM active
    pub toggle_dsm: Color32,
}

impl ThemeConfig {
    /// Construct a ThemeConfig from a Theme preset.
    pub fn from_theme(theme: Theme) -> Self {
        match theme {
            Theme::Calm => theme_calm(),
            Theme::Dark => theme_dark(),
            Theme::Light => theme_light(),
            Theme::Midnight => theme_midnight(),
            Theme::Solarized => theme_solarized(),
        }
    }

    /// Section background color at a given depth, using this theme's palette.
    pub fn section_color(&self, depth: u32) -> Color32 {
        let base = self.section_base as u32;
        let step = self.section_step as u32;
        if step == 0 {
            let b = base as u8;
            Color32::from_rgb(b, b, (b as u32 + 10).min(255) as u8)
        } else if self.section_is_dark {
            let v = (base + step * depth).min(255) as u8;
            Color32::from_rgb(v, v, (v as u32 + 5).min(255) as u8)
        } else {
            let v = base.saturating_sub(step * depth).clamp(80, 255) as u8;
            Color32::from_rgb(v, v, v)
        }
    }
}

// ── Theme palette constructors ──

fn theme_calm() -> ThemeConfig {
    ThemeConfig {
        canvas_bg: Color32::from_rgb(22, 22, 30),
        section_base: 30,
        section_step: 0,
        section_is_dark: true,
        section_border: Color32::from_rgb(44, 46, 58),
        section_label: Color32::from_rgb(130, 135, 160),
        file_label: Color32::from_rgb(192, 200, 220),
        hover_stroke: Color32::from_rgb(140, 160, 210),
        selected_stroke: Color32::from_rgb(120, 180, 240),
        minimap_bg: Color32::from_rgb(14, 14, 20),
        minimap_border: Color32::from_rgb(44, 46, 58),
        minimap_viewport: Color32::from_rgb(120, 180, 240),
        text_primary: Color32::from_rgb(192, 200, 220),
        text_secondary: Color32::from_rgb(130, 135, 160),
        header_strip_bg: Color32::from_rgb(16, 16, 22),
        file_surface: Color32::from_rgb(40, 42, 54),
        file_surface_spotlit: Color32::from_rgb(50, 54, 70),
        file_border: Color32::from_rgb(50, 52, 66),
        badge_high: Color32::from_rgb(200, 180, 60),
        badge_low: Color32::from_rgb(140, 140, 70),
        status_success: Color32::from_rgb(100, 200, 100),
        status_error: Color32::from_rgb(220, 100, 100),
        status_warning: Color32::from_rgb(200, 170, 80),
        status_inversion: Color32::from_rgb(200, 140, 80),
        accent_info: Color32::from_rgb(103, 150, 230),
        accent_unstable: Color32::from_rgb(180, 140, 200),
        accent_hotspot: Color32::from_rgb(200, 140, 80),
        accent_coupling: Color32::from_rgb(140, 180, 200),
        accent_bus_factor: Color32::from_rgb(200, 160, 200),
        accent_plugin_error: Color32::from_rgb(210, 110, 90),
        accent_high_complexity: Color32::from_rgb(203, 120, 60),
        diff_added: Color32::from_rgb(115, 201, 145),
        diff_removed: Color32::from_rgb(224, 108, 117),
        diff_modified: Color32::from_rgb(103, 150, 230),
        text_muted: Color32::from_rgb(140, 140, 140),
        toggle_inactive: Color32::from_rgb(120, 120, 120),
        toggle_edge: Color32::from_rgb(220, 180, 80),
        toggle_dsm: Color32::from_rgb(100, 200, 180),
    }
}

fn theme_dark() -> ThemeConfig {
    ThemeConfig {
        canvas_bg: Color32::from_rgb(22, 22, 26),
        section_base: 30,
        section_step: 6,
        section_is_dark: true,
        section_border: Color32::from_rgb(45, 45, 50),
        section_label: Color32::from_rgb(160, 160, 165),
        file_label: Color32::from_rgb(230, 230, 235),
        hover_stroke: Color32::from_rgb(255, 255, 255),
        selected_stroke: Color32::from_rgb(100, 200, 255),
        minimap_bg: Color32::from_rgb(18, 18, 22),
        minimap_border: Color32::from_rgb(60, 60, 70),
        minimap_viewport: Color32::from_rgb(100, 200, 255),
        text_primary: Color32::from_rgb(220, 220, 220),
        text_secondary: Color32::from_rgb(140, 140, 150),
        header_strip_bg: Color32::from_rgb(16, 16, 20),
        file_surface: Color32::from_rgb(42, 42, 48),
        file_surface_spotlit: Color32::from_rgb(55, 55, 65),
        file_border: Color32::from_rgb(55, 55, 62),
        badge_high: Color32::from_rgb(200, 180, 60),
        badge_low: Color32::from_rgb(140, 140, 70),
        status_success: Color32::from_rgb(100, 200, 100),
        status_error: Color32::from_rgb(220, 100, 100),
        status_warning: Color32::from_rgb(200, 170, 80),
        status_inversion: Color32::from_rgb(200, 140, 80),
        accent_info: Color32::from_rgb(103, 150, 230),
        accent_unstable: Color32::from_rgb(180, 140, 200),
        accent_hotspot: Color32::from_rgb(200, 140, 80),
        accent_coupling: Color32::from_rgb(140, 180, 200),
        accent_bus_factor: Color32::from_rgb(200, 160, 200),
        accent_plugin_error: Color32::from_rgb(210, 110, 90),
        accent_high_complexity: Color32::from_rgb(203, 120, 60),
        diff_added: Color32::from_rgb(115, 201, 145),
        diff_removed: Color32::from_rgb(224, 108, 117),
        diff_modified: Color32::from_rgb(103, 150, 230),
        text_muted: Color32::from_rgb(140, 140, 140),
        toggle_inactive: Color32::from_rgb(120, 120, 120),
        toggle_edge: Color32::from_rgb(220, 180, 80),
        toggle_dsm: Color32::from_rgb(100, 200, 180),
    }
}

fn theme_light() -> ThemeConfig {
    ThemeConfig {
        canvas_bg: Color32::from_rgb(240, 240, 244),
        section_base: 230,
        section_step: 8,
        section_is_dark: false,
        section_border: Color32::from_rgb(195, 195, 200),
        section_label: Color32::from_rgb(90, 90, 95),
        file_label: Color32::from_rgb(35, 35, 40),
        hover_stroke: Color32::from_rgb(40, 40, 40),
        selected_stroke: Color32::from_rgb(30, 120, 200),
        minimap_bg: Color32::from_rgb(235, 235, 240),
        minimap_border: Color32::from_rgb(180, 180, 190),
        minimap_viewport: Color32::from_rgb(30, 120, 200),
        text_primary: Color32::from_rgb(30, 30, 30),
        text_secondary: Color32::from_rgb(100, 100, 110),
        header_strip_bg: Color32::from_rgb(220, 220, 226),
        file_surface: Color32::from_rgb(250, 250, 252),
        file_surface_spotlit: Color32::from_rgb(235, 235, 240),
        file_border: Color32::from_rgb(200, 200, 210),
        badge_high: Color32::from_rgb(180, 160, 40),
        badge_low: Color32::from_rgb(120, 120, 50),
        status_success: Color32::from_rgb(30, 120, 50),
        status_error: Color32::from_rgb(180, 40, 40),
        status_warning: Color32::from_rgb(130, 95, 10),
        status_inversion: Color32::from_rgb(145, 80, 10),
        accent_info: Color32::from_rgb(40, 80, 170),
        accent_unstable: Color32::from_rgb(120, 60, 140),
        accent_hotspot: Color32::from_rgb(160, 90, 20),
        accent_coupling: Color32::from_rgb(40, 100, 140),
        accent_bus_factor: Color32::from_rgb(130, 60, 140),
        accent_plugin_error: Color32::from_rgb(160, 40, 30),
        accent_high_complexity: Color32::from_rgb(170, 70, 20),
        diff_added: Color32::from_rgb(30, 120, 50),
        diff_removed: Color32::from_rgb(180, 40, 40),
        diff_modified: Color32::from_rgb(40, 80, 170),
        text_muted: Color32::from_rgb(105, 105, 110),
        toggle_inactive: Color32::from_rgb(95, 95, 100),
        toggle_edge: Color32::from_rgb(130, 95, 10),
        toggle_dsm: Color32::from_rgb(20, 120, 100),
    }
}

fn theme_midnight() -> ThemeConfig {
    ThemeConfig {
        canvas_bg: Color32::from_rgb(10, 10, 18),
        section_base: 18,
        section_step: 5,
        section_is_dark: true,
        section_border: Color32::from_rgb(30, 30, 50),
        section_label: Color32::from_rgb(125, 135, 170),
        file_label: Color32::from_rgb(185, 195, 225),
        hover_stroke: Color32::from_rgb(180, 200, 255),
        selected_stroke: Color32::from_rgb(80, 160, 255),
        minimap_bg: Color32::from_rgb(8, 8, 16),
        minimap_border: Color32::from_rgb(40, 40, 70),
        minimap_viewport: Color32::from_rgb(80, 160, 255),
        text_primary: Color32::from_rgb(200, 210, 240),
        text_secondary: Color32::from_rgb(125, 135, 170),
        header_strip_bg: Color32::from_rgb(8, 8, 14),
        file_surface: Color32::from_rgb(28, 28, 40),
        file_surface_spotlit: Color32::from_rgb(38, 38, 55),
        file_border: Color32::from_rgb(40, 40, 60),
        badge_high: Color32::from_rgb(200, 180, 60),
        badge_low: Color32::from_rgb(140, 140, 70),
        status_success: Color32::from_rgb(100, 200, 100),
        status_error: Color32::from_rgb(220, 100, 100),
        status_warning: Color32::from_rgb(200, 170, 80),
        status_inversion: Color32::from_rgb(200, 140, 80),
        accent_info: Color32::from_rgb(103, 150, 230),
        accent_unstable: Color32::from_rgb(180, 140, 200),
        accent_hotspot: Color32::from_rgb(200, 140, 80),
        accent_coupling: Color32::from_rgb(140, 180, 200),
        accent_bus_factor: Color32::from_rgb(200, 160, 200),
        accent_plugin_error: Color32::from_rgb(210, 110, 90),
        accent_high_complexity: Color32::from_rgb(203, 120, 60),
        diff_added: Color32::from_rgb(115, 201, 145),
        diff_removed: Color32::from_rgb(224, 108, 117),
        diff_modified: Color32::from_rgb(103, 150, 230),
        text_muted: Color32::from_rgb(140, 140, 140),
        toggle_inactive: Color32::from_rgb(120, 120, 120),
        toggle_edge: Color32::from_rgb(220, 180, 80),
        toggle_dsm: Color32::from_rgb(100, 200, 180),
    }
}

fn theme_solarized() -> ThemeConfig {
    ThemeConfig {
        canvas_bg: Color32::from_rgb(0, 43, 54),
        section_base: 30,
        section_step: 5,
        section_is_dark: true,
        section_border: Color32::from_rgb(55, 75, 82),
        section_label: Color32::from_rgb(130, 145, 145),
        file_label: Color32::from_rgb(220, 215, 200),
        hover_stroke: Color32::from_rgb(253, 246, 227),
        selected_stroke: Color32::from_rgb(38, 139, 210),
        minimap_bg: Color32::from_rgb(0, 30, 38),
        minimap_border: Color32::from_rgb(88, 110, 117),
        minimap_viewport: Color32::from_rgb(38, 139, 210),
        text_primary: Color32::from_rgb(238, 232, 213),
        text_secondary: Color32::from_rgb(147, 161, 161),
        header_strip_bg: Color32::from_rgb(0, 30, 38),
        file_surface: Color32::from_rgb(7, 54, 66),
        file_surface_spotlit: Color32::from_rgb(15, 68, 82),
        file_border: Color32::from_rgb(42, 76, 84),
        badge_high: Color32::from_rgb(180, 160, 50),
        badge_low: Color32::from_rgb(130, 130, 60),
        status_success: Color32::from_rgb(133, 153, 0),
        status_error: Color32::from_rgb(240, 100, 97),
        status_warning: Color32::from_rgb(181, 137, 0),
        status_inversion: Color32::from_rgb(220, 110, 70),
        accent_info: Color32::from_rgb(80, 160, 225),
        accent_unstable: Color32::from_rgb(135, 140, 220),
        accent_hotspot: Color32::from_rgb(220, 110, 70),
        accent_coupling: Color32::from_rgb(80, 160, 225),
        accent_bus_factor: Color32::from_rgb(135, 140, 220),
        accent_plugin_error: Color32::from_rgb(240, 100, 97),
        accent_high_complexity: Color32::from_rgb(220, 110, 70),
        diff_added: Color32::from_rgb(133, 153, 0),
        diff_removed: Color32::from_rgb(240, 100, 97),
        diff_modified: Color32::from_rgb(80, 160, 225),
        text_muted: Color32::from_rgb(130, 150, 160),
        toggle_inactive: Color32::from_rgb(120, 140, 150),
        toggle_edge: Color32::from_rgb(181, 137, 0),
        toggle_dsm: Color32::from_rgb(42, 161, 152),
    }
}

#[cfg(test)]
mod wcag_tests {
    use super::*;

    fn srgb_to_linear(c: u8) -> f64 {
        let s = c as f64 / 255.0;
        if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    }

    fn luminance(color: Color32) -> f64 {
        let [r, g, b, _] = color.to_array();
        0.2126 * srgb_to_linear(r) + 0.7152 * srgb_to_linear(g) + 0.0722 * srgb_to_linear(b)
    }

    fn contrast_ratio(fg: Color32, bg: Color32) -> f64 {
        let l1 = luminance(fg);
        let l2 = luminance(bg);
        let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn all_semantic_text_colors_pass_wcag_aa() {
        for &theme in Theme::ALL {
            let tc = ThemeConfig::from_theme(theme);
            let text_colors: Vec<(&str, Color32)> = vec![
                ("status_success", tc.status_success),
                ("status_error", tc.status_error),
                ("status_warning", tc.status_warning),
                ("status_inversion", tc.status_inversion),
                ("accent_info", tc.accent_info),
                ("accent_unstable", tc.accent_unstable),
                ("accent_hotspot", tc.accent_hotspot),
                ("accent_coupling", tc.accent_coupling),
                ("accent_bus_factor", tc.accent_bus_factor),
                ("accent_plugin_error", tc.accent_plugin_error),
                ("accent_high_complexity", tc.accent_high_complexity),
                ("diff_added", tc.diff_added),
                ("diff_removed", tc.diff_removed),
                ("diff_modified", tc.diff_modified),
                ("text_muted", tc.text_muted),
                ("text_primary", tc.text_primary),
                ("text_secondary", tc.text_secondary),
                ("section_label", tc.section_label),
                ("file_label", tc.file_label),
            ];
            for (name, color) in &text_colors {
                let ratio = contrast_ratio(*color, tc.canvas_bg);
                assert!(
                    ratio >= 4.5,
                    "WCAG FAIL: {} theme '{}' has contrast {:.2}:1 (need >=4.5:1) -- {:?} on {:?}",
                    theme.label(), name, ratio, color, tc.canvas_bg
                );
            }
        }
    }
}
