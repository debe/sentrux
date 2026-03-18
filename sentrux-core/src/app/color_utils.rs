//! Color computation utilities — dynamic gradient and language profile colors.
//!
//! These functions compute colors from runtime data (scores, language profiles)
//! using `Color32::from_rgb` with computed values. Kept separate from panel
//! code so panels reference only semantic ThemeConfig colors.

/// Convert a language profile's color_rgb to an egui Color32.
pub(crate) fn lang_profile_color(profile: &crate::analysis::plugin::profile::LanguageProfile) -> egui::Color32 {
    egui::Color32::from_rgb(profile.color_rgb[0], profile.color_rgb[1], profile.color_rgb[2])
}

/// Continuous color from score in [0, 1]. No grade boundaries.
/// 0.0 = red, 0.5 = yellow, 1.0 = green. Smooth gradient.
/// WCAG-aware: produces darker colors when `dark_bg` is false (light theme)
/// to maintain >=4.5:1 contrast ratio against the background.
/// Default score color -- assumes dark background (most themes).
/// For light themes, use `score_color_for_theme(score, tc)`.
pub(crate) fn score_color(score: f64) -> egui::Color32 {
    score_color_themed(score, true)
}

/// Theme-aware score color -- picks dark or light palette based on theme.
pub(crate) fn score_color_for_theme(score: f64, tc: &crate::core::settings::ThemeConfig) -> egui::Color32 {
    score_color_themed(score, tc.section_is_dark)
}

/// Theme-aware score color. `dark_bg = true` for dark themes, `false` for light.
pub(crate) fn score_color_themed(score: f64, dark_bg: bool) -> egui::Color32 {
    let s = score.clamp(0.0, 1.0) as f32;
    if dark_bg {
        // Bright colors on dark background (high contrast)
        if s < 0.5 {
            let t = s * 2.0;
            egui::Color32::from_rgb(
                200,
                (80.0 + t * 120.0) as u8,
                (80.0 - t * 20.0) as u8,
            )
        } else {
            let t = (s - 0.5) * 2.0;
            egui::Color32::from_rgb(
                (200.0 - t * 100.0) as u8,
                200,
                (60.0 + t * 40.0) as u8,
            )
        }
    } else {
        // Dark colors on light background (WCAG >=4.5:1)
        if s < 0.5 {
            let t = s * 2.0;
            egui::Color32::from_rgb(
                180,
                (30.0 + t * 80.0) as u8,  // 30 -> 110
                (30.0 - t * 10.0) as u8,  // 30 -> 20
            )
        } else {
            let t = (s - 0.5) * 2.0;
            egui::Color32::from_rgb(
                (140.0 - t * 80.0) as u8,  // 140 -> 60
                (110.0 + t * 20.0) as u8,  // 110 -> 130
                (20.0 + t * 20.0) as u8,   // 20 -> 40
            )
        }
    }
}
