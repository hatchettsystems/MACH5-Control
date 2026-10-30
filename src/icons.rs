//! Tiny line icons for the sidebar, drawn with the painter so they scale and recolour freely.
use crate::data::Page;
use eframe::egui::{vec2, Color32, Painter, Pos2, Rect, Stroke, StrokeKind};

pub fn draw(p: &Painter, page: Page, c: Pos2, col: Color32) {
    let s = Stroke::new(1.4_f32, col);
    let ln = |a: Pos2, b: Pos2| p.line_segment([a, b], s);
    let sq = |center: Pos2, half: f32| p.rect_stroke(Rect::from_center_size(center, vec2(half * 2.0, half * 2.0)), 1.5, s, StrokeKind::Middle);
    let o = |dx: f32, dy: f32| c + vec2(dx, dy);
    match page {
        Page::Dashboard => {
            for (dx, dy) in [(-3.8, -3.8), (3.8, -3.8), (-3.8, 3.8), (3.8, 3.8)] {
                sq(o(dx, dy), 2.6);
            }
        }
        Page::Print => {
            ln(o(-4.0, -5.0), o(4.0, -5.0));
            ln(o(-5.5, -1.0), o(5.5, -1.0));
            ln(o(-7.0, 3.0), o(7.0, 3.0));
            ln(o(-7.0, 7.0), o(7.0, 7.0));
        }
        Page::Thermal => {
            p.rect_stroke(Rect::from_center_size(o(0.0, -2.5), vec2(4.0, 9.0)), 2.0, s, StrokeKind::Middle);
            p.circle_stroke(o(0.0, 4.5), 3.2, s);
            ln(o(0.0, -1.0), o(0.0, 4.0));
        }
        Page::Motion => {
            ln(o(-7.0, 0.0), o(7.0, 0.0));
            ln(o(7.0, 0.0), o(3.5, -3.5));
            ln(o(7.0, 0.0), o(3.5, 3.5));
            ln(o(-7.0, -5.5), o(1.0, -5.5));
            ln(o(-7.0, 5.5), o(1.0, 5.5));
        }
        Page::Calibration => {
            p.circle_stroke(c, 6.5, s);
            p.circle_stroke(c, 3.0, s);
            p.circle_filled(c, 1.0, col);
        }
        Page::Probe => {
            ln(o(0.0, -7.0), o(0.0, 2.0));
            p.circle_stroke(o(0.0, 4.5), 2.5, s);
            ln(o(-6.0, 7.5), o(6.0, 7.5));
        }
        Page::Drives => {
            p.circle_stroke(c, 3.0, s);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                ln(c + vec2(a.cos(), a.sin()) * 5.0, c + vec2(a.cos(), a.sin()) * 7.0);
            }
        }
        Page::Materials => {
            p.circle_stroke(c, 7.0, s);
            p.circle_stroke(c, 2.2, s);
            ln(o(-7.0, 0.0), o(-2.2, 0.0));
        }
        Page::Spools => {
            for (dx, dy) in [(-4.5, -4.5), (4.5, -4.5), (-4.5, 4.5)] {
                sq(o(dx, dy), 2.4);
            }
            for (dx, dy) in [(2.5, 2.5), (6.0, 2.5), (2.5, 6.0), (6.0, 6.0), (4.2, 4.2)] {
                p.circle_filled(o(dx, dy), 0.9, col);
            }
        }
        Page::Camera => {
            p.rect_stroke(Rect::from_center_size(c, vec2(15.0, 10.5)), 2.0, s, StrokeKind::Middle);
            p.circle_stroke(c, 3.0, s);
        }
        Page::CanBus => {
            ln(o(-7.0, -3.0), o(7.0, -3.0));
            ln(o(-7.0, 3.0), o(7.0, 3.0));
            for dx in [-4.5, 0.0, 4.5] {
                ln(o(dx, -3.0), o(dx, -7.0));
                ln(o(dx, 3.0), o(dx, 7.0));
            }
        }
        Page::Nodes => {
            let pts = [o(0.0, -5.5), o(-6.0, 4.5), o(6.0, 4.5)];
            ln(pts[0], pts[1]);
            ln(pts[1], pts[2]);
            ln(pts[2], pts[0]);
            for q in pts {
                p.circle_filled(q, 2.0, col);
            }
        }
        Page::Firmware => {
            sq(c, 4.0);
            for k in [-2.0, 2.0] {
                ln(o(k, -4.0), o(k, -7.0));
                ln(o(k, 4.0), o(k, 7.0));
                ln(o(-4.0, k), o(-7.0, k));
                ln(o(4.0, k), o(7.0, k));
            }
        }
        Page::Fleet => {
            for (dx, dy) in [(-5.0, -3.5), (0.0, -3.5), (5.0, -3.5), (-5.0, 3.5), (0.0, 3.5), (5.0, 3.5)] {
                sq(o(dx, dy), 1.7);
            }
        }
        Page::Builder => {
            p.rect_stroke(Rect::from_center_size(o(-2.0, 4.5), vec2(10.0, 5.0)), 1.5, s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_center_size(o(2.0, -1.0), vec2(10.0, 5.0)), 1.5, s, StrokeKind::Middle);
            p.rect_stroke(Rect::from_center_size(o(-2.0, -6.0), vec2(10.0, 4.0)), 1.5, s, StrokeKind::Middle);
        }
        Page::Config => {
            ln(o(-7.0, -4.0), o(7.0, -4.0));
            ln(o(-7.0, 4.0), o(7.0, 4.0));
            p.circle_filled(o(-2.5, -4.0), 2.2, col);
            p.circle_filled(o(3.0, 4.0), 2.2, col);
        }
        Page::Rules => {
            ln(o(-6.0, 0.0), o(-1.0, 0.0));
            ln(o(-1.0, 0.0), o(3.0, -5.0));
            ln(o(-1.0, 0.0), o(3.0, 5.0));
            p.circle_filled(o(5.0, -5.0), 1.8, col);
            p.circle_filled(o(5.0, 5.0), 1.8, col);
        }
        Page::Api => {
            ln(o(-3.0, -5.0), o(-7.0, 0.0));
            ln(o(-7.0, 0.0), o(-3.0, 5.0));
            ln(o(3.0, -5.0), o(7.0, 0.0));
            ln(o(7.0, 0.0), o(3.0, 5.0));
            ln(o(1.5, -6.0), o(-1.5, 6.0));
        }
        Page::Simulator => {
            p.add(eframe::egui::Shape::closed_line(vec![o(-4.0, -6.0), o(6.0, 0.0), o(-4.0, 6.0)], s));
        }
        Page::Logs => {
            for (k, w) in [(-5.0, 9.0), (0.0, 12.0), (5.0, 7.0)] {
                p.circle_filled(o(-6.5, k), 1.0, col);
                ln(o(-3.5, k), o(-3.5 + w, k));
            }
        }
        Page::Spec => {
            p.rect_stroke(Rect::from_center_size(c, vec2(11.0, 14.0)), 1.5, s, StrokeKind::Middle);
            ln(o(-3.0, -3.0), o(3.0, -3.0));
            ln(o(-3.0, 0.5), o(3.0, 0.5));
            ln(o(-3.0, 4.0), o(1.0, 4.0));
        }
    }
}

/// Small line glyphs used in panel headers, tiles and dials. `rot` spins the fan.
pub fn glyph(p: &Painter, name: &str, c: Pos2, col: Color32, rot: f32) {
    use eframe::egui::{Rect as R, Shape};
    let s = Stroke::new(1.5_f32, col);
    let ln = |a: Pos2, b: Pos2| p.line_segment([a, b], s);
    let o = |dx: f32, dy: f32| c + vec2(dx, dy);
    match name {
        "thermo" => {
            p.rect_stroke(R::from_center_size(o(0.0, -2.5), vec2(5.0, 11.0)), 2.5, s, StrokeKind::Middle);
            p.circle_stroke(o(0.0, 5.5), 3.6, s);
            ln(o(0.0, -1.0), o(0.0, 4.5));
        }
        "bed" => {
            ln(o(-8.0, -5.0), o(8.0, -5.0));
            ln(o(-8.0, 0.0), o(8.0, 0.0));
            ln(o(-8.0, 5.0), o(8.0, 5.0));
            ln(o(-5.0, -8.0), o(-5.0, -2.0));
            ln(o(5.0, -8.0), o(5.0, -2.0));
        }
        "cube" => {
            p.rect_stroke(R::from_center_size(o(-1.5, 1.5), vec2(11.0, 11.0)), 1.5, s, StrokeKind::Middle);
            ln(o(-7.0, -4.0), o(-2.0, -8.0));
            ln(o(-2.0, -8.0), o(8.0, -8.0));
            ln(o(8.0, -8.0), o(8.0, 2.0));
            ln(o(4.0, -4.0), o(8.0, -8.0));
        }
        "fan" => {
            for k in 0..3 {
                let a = rot + k as f32 * std::f32::consts::TAU / 3.0;
                let d = vec2(a.cos(), a.sin());
                let n = vec2(-d.y, d.x);
                p.add(Shape::closed_line(vec![c + d * 2.0, c + d * 9.0 + n * 3.5, c + d * 8.0 - n * 2.0], s));
            }
            p.circle_filled(c, 1.6, col);
        }
        "bolt" => {
            p.add(Shape::line(vec![o(2.0, -9.0), o(-4.0, 1.0), o(0.0, 1.0), o(-2.0, 9.0), o(4.0, -1.5), o(0.0, -1.5)], s));
        }
        "spool" => {
            p.circle_stroke(c, 8.5, s);
            p.circle_stroke(c, 3.0, s);
            ln(o(-8.5, 0.0), o(-3.0, 0.0));
        }
        "motion" => {
            ln(o(-8.0, 0.0), o(8.0, 0.0));
            ln(o(8.0, 0.0), o(4.0, -4.0));
            ln(o(8.0, 0.0), o(4.0, 4.0));
            ln(o(0.0, -8.0), o(0.0, 8.0));
            ln(o(0.0, -8.0), o(-3.5, -4.5));
            ln(o(0.0, 8.0), o(3.5, 4.5));
        }
        "grid" => {
            p.rect_stroke(R::from_center_size(c, vec2(16.0, 16.0)), 2.0, s, StrokeKind::Middle);
            ln(o(-2.7, -8.0), o(-2.7, 8.0));
            ln(o(2.7, -8.0), o(2.7, 8.0));
            ln(o(-8.0, -2.7), o(8.0, -2.7));
            ln(o(-8.0, 2.7), o(8.0, 2.7));
        }
        "camera" => {
            p.rect_stroke(R::from_center_size(c, vec2(17.0, 12.0)), 2.5, s, StrokeKind::Middle);
            p.circle_stroke(c, 3.4, s);
        }
        "printer" => {
            p.rect_stroke(R::from_center_size(o(0.0, 1.0), vec2(16.0, 12.0)), 2.0, s, StrokeKind::Middle);
            ln(o(-8.0, -2.0), o(8.0, -2.0));
            ln(o(-4.0, 1.0), o(4.0, 1.0));
            ln(o(0.0, -8.0), o(0.0, -3.0));
        }
        "sliders" => {
            ln(o(-8.0, -4.5), o(8.0, -4.5));
            ln(o(-8.0, 4.5), o(8.0, 4.5));
            p.circle_filled(o(-3.0, -4.5), 2.6, col);
            p.circle_filled(o(3.5, 4.5), 2.6, col);
        }
        "terminal" => {
            ln(o(-7.0, -4.0), o(-2.0, 0.0));
            ln(o(-2.0, 0.0), o(-7.0, 4.0));
            ln(o(1.0, 5.0), o(8.0, 5.0));
        }
        "clock" => {
            p.circle_stroke(c, 8.5, s);
            ln(c, o(0.0, -5.0));
            ln(c, o(4.0, 2.0));
        }
        "nodes" => {
            let pts = [o(0.0, -6.5), o(-7.0, 5.5), o(7.0, 5.5)];
            ln(pts[0], pts[1]);
            ln(pts[1], pts[2]);
            ln(pts[2], pts[0]);
            for q in pts { p.circle_filled(q, 2.4, col); }
        }
        "list" => {
            for dy in [-5.5, 0.0, 5.5] {
                p.circle_filled(o(-7.0, dy), 1.3, col);
                ln(o(-3.5, dy), o(8.0, dy));
            }
        }
        "bulb" => {
            p.circle_stroke(o(0.0, -2.0), 6.0, s);
            ln(o(-3.0, 6.0), o(3.0, 6.0));
            ln(o(-2.0, 9.0), o(2.0, 9.0));
        }
        "drop" => {
            p.add(Shape::closed_line(vec![o(0.0, -9.0), o(6.0, 1.0), o(5.0, 5.0), o(0.0, 8.5), o(-5.0, 5.0), o(-6.0, 1.0)], s));
        }
        "gauge" => {
            p.circle_stroke(c, 8.5, s);
            ln(c, o(4.5, -4.5));
            p.circle_filled(c, 1.4, col);
        }
        "home" => {
            ln(o(-6.0, -1.0), o(0.0, -7.0));
            ln(o(0.0, -7.0), o(6.0, -1.0));
            p.rect_stroke(R::from_center_size(o(0.0, 2.5), vec2(8.0, 6.0)), 0.5, s, StrokeKind::Middle);
        }
        "gantry" => {
            ln(o(-8.0, -6.0), o(8.0, -6.0));
            ln(o(-8.0, -6.0), o(-8.0, 7.0));
            ln(o(8.0, -6.0), o(8.0, 7.0));
            p.rect_stroke(R::from_center_size(o(0.0, -6.0), vec2(6.0, 5.0)), 1.5, s, StrokeKind::Middle);
        }
        "hash" => {
            ln(o(-3.0, -8.0), o(-5.0, 8.0));
            ln(o(4.0, -8.0), o(2.0, 8.0));
            ln(o(-8.0, -3.0), o(8.0, -3.0));
            ln(o(-8.0, 3.0), o(8.0, 3.0));
        }
        _ => {
            p.circle_stroke(c, 6.0, s);
        }
    }
}

/// Which glyph a panel title gets in its header.
pub fn head_glyph(head: &str) -> Option<&'static str> {
    let h = head.to_lowercase();
    let pairs: [(&str, &str); 24] = [
        ("print status", "printer"), ("printer status", "printer"), ("job", "printer"), ("quick controls", "sliders"),
        ("temperature", "thermo"), ("thermal", "thermo"), ("heater", "thermo"), ("fans", "fan"), ("fan", "fan"),
        ("power", "bolt"), ("filament", "spool"), ("spool", "spool"), ("motion", "motion"), ("toolhead", "gantry"),
        ("mesh", "grid"), ("camera", "camera"), ("limits", "sliders"), ("console", "terminal"), ("history", "clock"),
        ("node", "nodes"), ("event", "list"), ("alerts", "list"), ("queue", "list"), ("machines", "nodes"),
    ];
    pairs.iter().find(|(k, _)| h.contains(k)).map(|(_, g)| *g)
}

/// Solid house: eaves, body and a door cut-out. `half` is half the icon width in px.
pub fn house(p: &Painter, c: Pos2, half: f32, fill: Color32, door: Color32) {
    use eframe::egui::Shape;
    let u = half / 11.0;
    let pt = |x: f32, y: f32| c + vec2(x * u, y * u);
    // roof with overhanging eaves
    p.add(Shape::convex_polygon(vec![pt(0.0, -10.0), pt(11.0, 0.2), pt(-11.0, 0.2)], fill, Stroke::NONE));
    // body, slightly narrower than the eaves
    p.add(Shape::convex_polygon(vec![pt(-7.0, -1.0), pt(7.0, -1.0), pt(7.0, 9.5), pt(-7.0, 9.5)], fill, Stroke::NONE));
    // door
    p.add(Shape::convex_polygon(vec![pt(-2.2, 3.6), pt(2.2, 3.6), pt(2.2, 9.5), pt(-2.2, 9.5)], door, Stroke::NONE));
}
