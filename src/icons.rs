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
        Page::Control => {
            p.circle_stroke(c, 3.2, s);
            ln(o(-7.0, 0.0), o(-4.0, 0.0));
            ln(o(4.0, 0.0), o(7.0, 0.0));
            ln(o(0.0, -7.0), o(0.0, -4.0));
            ln(o(0.0, 4.0), o(0.0, 7.0));
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
