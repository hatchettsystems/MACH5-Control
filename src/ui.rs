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

/// Coarse, pre-blurred brightness field of the background wake, shared with every
/// translucent surface so it can paint a soft glow where the sharp characters are hidden.
pub struct FrostField {
    pub cw: f32,
    pub ch: f32,
    pub cols: usize,
    pub rows: usize,
    pub v: Vec<f32>,
}

pub fn set_frost(ctx: &egui::Context, f: Option<FrostField>) {
    let f = f.unwrap_or(FrostField { cw: 1.0, ch: 1.0, cols: 0, rows: 0, v: Vec::new() });
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("frost"), std::sync::Arc::new(f)));
}

/// A mesh of soft vertex-coloured quads covering `rect`: frosted glass over the wake.
pub fn frost_shape(ctx: &egui::Context, rect: Rect) -> Option<Shape> {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    let f = ctx.data(|d| d.get_temp::<std::sync::Arc<FrostField>>(egui::Id::new("frost")))?;
    if f.cols == 0 || rect.width() < 2.0 || rect.height() < 2.0 {
        return None;
    }
    let val = |i: isize, j: isize| -> f32 {
        let i = i.clamp(0, f.cols as isize - 1) as usize;
        let j = j.clamp(0, f.rows as isize - 1) as usize;
        f.v[j * f.cols + i]
    };
    let sample = |p: egui::Pos2| -> f32 {
        let (gx, gy) = (p.x / f.cw - 0.5, p.y / f.ch - 0.5);
        let (i0, j0) = (gx.floor(), gy.floor());
        let (fx, fy) = (gx - i0, gy - j0);
        let (i0, j0) = (i0 as isize, j0 as isize);
        let top = val(i0, j0) * (1.0 - fx) + val(i0 + 1, j0) * fx;
        let bot = val(i0, j0 + 1) * (1.0 - fx) + val(i0 + 1, j0 + 1) * fx;
        top * (1.0 - fy) + bot * fy
    };
    let breaks = |lo: f32, hi: f32, step: f32| -> Vec<f32> {
        let mut v = vec![lo];
        let mut k = (lo / step - 0.5).floor() as i32;
        loop {
            let c = (k as f32 + 0.5) * step;
            if c >= hi {
                break;
            }
            if c > lo + 0.5 {
                v.push(c);
            }
            k += 1;
        }
        v.push(hi);
        v
    };
    let xs = breaks(rect.left(), rect.right(), f.cw);
    let ys = breaks(rect.top(), rect.bottom(), f.ch);
    let mut mesh = Mesh::default();
    let mut peak = 0.0_f32;
    for &y in &ys {
        for &x in &xs {
            let v = sample(egui::pos2(x, y));
            peak = peak.max(v);
            let a = (v * 26.0).min(30.0) as u8;
            let k = (v * 1.3).clamp(0.0, 1.0) * 0.4;
            let base = [120.0_f32, 134.0, 150.0];
            let col = Color32::from_rgba_unmultiplied(
                (base[0] + (ACCENT.r() as f32 - base[0]) * k) as u8,
                (base[1] + (ACCENT.g() as f32 - base[1]) * k) as u8,
                (base[2] + (ACCENT.b() as f32 - base[2]) * k) as u8,
                a,
            );
            mesh.vertices.push(Vertex { pos: egui::pos2(x, y), uv: WHITE_UV, color: col });
        }
    }
    if peak < 0.02 {
        return None;
    }
    let w = xs.len();
    for j in 0..ys.len() - 1 {
        for i in 0..w - 1 {
            let a = (j * w + i) as u32;
            let (b, c, d) = (a + 1, a + w as u32, a + w as u32 + 1);
            mesh.indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    Some(Shape::mesh(mesh))
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
    let frame = Frame::new().fill(PANEL).stroke(Stroke::new(1.0_f32, LINE)).corner_radius(RAD as u8).inner_margin(Margin::same(10));
    let mut prep = frame.begin(ui);
    // Reserved after the frame's own fill slot, so the frosted glow sits on the glass, under the content.
    let slot = ui.painter().add(Shape::Noop);
    {
        let cu = &mut prep.content_ui;
        cu.set_width((w - 22.0).max(10.0));
        if !head.is_empty() {
            cu.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if let Some(g) = crate::icons::head_glyph(head) {
                    let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                    crate::icons::glyph(&ui.painter().clone(), g, r.center(), Color32::from_gray(205), 0.0);
                }
                ui.label(RichText::new(head).size(14.0).strong().color(WHITE));
            });
            let y = cu.cursor().top() + 2.0;
            let (l, r) = (cu.max_rect().left(), cu.max_rect().right());
            cu.painter().line_segment([egui::pos2(l, y), egui::pos2(r, y)], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 18)));
            cu.add_space(9.0);
        }
        f(cu);
    }
    let resp = prep.end(ui);
    let r = resp.rect;
    if let Some(s) = frost_shape(ui.ctx(), r) {
        ui.painter().set(slot, s);
    }
    set_anchor(ui.ctx(), head, r);
    let focus: String = ui.ctx().data(|d| d.get_temp(egui::Id::new("tour_focus"))).unwrap_or_default();
    if !focus.is_empty() && focus == head && !ui.clip_rect().contains_rect(r) {
        ui.scroll_to_rect(r, Some(egui::Align::Center));
    }
    ui.add_space(8.0);
}

/// Compact button for tables and rows: still sized to its text, never clipped.
pub fn sbtn(ui: &mut Ui, t: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = vec2(11.0, 3.0);
        ui.add(Button::new(RichText::new(t).size(12.0)).min_size(vec2(0.0, 24.0)))
    })
    .inner
}

/// Compact rotary knob. Hold the left button and drag up to increase, down to decrease;
/// hold Shift for fine control, or use the scroll wheel to nudge.
pub fn knob(ui: &mut Ui, val: &mut f32, range: (f32, f32), label: &str, unit: &str, decimals: usize) -> egui::Response {
    let size = vec2(58.0, 66.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let span = range.1 - range.0;
    let before = *val;
    if resp.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        *val -= resp.drag_delta().y * span / if fine { 700.0 } else { 150.0 };
    }
    if resp.hovered() {
        let s = ui.input(|i| i.raw_scroll_delta.y);
        if s != 0.0 {
            *val += s.signum() * span / 100.0;
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    if resp.double_clicked() {
        *val = range.0;
    }
    *val = val.clamp(range.0, range.1);
    if *val != before {
        resp.mark_changed();
    }
    let t = if span > 0.0 { (*val - range.0) / span } else { 0.0 };
    let c = egui::pos2(rect.center().x, rect.top() + 25.0);
    let p = ui.painter();
    let (a0, sweep) = (std::f32::consts::PI * 0.75, std::f32::consts::PI * 1.5);
    let arc = |from: f32, to: f32, r: f32| -> Vec<egui::Pos2> {
        let n = 36;
        (0..=n).map(|k| { let a = a0 + sweep * (from + (to - from) * k as f32 / n as f32); c + vec2(a.cos(), a.sin()) * r }).collect()
    };
    p.add(Shape::line(arc(0.0, 1.0, 21.0), Stroke::new(3.0_f32, Color32::from_gray(44))));
    if t > 0.003 {
        p.add(Shape::line(arc(0.0, t, 21.0), Stroke::new(3.0_f32, if resp.dragged() || resp.hovered() { Color32::from_rgb(120, 214, 255) } else { ACCENT })));
    }
    let body = if resp.dragged() { Color32::from_gray(40) } else { Color32::from_gray(26) };
    p.circle_filled(c, 15.5, body);
    p.circle_stroke(c, 15.5, Stroke::new(1.0_f32, if resp.hovered() || resp.dragged() { Color32::from_gray(150) } else { Color32::from_gray(76) }));
    let a = a0 + sweep * t;
    p.line_segment([c + vec2(a.cos(), a.sin()) * 5.0, c + vec2(a.cos(), a.sin()) * 13.0], Stroke::new(2.0_f32, Color32::WHITE));
    p.text(egui::pos2(rect.center().x, rect.top() + 49.0), Align2::CENTER_CENTER, format!("{:.*}{}", decimals, *val, unit), FontId::monospace(10.5), WHITE);
    p.text(egui::pos2(rect.center().x, rect.top() + 61.0), Align2::CENTER_CENTER, label, FontId::proportional(10.5), DIM);
    resp.on_hover_text(format!("{}: {:.*}{}\nDrag up / down to change, Shift = fine, scroll = nudge, double-click = minimum", label, decimals, *val, unit))
}

pub fn accent_bar(ui: &mut Ui, frac: f32, w: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 8.0), Sense::hover());
    ui.painter().rect_filled(r, 4.0, Color32::from_gray(10));
    ui.painter().rect_stroke(r, 4.0, Stroke::new(1.0_f32, LINE), StrokeKind::Inside);
    let fr = Rect::from_min_size(r.min, vec2(r.width() * frac.clamp(0.0, 1.0), r.height()));
    ui.painter().rect_filled(fr.shrink(1.0), 3.0, ACCENT);
}

pub fn tabs(ui: &mut Ui, cur: &mut usize, names: &[&str]) {
    let r = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().button_padding = vec2(14.0, 6.0);
        for (i, n) in names.iter().enumerate() {
            let sel = *cur == i;
            let col = if sel { WHITE } else { DIM };
            let resp = ui.add(Button::new(RichText::new(*n).color(col).size(13.5)).fill(Color32::TRANSPARENT).stroke(Stroke::NONE).min_size(vec2(0.0, 30.0)));
            if sel {
                let y = resp.rect.bottom();
                ui.painter().line_segment([egui::pos2(resp.rect.left() + 8.0, y), egui::pos2(resp.rect.right() - 8.0, y)], Stroke::new(2.0_f32, ACCENT));
            }
            if resp.clicked() {
                *cur = i;
            }
        }
    });
    set_anchor(ui.ctx(), "tabs", r.response.rect);
    let y = r.response.rect.bottom();
    ui.painter().line_segment([egui::pos2(r.response.rect.left(), y), egui::pos2(ui.max_rect().right(), y)], Stroke::new(1.0_f32, LINE));
    ui.add_space(10.0);
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
/// A single inset tile: small caption above a large value.
pub fn tile(ui: &mut Ui, w: f32, caption: &str, value: &str) {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, CARD_LINE))
        .corner_radius(7)
        .inner_margin(Margin::symmetric(11, 8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(w - 24.0);
                ui.set_min_height(36.0);
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(dim(caption));
                ui.add(egui::Label::new(mono(value).size(14.5).color(WHITE)).wrap());
            });
        });
}

/// A wrapped grid of equal tiles that fills the available width.
pub fn tiles(ui: &mut Ui, items: &[(&str, String)]) {
    tiles_min(ui, items, 168.0);
}
pub fn tiles_min(ui: &mut Ui, items: &[(&str, String)], min_w: f32) {
    let gap = 8.0;
    let avail = ui.available_width();
    let n = (((avail + gap) / (min_w + gap)).floor() as usize).clamp(1, items.len().max(1));
    let w = (avail - gap * (n as f32 - 1.0)) / n as f32;
    for row in items.chunks(n) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (k, v) in row {
                tile(ui, w, k, v);
            }
        });
        ui.add_space(gap - 2.0);
    }
}

/// Replaces the old two-column key/value grid with tiles.
pub fn kv(ui: &mut Ui, _id: &str, rows: &[(&str, String)]) {
    tiles(ui, rows);
}

/// Icon tile: glyph, big value, caption (like the reference status tiles).
pub fn stat_tile(ui: &mut Ui, w: f32, glyph: &str, value: &str, caption: &str) {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, CARD_LINE))
        .corner_radius(8)
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(w - 26.0);
                ui.spacing_mut().item_spacing.y = 3.0;
                let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                crate::icons::glyph(&ui.painter().clone(), glyph, r.center(), Color32::from_gray(190), 0.0);
                ui.label(mono(value).size(19.0).color(WHITE));
                ui.label(dim(caption));
            });
        });
}

/// One list item as its own rounded card, so rows read as separate things.
pub fn row_card(ui: &mut Ui, f: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, CARD_LINE))
        .corner_radius(9)
        .inner_margin(Margin::symmetric(11, 8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(w - 24.0);
                f(ui);
            });
        });
    ui.add_space(5.0);
}

/// Square icon box used at the start of a row card.
pub fn icon_box(ui: &mut Ui, glyph: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
    ui.painter().rect_filled(r, 8.0, Color32::from_rgb(22, 24, 29));
    ui.painter().rect_stroke(r, 8.0, Stroke::new(1.0_f32, CARD_LINE), StrokeKind::Inside);
    crate::icons::glyph(&ui.painter().clone(), glyph, r.center(), Color32::from_gray(215), 0.0);
}

/// `-  value  +` stepper. Returns true when changed.
pub fn stepper(ui: &mut Ui, v: &mut f32, range: (f32, f32), step: f32, unit: &str) -> bool {
    let mut ch = false;
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().button_padding = vec2(0.0, 0.0);
        if ui.add(Button::new(RichText::new("-").size(16.0)).min_size(vec2(30.0, 30.0))).clicked() { *v = (*v - step).max(range.0); ch = true; }
        let (r, _) = ui.allocate_exact_size(vec2(66.0, 30.0), Sense::hover());
        ui.painter().rect_filled(r, 7.0, INSET);
        ui.painter().rect_stroke(r, 7.0, Stroke::new(1.0_f32, CARD_LINE), StrokeKind::Inside);
        ui.painter().text(r.center(), Align2::CENTER_CENTER, format!("{:.0}{}", v, unit), FontId::monospace(13.0), WHITE);
        if ui.add(Button::new(RichText::new("+").size(16.0)).min_size(vec2(30.0, 30.0))).clicked() { *v = (*v + step).min(range.1); ch = true; }
    });
    ch
}

/// Round dial with an icon in the middle (drag up/down like `knob`).
pub fn dial(ui: &mut Ui, val: &mut f32, range: (f32, f32), label: &str, glyph: &str, rot: f32, unit: &str) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(66.0, 98.0), Sense::click_and_drag());
    let span = range.1 - range.0;
    let before = *val;
    if resp.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        *val -= resp.drag_delta().y * span / if fine { 700.0 } else { 150.0 };
    }
    if resp.hovered() {
        let s = ui.input(|i| i.raw_scroll_delta.y);
        if s != 0.0 { *val += s.signum() * span / 100.0; }
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    if resp.double_clicked() { *val = range.0; }
    *val = val.clamp(range.0, range.1);
    if *val != before { resp.mark_changed(); }
    let t = if span > 0.0 { (*val - range.0) / span } else { 0.0 };
    let c = egui::pos2(rect.center().x, rect.top() + 36.0);
    let p = ui.painter();
    let (a0, sweep) = (std::f32::consts::PI * 0.75, std::f32::consts::PI * 1.5);
    let arc = |to: f32, r: f32| -> Vec<egui::Pos2> {
        (0..=40).map(|k| { let a = a0 + sweep * (to * k as f32 / 40.0); c + vec2(a.cos(), a.sin()) * r }).collect()
    };
    p.circle_filled(c, 26.0, CARD);
    p.add(Shape::line(arc(1.0, 26.0), Stroke::new(4.0_f32, Color32::from_gray(46))));
    if t > 0.004 {
        p.add(Shape::line(arc(t, 26.0), Stroke::new(4.0_f32, if resp.dragged() || resp.hovered() { Color32::from_rgb(120, 214, 255) } else { ACCENT })));
    }
    crate::icons::glyph(&p.clone(), glyph, c, Color32::from_gray(225), rot);
    p.text(egui::pos2(rect.center().x, rect.top() + 78.0), Align2::CENTER_CENTER, format!("{:.0}{}", *val, unit), FontId::monospace(14.0), WHITE);
    p.text(egui::pos2(rect.center().x, rect.top() + 93.0), Align2::CENTER_CENTER, label, FontId::proportional(11.5), DIM);
    resp.on_hover_text(format!("{}: {:.0}{}\nDrag up / down, Shift = fine, scroll = nudge, double-click = minimum", label, *val, unit))
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

fn lerp_c(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Gradient used for the mesh: near-black, deep blue, accent, white.
pub fn ramp(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let stops = [(0.0, Color32::from_rgb(10, 12, 18)), (0.4, Color32::from_rgb(26, 70, 112)), (0.75, ACCENT), (1.0, Color32::WHITE)];
    for w in stops.windows(2) {
        if t <= w[1].0 {
            return lerp_c(w[0].1, w[1].1, (t - w[0].0) / (w[1].0 - w[0].0));
        }
    }
    Color32::WHITE
}

/// Smoothly interpolated value of a grid at fractional position (u, v in 0..1).
pub fn sample_grid(data: &[Vec<f32>], u: f32, v: f32) -> f32 {
    let rows = data.len();
    let cols = data.first().map(|d| d.len()).unwrap_or(0);
    if rows < 2 || cols < 2 {
        return 0.0;
    }
    let (gx, gy) = (u.clamp(0.0, 1.0) * (cols - 1) as f32, v.clamp(0.0, 1.0) * (rows - 1) as f32);
    let (x0, y0) = ((gx.floor() as usize).min(cols - 2), (gy.floor() as usize).min(rows - 2));
    let ease = |t: f32| t * t * (3.0 - 2.0 * t);
    let (fx, fy) = (ease(gx - x0 as f32), ease(gy - y0 as f32));
    let top = data[y0][x0] * (1.0 - fx) + data[y0][x0 + 1] * fx;
    let bot = data[y0 + 1][x0] * (1.0 - fx) + data[y0 + 1][x0 + 1] * fx;
    top * (1.0 - fy) + bot * fy
}

/// Smooth gradient heat map. Hovering reports the bed coordinate and the height there.
/// `bounds` is an optional highlighted region (fractions); `bed` is the bed size in mm.
pub fn heatmap(ui: &mut Ui, data: &[Vec<f32>], size: egui::Vec2, range: (f32, f32), bounds: Option<(f32, f32, f32, f32)>, bed: (f32, f32)) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    let (r, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, RAD, Color32::BLACK);
    const N: usize = 56;
    let mut mesh = Mesh::default();
    for j in 0..=N {
        for i in 0..=N {
            let (u, v) = (i as f32 / N as f32, j as f32 / N as f32);
            let t = (sample_grid(data, u, v) - range.0) / (range.1 - range.0);
            mesh.vertices.push(Vertex { pos: r.min + vec2(u * r.width(), v * r.height()), uv: WHITE_UV, color: ramp(t) });
        }
    }
    for j in 0..N {
        for i in 0..N {
            let a = (j * (N + 1) + i) as u32;
            let b = a + 1;
            let c = a + (N + 1) as u32;
            let d = c + 1;
            mesh.indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    p.add(egui::Shape::mesh(mesh));
    if let Some((x0, y0, x1, y1)) = bounds {
        let b = Rect::from_min_max(r.min + vec2(x0 * r.width(), y0 * r.height()), r.min + vec2(x1 * r.width(), y1 * r.height()));
        p.rect_stroke(b, 2.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Inside);
    }
    p.rect_stroke(r, RAD, Stroke::new(1.0_f32, LINE2), StrokeKind::Inside);
    if r.width() > 120.0 {
        for k in 0..=4 {
            let f = k as f32 / 4.0;
            p.text(r.left_bottom() + vec2(f * r.width(), -2.0), if k == 0 { Align2::LEFT_BOTTOM } else if k == 4 { Align2::RIGHT_BOTTOM } else { Align2::CENTER_BOTTOM }, format!("{:.0}", f * bed.0), FontId::monospace(9.0), Color32::from_white_alpha(140));
            p.text(r.left_top() + vec2(3.0, f * r.height()), if k == 0 { Align2::LEFT_TOP } else if k == 4 { Align2::LEFT_BOTTOM } else { Align2::LEFT_CENTER }, format!("{:.0}", f * bed.1), FontId::monospace(9.0), Color32::from_white_alpha(140));
        }
    }
    if let Some(pos) = resp.hover_pos() {
        let (u, v) = (((pos.x - r.left()) / r.width()).clamp(0.0, 1.0), ((pos.y - r.top()) / r.height()).clamp(0.0, 1.0));
        let z = sample_grid(data, u, v);
        let s = Stroke::new(1.0_f32, Color32::from_white_alpha(170));
        p.line_segment([egui::pos2(pos.x, r.top()), egui::pos2(pos.x, r.bottom())], s);
        p.line_segment([egui::pos2(r.left(), pos.y), egui::pos2(r.right(), pos.y)], s);
        p.circle_stroke(pos, 5.0, Stroke::new(1.5_f32, Color32::WHITE));
        resp.on_hover_text(format!("X {:.1} mm   Y {:.1} mm\nZ {:+.3} mm  ({:+.0} um)", u * bed.0, v * bed.1, z, z * 1000.0));
    }
}

/// Legend strip for `ramp`.
pub fn ramp_legend(ui: &mut Ui, w: f32, range: (f32, f32)) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 10.0), Sense::hover());
    let n = 48;
    for i in 0..n {
        let seg = Rect::from_min_size(r.min + vec2(i as f32 / n as f32 * w, 0.0), vec2(w / n as f32 + 0.5, 10.0));
        ui.painter().rect_filled(seg, 0.0, ramp(i as f32 / (n - 1) as f32));
    }
    ui.horizontal(|ui| {
        ui.label(dim(format!("{:+.3} mm", range.0)));
        ui.add_space((w - 130.0).max(0.0));
        ui.label(dim(format!("{:+.3} mm", range.1)));
    });
}

pub fn fmt_dur(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}
