// Centralised design system: colors, spacing, and the egui style installer.
//
// Everything visual goes through these tokens — no widget should reach for an
// ad-hoc color or pixel value. Tweaking the look of the whole app is a single
// edit here.

use egui::{FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};

/// Color palette. Background is dark grey, foreground white, with a tightly
/// scoped accent set (yellow / red / purple) used sparingly for emphasis.
pub mod color {
    use egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
    pub const BG_PANEL: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
    pub const BG_HOVER: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(0x40, 0x40, 0x40);

    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF2);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x55, 0x55, 0x55);

    pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(0xE5, 0xC0, 0x7B);
    pub const ACCENT_RED: Color32 = Color32::from_rgb(0xE0, 0x6C, 0x6C);
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(0xB7, 0x8F, 0xD4);
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(0x9C, 0xC8, 0x8E);

    pub const GRID_LINE: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x2A);
    pub const WIRE: Color32 = Color32::from_rgb(0x6F, 0x6F, 0x6F);
}

/// Spacing scale — every gap should snap to one of these values.
#[allow(dead_code)]
pub mod space {
    pub const XS: f32 = 2.0;
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 8.0;
    pub const LG: f32 = 12.0;
    pub const XL: f32 = 16.0;
}

/// Apply the qsim look to an egui context. Called once during app init.
pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut v = Visuals::dark();

    // Surfaces
    v.panel_fill = color::BG;
    v.window_fill = color::BG_PANEL;
    v.window_stroke = Stroke::NONE;
    // Text fields / editors that still sample `extreme_bg` should stay on the
    // same surface as the main canvas — not a separate light "sheet".
    v.extreme_bg_color = color::BG;
    v.faint_bg_color = color::BG_ELEVATED;

    // Widget palette — keep borders almost invisible; rely on fills.
    v.widgets.noninteractive.bg_fill = color::BG_PANEL;
    v.widgets.noninteractive.weak_bg_fill = color::BG_PANEL;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, color::TEXT_PRIMARY);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, color::GRID_LINE);

    v.widgets.inactive.bg_fill = color::BG_ELEVATED;
    v.widgets.inactive.weak_bg_fill = color::BG_ELEVATED;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, color::TEXT_PRIMARY);
    v.widgets.inactive.bg_stroke = Stroke::NONE;

    v.widgets.hovered.bg_fill = color::BG_HOVER;
    v.widgets.hovered.weak_bg_fill = color::BG_HOVER;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, color::ACCENT_YELLOW);
    v.widgets.hovered.bg_stroke = Stroke::NONE;

    v.widgets.active.bg_fill = color::BG_ACTIVE;
    v.widgets.active.weak_bg_fill = color::BG_ACTIVE;
    v.widgets.active.fg_stroke = Stroke::new(1.0, color::ACCENT_YELLOW);
    v.widgets.active.bg_stroke = Stroke::NONE;

    v.widgets.open.bg_fill = color::BG_ACTIVE;
    v.widgets.open.fg_stroke = Stroke::new(1.0, color::TEXT_PRIMARY);

    v.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(0xE5, 0xC0, 0x7B, 40);
    v.selection.stroke = Stroke::new(1.0, color::ACCENT_YELLOW);

    v.hyperlink_color = color::ACCENT_PURPLE;

    style.visuals = v;

    // Type scale — small, dense, dev-tool feel.
    style.text_styles = [
        (TextStyle::Heading, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(space::MD, space::SM);
    style.spacing.button_padding = egui::vec2(space::MD, space::XS + 1.0);
    style.spacing.window_margin = Margin::same(space::LG as i8);
    style.spacing.menu_margin = Margin::same(space::SM as i8);

    ctx.set_style(style);
}
