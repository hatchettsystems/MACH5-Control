//! Small shared widgets. Flat, square, monochrome.
use crate::theme::*;
use eframe::egui::{self, vec2, Align2, Button, Color32, FontId, Frame, Margin, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui};

/// On-screen rectangles of named UI regions, so the tutorial can point at them.
pub fn set_anchor(ctx: &egui::Context, key: &str, r: Rect) {
    let n = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(("anchor", key.to_string())), (n, r)));
}
pub fn get_anchor(ctx: &egui::Context, key: &str) -> Option<Rect> {
    let n = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<(u64, Rect)>(egui::Id::new(("anchor", key.to_string()))))
        .filter(|(f, _)| n.saturating_sub(*f) <= 3)
        .map(|(_, r)| r)
}

pub fn mono(s: impl Into<String>) -> RichText {
    RichText::new(s.into()).monospace()
}
pub fn dim(s: impl Into<String>) -> RichText {
    RichText::new(s.into()).color(DIM).size(12.0)
}

pub fn title(ui: &mut Ui, t: &str, sub: &str) {
    ui.label(RichText::new(t).size(20.0).strong().color(WHITE));
    ui.label(dim(sub));
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
}

/// Flat bordered section with a small caps header.
pub fn panel(ui: &mut Ui, head: &str, f: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let resp = Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, LINE))
        .corner_radius(RAD as u8)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width((w - 22.0).max(10.0));
            if !head.is_empty() {
                ui.label(RichText::new(head.to_uppercase()).size(11.0).color(DIM).monospace());
                ui.add_space(4.0);
            }
            f(ui);
        });
    let r = resp.response.rect;
    set_anchor(ui.ctx(), head, r);
    let focus: String = ui.ctx().data(|d| d.get_temp(egui::Id::new("tour_focus"))).unwrap_or_default();
    if !focus.is_empty() && focus == head && !ui.clip_rect().contains_rect(r) {
        ui.scroll_to_rect(r, Some(egui::Align::Center));
    }
    ui.add_space(8.0);
}

pub fn tabs(ui: &mut Ui, cur: &mut usize, names: &[&str]) {
    let r = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, n) in names.iter().enumerate() {
            let sel = *cur == i;
            let b = Button::new(RichText::new(*n).color(if sel { BG } else { TEXT }))
                .fill(if sel { WHITE } else { Color32::TRANSPARENT })
                .stroke(Stroke::new(1.0_f32, if sel { WHITE } else { LINE }));
            if ui.add(b).clicked() {
                *cur = i;
            }
        }
    });
    set_anchor(ui.ctx(), "tabs", r.response.rect);
    ui.add_space(8.0);
}

pub fn primary(ui: &mut Ui, t: &str) -> bool {
    ui.add(Button::new(RichText::new(t).color(BG).strong()).fill(WHITE).stroke(Stroke::new(1.0_f32, WHITE))).clicked()
}
pub fn button(ui: &mut Ui, t: &str) -> bool {
    ui.button(t).clicked()
}
pub fn danger(ui: &mut Ui, t: &str) -> bool {
    let r = ui.add(Button::new(RichText::new(t).color(Color32::WHITE).strong()).fill(Color32::from_rgb(150, 30, 30)).stroke(Stroke::new(1.0_f32, ERR)));
    set_anchor(ui.ctx(), "estop", r.rect);
    r.clicked()
}

pub fn dot(ui: &mut Ui, s: Status) {
    let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
    ui.painter().rect_filled(r.translate(vec2(0.0, 1.0)), crate::theme::RAD, s.color());
}
pub fn status(ui: &mut Ui, s: Status, text: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        dot(ui, s);
        let col = if matches!(s, Status::Ok | Status::Idle | Status::Off) { TEXT } else { s.color() };
        ui.label(RichText::new(text).color(col));
    });
}

/// Label that wraps at a fixed width, for long cells inside grids.
pub fn wrap(ui: &mut Ui, w: f32, text: impl Into<egui::WidgetText>) {
    ui.scope(|ui| {
        ui.set_max_width(w);
        ui.label(text);
    });
}

pub fn th(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t.to_uppercase()).size(10.5).color(DIM).monospace());
}
pub fn kv(ui: &mut Ui, id: &str, rows: &[(&str, String)]) {
    egui::Grid::new(id).num_columns(2).spacing([18.0, 4.0]).show(ui, |ui| {
        for (k, v) in rows {
            ui.label(dim(*k));
            ui.label(mono(v.clone()));
            ui.end_row();
        }
    });
}

/// Horizontal bar. `st` colours it only when it signals a state.
pub fn bar(ui: &mut Ui, frac: f32, w: f32, st: Option<Status>) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 8.0), Sense::hover());
    ui.painter().rect_filled(r, crate::theme::RAD, Color32::BLACK);
    ui.painter().rect_stroke(r, crate::theme::RAD, Stroke::new(1.0_f32, LINE), StrokeKind::Inside);
    let f = frac.clamp(0.0, 1.0);
    let fr = Rect::from_min_size(r.min, vec2(r.width() * f, r.height()));
    ui.painter().rect_filled(fr.shrink(1.0), crate::theme::RAD, st.map(|s| s.color()).unwrap_or(Color32::from_gray(190)));
}
pub fn bar_full(ui: &mut Ui, frac: f32, st: Option<Status>) {
    let w = ui.available_width().max(40.0);
    bar(ui, frac, w, st);
}

pub struct Series {
    pub name: String,
    pub col: Color32,
    pub pts: Vec<[f32; 2]>,
    pub dashed: bool,
}
impl Series {
    pub fn new(name: &str, col: Color32, pts: Vec<[f32; 2]>) -> Self {
        Self { name: name.into(), col, pts, dashed: false }
    }
    pub fn dashed(mut self) -> Self {
        self.dashed = true;
        self
    }
    pub fn from_ring(name: &str, col: Color32, d: &std::collections::VecDeque<f32>) -> Self {
        Self::new(name, col, d.iter().enumerate().map(|(i, v)| [i as f32, *v]).collect())
    }
}

pub fn plot(ui: &mut Ui, h: f32, series: &[Series], x: (f32, f32), y: (f32, f32), unit: &str) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, crate::theme::RAD, Color32::BLACK);
    p.rect_stroke(r, crate::theme::RAD, Stroke::new(1.0_f32, LINE), StrokeKind::Inside);
    let inner = Rect::from_min_max(r.min + vec2(44.0, 8.0), r.max - vec2(8.0, 8.0));
    let fy = |v: f32| inner.bottom() - (v - y.0) / (y.1 - y.0).max(1e-6) * inner.height();
    let fx = |v: f32| inner.left() + (v - x.0) / (x.1 - x.0).max(1e-6) * inner.width();
    let span = y.1 - y.0;
    for i in 0..=4 {
        let v = y.0 + (y.1 - y.0) * i as f32 / 4.0;
        let yy = fy(v);
        p.line_segment([egui::pos2(inner.left(), yy), egui::pos2(inner.right(), yy)], Stroke::new(1.0_f32, Color32::from_gray(28)));
        p.text(egui::pos2(inner.left() - 4.0, yy), Align2::RIGHT_CENTER, if span < 0.5 { format!("{:.3}", v) } else if span < 5.0 { format!("{:.2}", v) } else if span < 50.0 { format!("{:.1}", v) } else { format!("{:.0}", v) }, FontId::monospace(10.0), DIM);
    }
    for s in series {
        let pts: Vec<_> = s.pts.iter().map(|q| egui::pos2(fx(q[0]), fy(q[1].clamp(y.0, y.1)))).collect();
        if pts.len() < 2 {
            continue;
        }
        if s.dashed {
            p.extend(Shape::dashed_line(&pts, Stroke::new(1.0_f32, s.col), 4.0, 3.0));
        } else {
            p.add(Shape::line(pts, Stroke::new(1.3_f32, s.col)));
        }
    }
    let mut lx = inner.left() + 6.0;
    for s in series {
        let g = p.text(egui::pos2(lx, inner.top() + 2.0), Align2::LEFT_TOP, &s.name, FontId::monospace(10.0), s.col);
        lx += g.width() + 14.0;
    }
    if !unit.is_empty() {
        p.text(egui::pos2(r.right() - 6.0, r.top() + 3.0), Align2::RIGHT_TOP, unit, FontId::monospace(10.0), FAINT);
    }
}

pub const G1: Color32 = Color32::from_gray(245);
pub const G2: Color32 = Color32::from_gray(170);
pub const G3: Color32 = Color32::from_gray(105);

/// Grayscale heat map. `bounds` is an optional highlighted region (fractions).
pub fn heatmap(ui: &mut Ui, data: &[Vec<f32>], size: egui::Vec2, range: (f32, f32), bounds: Option<(f32, f32, f32, f32)>) {
    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, crate::theme::RAD, Color32::BLACK);
    let rows = data.len().max(1);
    let cols = data.first().map(|d| d.len()).unwrap_or(1).max(1);
    let (cw, ch) = (r.width() / cols as f32, r.height() / rows as f32);
    for (j, row) in data.iter().enumerate() {
        for (i, v) in row.iter().enumerate() {
            let t = ((v - range.0) / (range.1 - range.0)).clamp(0.0, 1.0);
            let g = (30.0 + t * 200.0) as u8;
            let c = Rect::from_min_size(r.min + vec2(i as f32 * cw, j as f32 * ch), vec2(cw, ch));
            p.rect_filled(c.shrink(0.5), crate::theme::RAD, Color32::from_gray(g));
            if cw > 34.0 {
                p.text(c.center(), Align2::CENTER_CENTER, format!("{:+.0}", v * 1000.0), FontId::monospace(9.0), if g > 130 { Color32::BLACK } else { Color32::from_gray(200) });
            }
        }
    }
    if let Some((x0, y0, x1, y1)) = bounds {
        let b = Rect::from_min_max(r.min + vec2(x0 * r.width(), y0 * r.height()), r.min + vec2(x1 * r.width(), y1 * r.height()));
        p.rect_stroke(b, crate::theme::RAD, Stroke::new(2.0_f32, Color32::WHITE), StrokeKind::Inside);
    }
    p.rect_stroke(r, crate::theme::RAD, Stroke::new(1.0_f32, LINE2), StrokeKind::Inside);
}

pub fn fmt_dur(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}
