//! Monochrome palette. Colour is reserved for status indication only.
use eframe::egui::{self, Color32 as C, CornerRadius, Stroke};

pub const BG: C = C::from_rgb(8, 8, 10);
pub const PANEL: C = C::from_rgba_premultiplied(13, 13, 13, 236);
pub const RAD: f32 = 5.0;
pub const RAISED: C = C::from_rgb(27, 27, 27);
pub const LINE: C = C::from_rgb(44, 44, 44);
pub const LINE2: C = C::from_rgb(78, 78, 78);
pub const TEXT: C = C::from_rgb(225, 225, 225);
pub const DIM: C = C::from_rgb(156, 156, 156);
pub const FAINT: C = C::from_rgb(88, 88, 88);
/// Solid dark fills for tiles and row cards (never light grey).
pub const CARD: C = C::from_rgb(15, 16, 19);
pub const CARD_LINE: C = C::from_rgb(40, 42, 48);
pub const INSET: C = C::from_rgb(8, 9, 11);

pub const WHITE: C = C::from_rgb(250, 250, 250);

/// Tertiary accent, used sparingly: active nav icon, tab underline, live job progress,
/// primary plot series, sliders, the mesh gradient and the peak of the background wake.
/// A signal cyan-blue (the Creality/Elegoo-style blue) that cannot be confused with the
/// green / amber / red status colours.
pub const ACCENT: C = C::from_rgb(46, 196, 255);
pub const ACCENT_DIM: C = C::from_rgb(22, 88, 124);

pub const OK: C = C::from_rgb(86, 186, 108);
pub const WARN: C = C::from_rgb(224, 163, 48);
pub const ERR: C = C::from_rgb(228, 78, 78);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Ok,
    Warn,
    Err,
    Idle,
    Off,
}

impl Status {
    pub fn color(self) -> C {
        match self {
            Status::Ok => OK,
            Status::Warn => WARN,
            Status::Err => ERR,
            Status::Idle => DIM,
            Status::Off => FAINT,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Status::Ok => "OK",
            Status::Warn => "WARN",
            Status::Err => "FAULT",
            Status::Idle => "IDLE",
            Status::Off => "OFFLINE",
        }
    }
}

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    let r = CornerRadius::same(RAD as u8);
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.window_stroke = Stroke::new(1.0_f32, LINE2);
    v.window_corner_radius = r;
    v.menu_corner_radius = r;
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.extreme_bg_color = C::BLACK;
    v.faint_bg_color = C::from_rgb(20, 20, 20);
    v.code_bg_color = C::BLACK;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = WHITE;
    v.warn_fg_color = WARN;
    v.error_fg_color = ERR;
    v.selection.bg_fill = ACCENT_DIM;
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.slider_trailing_fill = true;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.inactive.bg_fill = C::from_rgb(22, 22, 26);
    w.inactive.weak_bg_fill = C::from_rgb(22, 22, 26);
    w.inactive.bg_stroke = Stroke::new(1.0_f32, C::from_rgb(86, 86, 96));
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.hovered.bg_fill = C::from_rgb(40, 40, 40);
    w.hovered.weak_bg_fill = C::from_rgb(40, 40, 40);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, C::from_rgb(140, 140, 140));
    w.hovered.fg_stroke = Stroke::new(1.0_f32, WHITE);
    w.active.bg_fill = C::from_rgb(60, 60, 60);
    w.active.weak_bg_fill = C::from_rgb(60, 60, 60);
    w.active.bg_stroke = Stroke::new(1.0_f32, WHITE);
    w.active.fg_stroke = Stroke::new(1.0_f32, WHITE);
    w.open.bg_fill = RAISED;
    w.open.weak_bg_fill = RAISED;
    for x in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        x.corner_radius = r;
    }

    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(16.0, 7.0);
        s.spacing.interact_size.y = 30.0;
        s.spacing.slider_width = 96.0;
        s.spacing.slider_rail_height = 3.0;
        s.spacing.scroll.bar_width = 6.0;
        s.visuals.striped = true;
        use egui::{FontFamily as F, FontId, TextStyle as T};
        s.text_styles.insert(T::Body, FontId::new(13.0, F::Proportional));
        s.text_styles.insert(T::Button, FontId::new(13.0, F::Proportional));
        s.text_styles.insert(T::Small, FontId::new(11.0, F::Proportional));
        s.text_styles.insert(T::Monospace, FontId::new(12.5, F::Monospace));
        s.text_styles.insert(T::Heading, FontId::new(20.0, F::Proportional));
    });
}
