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

/// All-caps display text (Badd Mono) for titles, hero status and big values.
pub fn disp(s: impl Into<String>, size: f32) -> RichText {
    RichText::new(s.into()).font(FontId::new(size, crate::theme::display_family()))
}

pub fn mono(s: impl Into<String>) -> RichText {
    RichText::new(s.into()).monospace()
}
pub fn dim(s: impl Into<String>) -> RichText {
    RichText::new(s.into()).color(DIM).size(12.0)
}

pub fn title(ui: &mut Ui, t: &str, sub: &str) {
    let in_hub = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("in_hub"))).unwrap_or(false);
    if in_hub {
        // The hub already shows the page title and tabs; keep only the page's description.
        ui.label(dim(sub));
        ui.add_space(6.0);
        return;
    }
    ui.label(disp(t, 24.0).color(WHITE));
    ui.label(dim(sub));
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
}

/// Flat bordered section with a small caps header.
pub fn panel(ui: &mut Ui, head: &str, f: impl FnOnce(&mut Ui)) {
    panel_min(ui, head, 0.0, f)
}

/// A card with a minimum height, so cards in one row line up.
pub fn panel_min(ui: &mut Ui, head: &str, min_h: f32, f: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let expect_right = ui.cursor().left() + w;
    // A page can mark the next card as draggable (id) so its header becomes a handle.
    let drag_id: Option<u8> = ui.ctx().data_mut(|d| d.remove_temp::<u8>(egui::Id::new("cur_card")));
    let frame = Frame::new().fill(PANEL).stroke(Stroke::new(1.0_f32, LINE)).corner_radius(RAD as u8).inner_margin(Margin::same(10));
    let mut prep = frame.begin(ui);
    // Reserved after the frame's own fill slot, so the frosted glow sits on the glass, under the content.
    let slot = ui.painter().add(Shape::Noop);
    {
        let cu = &mut prep.content_ui;
        cu.set_width((w - 22.0).max(10.0));
        cu.set_min_height((min_h - 22.0).max(0.0));
        if !head.is_empty() {
            let hdr = cu.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if drag_id.is_some() {
                    // grip: two columns of three dots
                    let (r, _) = ui.allocate_exact_size(vec2(10.0, 20.0), Sense::hover());
                    for dx in [-2.5_f32, 2.5] {
                        for dy in [-5.0_f32, 0.0, 5.0] {
                            ui.painter().circle_filled(r.center() + vec2(dx, dy), 1.3, Color32::from_gray(120));
                        }
                    }
                }
                if let Some(g) = crate::icons::head_glyph(head) {
                    let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                    crate::icons::glyph(&ui.painter().clone(), g, r.center(), Color32::from_gray(205), 0.0);
                }
                ui.add(egui::Label::new(disp(head, 16.0).color(WHITE)).truncate());
            });
            if let Some(id) = drag_id {
                let hr = hdr.response.rect;
                let full = Rect::from_min_max(hr.min, egui::pos2(cu.max_rect().right(), hr.max.y));
                let resp = cu.interact(full, egui::Id::new(("card_hdr", id)), Sense::click_and_drag());
                let ctx = cu.ctx().clone();
                if resp.dragged() {
                    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                    ctx.data_mut(|d| d.insert_temp(egui::Id::new("dash_drag"), id));
                } else if resp.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::Grab);
                }
                if resp.drag_stopped() {
                    let p = ctx.input(|i| i.pointer.interact_pos()).unwrap_or(egui::Pos2::ZERO);
                    ctx.data_mut(|d| {
                        d.remove_temp::<u8>(egui::Id::new("dash_drag"));
                        d.insert_temp(egui::Id::new("drag_drop"), (id, p));
                    });
                }
            }
            let y = cu.cursor().top() + 2.0;
            let (l, r) = (cu.max_rect().left(), cu.max_rect().right());
            cu.painter().line_segment([egui::pos2(l, y), egui::pos2(r, y)], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 18)));
            cu.add_space(9.0);
        }
        // Columns lay children out "justified"; force a plain top-down layout so buttons keep their natural width.
        cu.with_layout(egui::Layout::top_down(egui::Align::Min), |cu| f(cu));
    }
    let resp = prep.end(ui);
    let r = resp.rect;
    if std::env::var("MACH5_DEBUG").is_ok() && r.right() > expect_right + 2.0 {
        eprintln!("CARD-OVERFLOW '{}' wider by {:.0}px", head, r.right() - expect_right);
    }
    if let Some(s) = frost_shape(ui.ctx(), r) {
        ui.painter().set(slot, s);
    }
    if let Some(id) = drag_id {
        ui.ctx().data_mut(|d| {
            let mut v: Vec<(u8, Rect)> = d.get_temp(egui::Id::new("card_rects")).unwrap_or_default();
            v.push((id, r));
            d.insert_temp(egui::Id::new("card_rects"), v);
        });
    }
    set_anchor(ui.ctx(), head, r);
    let focus: String = ui.ctx().data(|d| d.get_temp(egui::Id::new("tour_focus"))).unwrap_or_default();
    if !focus.is_empty() && focus == head && !ui.clip_rect().contains_rect(r) {
        ui.scroll_to_rect(r, Some(egui::Align::Center));
    }
    ui.add_space(8.0);
}

/// Responsive columns: side by side when each column can be at least `min_w` wide,
/// otherwise stacked, so nothing is clipped on narrow windows.
pub struct Cols<'a> {
    solo: Option<&'a mut Ui>,
    many: Option<&'a mut [Ui]>,
}
impl<'a> Cols<'a> {
    pub fn at(&mut self, i: usize) -> &mut Ui {
        if let Some(u) = self.solo.as_deref_mut() {
            u
        } else {
            &mut self.many.as_deref_mut().unwrap()[i]
        }
    }
}
pub fn adaptive_w(ui: &mut Ui, n: usize, min_w: f32, f: impl FnOnce(&mut Cols)) {
    let w = ui.available_width();
    if w >= n as f32 * min_w + (n as f32 - 1.0) * 8.0 {
        ui.columns(n, |cols| {
            let mut c = Cols { solo: None, many: Some(cols) };
            f(&mut c)
        });
    } else {
        let mut c = Cols { solo: Some(ui), many: None };
        f(&mut c);
    }
}
pub fn adaptive(ui: &mut Ui, n: usize, f: impl FnOnce(&mut Cols)) {
    adaptive_w(ui, n, 420.0, f)
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
            let resp = ui.add(Button::new(disp(*n, 14.5).color(col)).fill(Color32::TRANSPARENT).stroke(Stroke::NONE).min_size(vec2(0.0, 30.0)));
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

#[derive(Clone, Copy, PartialEq)]
enum BK {
    /// the one main action of a form or card: solid accent
    Primary,
    /// ordinary action: dark with a thin neutral outline
    Secondary,
    /// row action: plain text, no box until hovered
    Ghost,
    /// chosen option in a filter row: tinted, not solid
    Selected,
}

fn with_button_style<R>(ui: &mut Ui, kind: BK, f: impl FnOnce(&mut Ui) -> R) -> R {
    // Applied in place (not in a child ui) so buttons still wrap inside horizontal_wrapped rows.
    let saved = ui.style().clone();
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    let v = ui.visuals_mut();
    let none = Stroke::NONE;
    let (fill, hov, act, stroke, hstroke, fg, hfg) = match kind {
        BK::Primary => (ACCENT, Color32::from_rgb(130, 218, 255), Color32::WHITE, Stroke::new(1.0_f32, ACCENT), Stroke::new(1.0_f32, Color32::WHITE), Color32::from_rgb(4, 14, 22), Color32::from_rgb(4, 14, 22)),
        BK::Secondary => (Color32::from_rgb(20, 22, 27), Color32::from_rgb(30, 34, 42), Color32::from_rgb(40, 46, 56), Stroke::new(1.0_f32, Color32::from_rgb(58, 62, 72)), Stroke::new(1.0_f32, Color32::from_rgb(96, 112, 130)), Color32::from_rgb(226, 230, 236), Color32::WHITE),
        BK::Ghost => (Color32::TRANSPARENT, Color32::from_rgba_unmultiplied(255, 255, 255, 16), Color32::from_rgba_unmultiplied(255, 255, 255, 28), none, none, Color32::from_rgb(168, 176, 188), Color32::WHITE),
        BK::Selected => (Color32::from_rgb(14, 40, 60), Color32::from_rgb(20, 54, 80), Color32::from_rgb(26, 66, 96), Stroke::new(1.0_f32, Color32::from_rgb(30, 120, 168)), Stroke::new(1.0_f32, ACCENT), Color32::from_rgb(150, 218, 255), Color32::WHITE),
    };
    v.widgets.inactive.weak_bg_fill = fill;
    v.widgets.inactive.bg_fill = fill;
    v.widgets.inactive.bg_stroke = stroke;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, fg);
    v.widgets.hovered.weak_bg_fill = hov;
    v.widgets.hovered.bg_fill = hov;
    v.widgets.hovered.bg_stroke = hstroke;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, hfg);
    v.widgets.active.weak_bg_fill = act;
    v.widgets.active.bg_fill = act;
    v.widgets.active.bg_stroke = hstroke;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, hfg);
    let r = f(ui);
    ui.set_style(saved);
    r
}

/// The one main action of a form or card: solid accent fill.
pub fn primary(ui: &mut Ui, t: &str) -> bool {
    with_button_style(ui, BK::Primary, |ui| ui.add(Button::new(RichText::new(t).strong()).min_size(vec2(0.0, 32.0))).clicked())
}
/// Ordinary button: dark, thin neutral outline.
pub fn button(ui: &mut Ui, t: &str) -> bool {
    with_button_style(ui, BK::Secondary, |ui| ui.add(Button::new(t).min_size(vec2(0.0, 32.0))).clicked())
}
/// Like `button`, but can be disabled without wrapping the button in a child ui.
pub fn button_en(ui: &mut Ui, enabled: bool, t: &str) -> bool {
    with_button_style(ui, BK::Secondary, |ui| ui.add_enabled(enabled, Button::new(t).min_size(vec2(0.0, 32.0))).clicked())
}
/// Plain-text action ("Open nodes"): no box until hovered.
pub fn ghost(ui: &mut Ui, t: &str) -> bool {
    with_button_style(ui, BK::Ghost, |ui| ui.add(Button::new(RichText::new(t).size(13.0)).min_size(vec2(0.0, 28.0))).clicked())
}
/// Small outlined chip-button: presets and macros.
pub fn chip_btn(ui: &mut Ui, t: &str) -> bool {
    with_button_style(ui, BK::Secondary, |ui| {
        ui.spacing_mut().button_padding = vec2(10.0, 3.0);
        ui.add(Button::new(RichText::new(t).size(12.5)).min_size(vec2(0.0, 26.0))).clicked()
    })
}
/// Compact row action: plain text.
pub fn sbtn(ui: &mut Ui, t: &str) -> egui::Response {
    with_button_style(ui, BK::Ghost, |ui| {
        ui.spacing_mut().button_padding = vec2(8.0, 3.0);
        ui.add(Button::new(RichText::new(t).size(12.5)).min_size(vec2(0.0, 26.0)))
    })
}
/// Icon-only button with a tooltip. `accent` tints the icon with the accent colour.
pub fn icon_btn(ui: &mut Ui, glyph: &str, tip: &str, accent: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
    let hot = resp.hovered();
    if hot {
        ui.painter().rect_filled(r, 7.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20));
    }
    let col = if accent { if hot { Color32::from_rgb(130, 218, 255) } else { ACCENT } } else if hot { WHITE } else { Color32::from_rgb(160, 168, 180) };
    crate::icons::glyph(&ui.painter().clone(), glyph, r.center(), col, 0.0);
    resp.on_hover_text(tip).clicked()
}
/// Toggle for a view mode: icon, tinted when selected.
pub fn icon_toggle(ui: &mut Ui, glyph: &str, tip: &str, selected: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(30.0, 28.0), Sense::click());
    let hot = resp.hovered();
    if selected {
        ui.painter().rect_filled(r, 7.0, Color32::from_rgb(14, 40, 60));
        ui.painter().rect_stroke(r, 7.0, Stroke::new(1.0_f32, Color32::from_rgb(30, 120, 168)), StrokeKind::Inside);
    } else if hot {
        ui.painter().rect_filled(r, 7.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20));
    }
    let col = if selected { ACCENT } else if hot { WHITE } else { Color32::from_rgb(160, 168, 180) };
    crate::icons::glyph(&ui.painter().clone(), glyph, r.center(), col, 0.0);
    resp.on_hover_text(tip).clicked()
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
/// A single inset tile: small caption above a large value. `h` is the shared row height.
pub fn tile(ui: &mut Ui, w: f32, h: f32, caption: &str, value: &str) {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, CARD_LINE))
        .corner_radius(8)
        .inner_margin(Margin::symmetric(11, 8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(w - 24.0);
                ui.set_min_height((h - 18.0).max(20.0));
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(dim(caption));
                ui.add(egui::Label::new(mono(value).size(14.5).color(WHITE)).wrap());
            });
        });
}

/// A grid of equal tiles that fills the available width and keeps every row one height.
pub fn tiles(ui: &mut Ui, items: &[(&str, String)]) {
    tiles_min(ui, items, 168.0);
}
pub fn tiles_min(ui: &mut Ui, items: &[(&str, String)], min_w: f32) {
    let gap = 8.0;
    let avail = ui.available_width();
    let n = (((avail + gap) / (min_w + gap)).floor() as usize).clamp(1, items.len().max(1));
    let w = (avail - gap * (n as f32 - 1.0)) / n as f32;
    for row in items.chunks(n) {
        let mut vh = 0.0_f32;
        for (_, v) in row {
            let g = ui.painter().layout(v.clone(), FontId::monospace(14.5), WHITE, (w - 24.0).max(20.0));
            vh = vh.max(g.size().y);
        }
        let h = vh + 15.0 + 2.0 + 16.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (k, v) in row {
                tile(ui, w, h, k, v);
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
                ui.label(disp(value, 24.0).color(WHITE));
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
    pub fill: bool,
}
impl Series {
    pub fn new(name: &str, col: Color32, pts: Vec<[f32; 2]>) -> Self {
        Self { name: name.into(), col, pts, dashed: false, fill: false }
    }
    pub fn dashed(mut self) -> Self {
        self.dashed = true;
        self
    }
    /// Soft gradient shading under the curve, like Mainsail.
    pub fn filled(mut self) -> Self {
        self.fill = true;
        self
    }
    pub fn from_ring(name: &str, col: Color32, d: &std::collections::VecDeque<f32>) -> Self {
        Self::new(name, col, d.iter().enumerate().map(|(i, v)| [i as f32, *v]).collect())
    }
}

/// One colour per sensor, deliberately unrelated to the accent.
pub const SENS_HOT: Color32 = Color32::from_rgb(255, 143, 64);
pub const SENS_BED: Color32 = Color32::from_rgb(176, 112, 255);
pub const SENS_CHAM: Color32 = Color32::from_rgb(52, 214, 160);

pub fn plot(ui: &mut Ui, h: f32, series: &[Series], x: (f32, f32), y: (f32, f32), unit: &str) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, 4.0, Color32::from_rgb(6, 7, 9));
    p.rect_stroke(r, 4.0, Stroke::new(1.0_f32, LINE), StrokeKind::Inside);
    let inner = Rect::from_min_max(r.min + vec2(44.0, 8.0), r.max - vec2(8.0, 8.0));
    let fy = |v: f32| inner.bottom() - (v - y.0) / (y.1 - y.0).max(1e-6) * inner.height();
    let fx = |v: f32| inner.left() + (v - x.0) / (x.1 - x.0).max(1e-6) * inner.width();
    let span = y.1 - y.0;
    for i in 0..=4 {
        let v = y.0 + (y.1 - y.0) * i as f32 / 4.0;
        let yy = fy(v);
        p.line_segment([egui::pos2(inner.left(), yy), egui::pos2(inner.right(), yy)], Stroke::new(1.0_f32, Color32::from_gray(26)));
        p.text(egui::pos2(inner.left() - 4.0, yy), Align2::RIGHT_CENTER, if span < 0.5 { format!("{:.3}", v) } else if span < 5.0 { format!("{:.2}", v) } else if span < 50.0 { format!("{:.1}", v) } else { format!("{:.0}", v) }, FontId::monospace(10.0), DIM);
    }
    for s in series {
        let pts: Vec<_> = s.pts.iter().map(|q| egui::pos2(fx(q[0]), fy(q[1].clamp(y.0, y.1)))).collect();
        if pts.len() < 2 {
            continue;
        }
        if s.fill {
            let mut mesh = Mesh::default();
            let (top, bot) = (Color32::from_rgba_unmultiplied(s.col.r(), s.col.g(), s.col.b(), 34), Color32::from_rgba_unmultiplied(s.col.r(), s.col.g(), s.col.b(), 0));
            for q in &pts {
                mesh.vertices.push(Vertex { pos: *q, uv: WHITE_UV, color: top });
                mesh.vertices.push(Vertex { pos: egui::pos2(q.x, inner.bottom()), uv: WHITE_UV, color: bot });
            }
            for i in 0..pts.len() as u32 - 1 {
                let k = i * 2;
                mesh.indices.extend_from_slice(&[k, k + 1, k + 2, k + 1, k + 3, k + 2]);
            }
            p.add(Shape::mesh(mesh));
        }
        if s.dashed {
            p.extend(Shape::dashed_line(&pts, Stroke::new(1.0_f32, s.col.gamma_multiply(0.7)), 4.0, 3.0));
        } else {
            p.add(Shape::line(pts, Stroke::new(1.8_f32, s.col)));
        }
    }
    let mut lx = inner.left() + 6.0;
    for s in series.iter().filter(|s| !s.name.is_empty()) {
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
        ui.set_max_width(w);
        ui.label(dim(format!("{:+.3} mm", range.0)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(dim(format!("{:+.3} mm", range.1)));
        });
    });
}

pub fn fmt_dur(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

/// Console palette: G/M codes use the accent, macros a violet, then warning / error / ok.
pub const MACRO: Color32 = Color32::from_rgb(176, 148, 255);

fn is_gm(t: &str) -> bool {
    let mut c = t.chars();
    matches!(c.next(), Some('G') | Some('M') | Some('g') | Some('m')) && t.len() > 1 && t[1..].chars().all(|x| x.is_ascii_digit() || x == '.')
}
fn is_macro(t: &str) -> bool {
    t.len() >= 3 && t.chars().all(|x| x.is_ascii_uppercase() || x.is_ascii_digit() || x == '_') && t.chars().any(|x| x.is_ascii_uppercase())
}
fn split_param(t: &str) -> Option<(char, &str)> {
    let mut c = t.chars();
    let l = c.next()?;
    let rest = &t[l.len_utf8()..];
    (l.is_ascii_alphabetic() && !rest.is_empty() && rest.chars().all(|x| x.is_ascii_digit() || x == '.' || x == '-')).then_some((l, rest))
}

/// Colour-coded layout for one console line.
pub fn console_job(line: &str) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    let f = |c: Color32| TextFormat { font_id: FontId::monospace(13.0), color: c, ..Default::default() };
    let mut j = LayoutJob::default();
    let lower = line.to_lowercase();
    if let Some(rest) = line.strip_prefix("> ") {
        j.append("> ", 0.0, f(FAINT));
        for (i, tok) in rest.split(' ').enumerate() {
            if i > 0 {
                j.append(" ", 0.0, f(DIM));
            }
            if i == 0 && is_gm(tok) {
                j.append(&tok.to_uppercase(), 0.0, f(ACCENT));
            } else if i == 0 && is_macro(tok) {
                j.append(tok, 0.0, f(MACRO));
            } else if let Some((l, v)) = split_param(tok) {
                j.append(&l.to_string(), 0.0, f(Color32::from_rgb(150, 160, 175)));
                j.append(v, 0.0, f(WHITE));
            } else {
                j.append(tok, 0.0, f(TEXT));
            }
        }
    } else if lower.starts_with("error") || line.starts_with("!!") {
        j.append(line, 0.0, f(ERR));
    } else if lower.starts_with("warning") {
        j.append(line, 0.0, f(WARN));
    } else if lower.starts_with("ok") {
        j.append("ok", 0.0, f(OK));
        j.append(&line[2..], 0.0, f(DIM));
    } else if line.starts_with("//") {
        j.append(line, 0.0, f(FAINT));
    } else {
        j.append(line, 0.0, f(Color32::from_rgb(170, 175, 185)));
    }
    j
}

/// Standard card widths for page layouts: full, half and quarter of the row.
#[derive(Clone, Copy, PartialEq)]
pub enum Span { Full, Half, Quarter }


/// A card with a thin border; the border takes the accent colour when `selected`.
pub fn card_frame(ui: &mut Ui, w: f32, selected: bool, f: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(if selected { 1.5_f32 } else { 1.0_f32 }, if selected { ACCENT } else { CARD_LINE }))
        .corner_radius(10)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(w - 26.0);
                f(ui);
            });
        });
}

/// Responsive grid of cards: as many columns as fit at `min_w`, never clipped.
pub fn card_grid(ui: &mut Ui, count: usize, min_w: f32, mut f: impl FnMut(&mut Ui, usize, f32)) {
    let gap = 8.0;
    let avail = ui.available_width();
    let n = (((avail + gap) / (min_w + gap)).floor() as usize).clamp(1, count.max(1));
    let w = (avail - gap * (n as f32 - 1.0)) / n as f32 - 1.0;
    let all: Vec<usize> = (0..count).collect();
    for row in all.chunks(n) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for &i in row {
                ui.allocate_ui_with_layout(vec2(w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(w);
                    f(ui, i, w);
                });
            }
        });
        ui.add_space(gap);
    }
}

/// Mark the next `panel` as a draggable dashboard card with this id.
pub fn set_card(ctx: &egui::Context, id: u8) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("cur_card"), id));
}

/// Row card with a title line and labelled wrapped fields, used instead of wide tables.
pub fn info_card(ui: &mut Ui, title: &str, fields: &[(&str, String)]) {
    row_card(ui, |ui| {
        ui.add(egui::Label::new(RichText::new(title).strong().size(13.5).color(WHITE)).wrap());
        for (k, v) in fields {
            ui.horizontal_wrapped(|ui| {
                ui.label(dim(*k));
                ui.add(egui::Label::new(RichText::new(v.clone()).size(12.5)).wrap());
            });
        }
    });
}



/// Labelled `- value +` stepper with decimals, used in dense parameter grids.
pub fn param(ui: &mut Ui, label: &str, v: &mut f32, range: (f32, f32), step: f32, dec: usize, unit: &str) -> bool {
    let mut ch = false;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(dim(label));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.spacing_mut().button_padding = vec2(0.0, 0.0);
            if ui.add(Button::new(RichText::new("-").size(15.0)).min_size(vec2(28.0, 28.0))).clicked() { *v = (*v - step).max(range.0); ch = true; }
            let (r, _) = ui.allocate_exact_size(vec2(84.0, 28.0), Sense::hover());
            ui.painter().rect_filled(r, 7.0, INSET);
            ui.painter().rect_stroke(r, 7.0, Stroke::new(1.0_f32, CARD_LINE), StrokeKind::Inside);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, format!("{:.*}{}", dec, v, unit), FontId::monospace(12.5), WHITE);
            if ui.add(Button::new(RichText::new("+").size(15.0)).min_size(vec2(28.0, 28.0))).clicked() { *v = (*v + step).min(range.1); ch = true; }
        });
    });
    ch
}

/// Masonry layout: cards (of Full / Half / Quarter width) are packed into the highest free
/// space on a 4, 2 or 1 column grid, so short cards never leave gaps.
pub fn masonry(ui: &mut Ui, items: &[(u8, Span)], mut f: impl FnMut(&mut Ui, u8)) {
    let gap = 8.0;
    let w = ui.available_width();
    let n: usize = if w >= 1060.0 { 4 } else if w >= 560.0 { 2 } else { 1 };
    let col_w = (w - gap * (n as f32 - 1.0)) / n as f32;
    let ctx = ui.ctx().clone();
    let height = |id: u8| -> f32 { ctx.data(|d| d.get_temp::<f32>(egui::Id::new(("card_h", id)))).unwrap_or(380.0) };
    let cols_of = |s: Span| -> usize {
        match (n, s) {
            (4, Span::Full) => 4,
            (4, Span::Half) => 2,
            (2, Span::Full) => 2,
            _ => 1,
        }
    };
    let mut col_h = vec![0.0_f32; n];
    let mut placed: Vec<(u8, f32, f32, usize)> = Vec::new(); // id, x, y, span cols
    for (id, span) in items {
        let sc = cols_of(*span);
        let mut best = (0usize, f32::MAX);
        for x in 0..=(n - sc) {
            let y = col_h[x..x + sc].iter().cloned().fold(0.0_f32, f32::max);
            if y < best.1 - 0.5 {
                best = (x, y);
            }
        }
        let h = height(*id);
        for c in col_h.iter_mut().skip(best.0).take(sc) {
            *c = best.1 + h;
        }
        placed.push((*id, best.0 as f32 * (col_w + gap), best.1, sc));
    }
    let total = col_h.iter().cloned().fold(0.0_f32, f32::max);
    let (outer, _) = ui.allocate_exact_size(vec2(w, total.max(60.0)), Sense::hover());
    let mut changed = false;
    for (id, x, y, sc) in placed {
        let cw = col_w * sc as f32 + gap * (sc as f32 - 1.0);
        let rect = Rect::from_min_size(outer.min + vec2(x, y), vec2(cw, 20000.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_width(cw);
        f(&mut child, id);
        let h = child.min_rect().height();
        if (h - height(id)).abs() > 1.0 {
            changed = true;
        }
        ctx.data_mut(|d| d.insert_temp(egui::Id::new(("card_h", id)), h));
    }
    if changed {
        ctx.request_repaint();
    }
}

// ---------------------------------------------------------------------------------------------
// Jog dial: concentric step rings split into four wedges (X-, Y+, X+, Y-), corner home badges,
// and a vertical Z strip with a motors-off button in the middle.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Jog {
    Move(usize, f32),
    HomeX,
    HomeY,
    HomeZ,
    HomeAll,
    MotorsOff,
}

const RINGS: [f32; 4] = [1.0, 10.0, 50.0, 100.0];
const ZSTEPS: [f32; 4] = [0.1, 1.0, 10.0, 50.0];

fn annular(p: &egui::Painter, c: egui::Pos2, r_in: f32, r_out: f32, a0: f32, a1: f32, fill: Color32, stroke: Stroke) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    let n = 20;
    let mut mesh = Mesh::default();
    let mut outline = Vec::new();
    for k in 0..=n {
        let a = a0 + (a1 - a0) * k as f32 / n as f32;
        let d = vec2(a.cos(), a.sin());
        mesh.vertices.push(Vertex { pos: c + d * r_in, uv: WHITE_UV, color: fill });
        mesh.vertices.push(Vertex { pos: c + d * r_out, uv: WHITE_UV, color: fill });
    }
    for k in 0..n as u32 {
        let i = k * 2;
        mesh.indices.extend_from_slice(&[i, i + 1, i + 2, i + 1, i + 3, i + 2]);
    }
    p.add(Shape::mesh(mesh));
    for k in 0..=n {
        let a = a0 + (a1 - a0) * k as f32 / n as f32;
        outline.push(c + vec2(a.cos(), a.sin()) * r_out);
    }
    p.add(Shape::line(outline, stroke));
}

/// A home badge: accent blue when that axis is homed, grey when it is not.
fn corner_badge(p: &egui::Painter, rect: Rect, label: &str, hot: bool, lit: bool) {
    let fill = if lit { if hot { Color32::from_rgb(130, 218, 255) } else { ACCENT } } else if hot { Color32::from_rgb(92, 98, 110) } else { Color32::from_rgb(60, 65, 75) };
    p.rect_filled(rect, 8.0, fill);
    let ink = if lit { Color32::from_rgb(4, 14, 22) } else { Color32::from_rgb(190, 196, 206) };
    crate::icons::house(p, rect.center() - vec2(0.0, rect.height() * 0.12), rect.width() * 0.26, ink, fill);
    p.text(rect.center() + vec2(0.0, rect.height() * 0.30), Align2::CENTER_CENTER, label, FontId::monospace((rect.height() * 0.26).clamp(8.0, 11.0)), ink);
}

fn chevron(p: &egui::Painter, c: egui::Pos2, dir: egui::Vec2, size: f32, col: Color32) {
    let n = vec2(-dir.y, dir.x);
    p.add(Shape::convex_polygon(vec![c + dir * size, c - dir * size * 0.6 + n * size * 0.9, c - dir * size * 0.6 - n * size * 0.9], col, Stroke::NONE));
}

/// Returns the action clicked this frame, if any.
pub fn jog_pad(ui: &mut Ui, side: f32, homed: [bool; 3]) -> Option<Jog> {
    let zw = (side * 0.2).clamp(40.0, 52.0);
    let gap = 16.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(side + gap + zw, side), Sense::click());
    let pad = Rect::from_min_size(rect.min, vec2(side, side));
    let zr = Rect::from_min_size(rect.min + vec2(side + gap, 0.0), vec2(zw, side));
    let p = ui.painter_at(rect);
    let hover = resp.hover_pos();
    let click = if resp.clicked() { resp.interact_pointer_pos() } else { None };
    let mut action = None;
    let hot_fill = Color32::from_rgb(22, 72, 104);
    let line = Stroke::new(1.0_f32, Color32::from_rgb(46, 50, 60));
    let dim_txt = Color32::from_rgb(120, 128, 142);

    // --- circular pad: 4 rings x 4 wedges ---
    let c = pad.center();
    let rr = side / 2.0 - 1.0;
    let r0 = rr * 0.26;
    let bounds = [r0, rr * 0.46, rr * 0.64, rr * 0.82, rr];
    p.circle_filled(c, rr + 1.0, Color32::from_rgb(10, 11, 14));
    // (axis, sign, from deg, to deg, direction vector)
    let wedges: [(usize, f32, f32, f32, egui::Vec2, &str); 4] = [
        (1, 1.0, 225.0, 315.0, vec2(0.0, -1.0), "Y+"),
        (0, 1.0, -45.0, 45.0, vec2(1.0, 0.0), "X+"),
        (1, -1.0, 45.0, 135.0, vec2(0.0, 1.0), "Y-"),
        (0, -1.0, 135.0, 225.0, vec2(-1.0, 0.0), "X-"),
    ];
    let cell_at = |h: egui::Pos2| -> Option<(usize, usize)> {
        let d = h - c;
        let r = d.length();
        if r < bounds[0] || r > rr {
            return None;
        }
        let ring = (0..4).find(|i| r >= bounds[*i] && r <= bounds[*i + 1])?;
        let wedge = if d.y.abs() > d.x.abs() { if d.y < 0.0 { 0 } else { 2 } } else if d.x > 0.0 { 1 } else { 3 };
        Some((wedge, ring))
    };
    let hover_cell = hover.and_then(cell_at);
    for (wi, (_, _, a0, a1, _, _)) in wedges.iter().enumerate() {
        for ring in 0..4 {
            let hot = hover_cell == Some((wi, ring));
            let shade = 24 + ring as u8 * 6;
            let fill = if hot { hot_fill } else { Color32::from_rgb(shade, shade + 1, shade + 4) };
            annular(&p, c, bounds[ring] + 1.5, bounds[ring + 1] - 1.0, (a0 + 4.0).to_radians(), (a1 - 4.0).to_radians(), fill, if hot { Stroke::new(1.5_f32, ACCENT) } else { line });
        }
    }
    // step values on every wedge, centred in each ring
    for (wi, (_, _, _, _, dir, _)) in wedges.iter().enumerate() {
        for (ring, s) in RINGS.iter().enumerate() {
            let r = (bounds[ring] + bounds[ring + 1]) / 2.0;
            let hot = hover_cell == Some((wi, ring));
            p.text(c + *dir * r, Align2::CENTER_CENTER, format!("{}", s), FontId::proportional((side * 0.052).max(9.0)), if hot { WHITE } else { dim_txt });
        }
        // direction chevron in the hub ring
        let hub_hot = hover_cell.map_or(false, |h| h.0 == wi);
        chevron(&p, c + *dir * (r0 - 4.5), *dir, 3.2, if hub_hot { ACCENT } else { Color32::from_gray(90) });
    }
    // centre: home all (lit when every axis is homed)
    let all_homed = homed.iter().all(|h| *h);
    let centre_hot = hover.map_or(false, |h| (h - c).length() < bounds[0] - 9.0);
    let cfill = if all_homed { if centre_hot { Color32::from_rgb(130, 218, 255) } else { ACCENT } } else if centre_hot { Color32::from_rgb(92, 98, 110) } else { Color32::from_rgb(60, 65, 75) };
    let cink = if all_homed { Color32::from_rgb(4, 14, 22) } else { Color32::from_rgb(190, 196, 206) };
    p.circle_filled(c, bounds[0] - 9.0, cfill);
    crate::icons::house(&p, c - vec2(0.0, 3.0), 7.5, cink, cfill);
    p.text(c + vec2(0.0, 11.5), Align2::CENTER_CENTER, "ALL", FontId::monospace(8.5), cink);

    // corner badges (rounded squares, clear of the circle)
    let bs = (side * 0.16).clamp(30.0, 40.0);
    let m = 2.0;
    let badges: [(Rect, &str, Jog, bool); 3] = [
        (Rect::from_min_size(pad.left_top() + vec2(m, m), vec2(bs, bs)), "X", Jog::HomeX, homed[0]),
        (Rect::from_min_size(pad.right_top() + vec2(-bs - m, m), vec2(bs, bs)), "Y", Jog::HomeY, homed[1]),
        (Rect::from_min_size(pad.right_bottom() + vec2(-bs - m, -bs - m), vec2(bs, bs)), "Z", Jog::HomeZ, homed[2]),
    ];
    for (r, label, jog, lit) in badges {
        let hot = hover.map_or(false, |h| r.contains(h));
        corner_badge(&p, r, label, hot, lit);
        if hot && click.is_some() { action = Some(jog); }
    }

    // --- Z strip: four step segments above and below a motors-off button ---
    let seg_h = (zr.height() - zw - 28.0) / 8.0;
    let mid = zr.center().y;
    let top0 = zr.top() + 14.0;
    let z_cells: Vec<(Rect, f32, String)> = (0..8).map(|i| {
        if i < 4 {
            (Rect::from_min_size(egui::pos2(zr.left(), top0 + i as f32 * seg_h), vec2(zw, seg_h)), ZSTEPS[3 - i], format!("{}", ZSTEPS[3 - i]))
        } else {
            let k = i - 4;
            (Rect::from_min_size(egui::pos2(zr.left(), mid + zw / 2.0 + k as f32 * seg_h), vec2(zw, seg_h)), -ZSTEPS[k], format!("{}", ZSTEPS[k]))
        }
    }).collect();
    for (i, (r, delta, label)) in z_cells.iter().enumerate() {
        let hot = hover.map_or(false, |h| r.contains(h));
        let k = if i < 4 { 3 - i } else { i - 4 };
        let shade = 24 + (3 - k as u8) * 6;
        p.rect_filled(r.shrink2(vec2(0.0, 1.0)), 5.0, if hot { hot_fill } else { Color32::from_rgb(shade, shade + 1, shade + 4) });
        p.rect_stroke(r.shrink2(vec2(0.0, 1.0)), 5.0, if hot { Stroke::new(1.5_f32, ACCENT) } else { line }, StrokeKind::Inside);
        p.text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional((side * 0.05).max(9.0)), if hot { WHITE } else { dim_txt });
        if hot && click.is_some() { action = Some(Jog::Move(2, *delta)); }
    }
    p.text(egui::pos2(zr.center().x, zr.top() + 6.0), Align2::CENTER_CENTER, "Z+", FontId::monospace(10.5), DIM);
    p.text(egui::pos2(zr.center().x, zr.bottom() - 6.0), Align2::CENTER_CENTER, "Z-", FontId::monospace(10.5), DIM);
    let zc = egui::pos2(zr.center().x, mid);
    let off_hot = hover.map_or(false, |h| (h - zc).length() < zw / 2.0);
    p.circle_filled(zc, zw / 2.0 - 2.0, if off_hot { Color32::from_rgb(130, 218, 255) } else { ACCENT });
    crate::icons::glyph(&p, "motor", zc, Color32::from_rgb(4, 14, 22), 0.0);
    if off_hot && click.is_some() { action = Some(Jog::MotorsOff); }

    // click handling for the circular pad
    if let (Some(pt), None) = (click, action) {
        if pad.contains(pt) {
            let d = pt - c;
            let r = d.length();
            if r < bounds[0] - 6.0 {
                action = Some(Jog::HomeAll);
            } else if let Some((wedge, ring)) = cell_at(pt) {
                let (axis, sign, ..) = wedges[wedge];
                action = Some(Jog::Move(axis, sign * RINGS[ring]));
            }
        }
    }
    if let Some(h) = hover_cell {
        resp.on_hover_text(format!("{} {} mm", wedges[h.0].5, RINGS[h.1]));
    } else if hover.map_or(false, |h| (h - c).length() < bounds[0] - 6.0) {
        resp.on_hover_text("Home all axes");
    }
    action
}

// ---------------------------------------------------------------------------------------------
// Spool illustration

pub fn filament_rgb(name: &str) -> Color32 {
    match name.to_lowercase().as_str() {
        "black" => Color32::from_rgb(34, 36, 40),
        "white" => Color32::from_rgb(232, 232, 228),
        "grey" | "gray" => Color32::from_rgb(132, 136, 142),
        "red" => Color32::from_rgb(214, 52, 52),
        "natural" => Color32::from_rgb(228, 210, 168),
        "galaxy black" => Color32::from_rgb(52, 40, 92),
        "blue" => Color32::from_rgb(46, 104, 226),
        "orange" => Color32::from_rgb(240, 132, 32),
        "green" => Color32::from_rgb(52, 178, 92),
        "yellow" => Color32::from_rgb(244, 208, 48),
        _ => Color32::from_rgb(150, 154, 160),
    }
}

pub fn on_color(c: Color32) -> Color32 {
    let l = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if l > 150.0 { Color32::from_rgb(12, 14, 18) } else { Color32::WHITE }
}

/// A spool seen from the front: dark flanges, filament wound to `frac` of the capacity, hub and spokes.
pub fn draw_spool(p: &egui::Painter, c: egui::Pos2, r: f32, col: Color32, frac: f32) {
    p.circle_filled(c, r, Color32::from_rgb(26, 28, 33));
    p.circle_stroke(c, r, Stroke::new(1.5_f32, Color32::from_rgb(70, 74, 84)));
    let r_hub = r * 0.30;
    let r_max = r * 0.90;
    let r_w = r_hub + (r_max - r_hub) * frac.clamp(0.04, 1.0).sqrt();
    p.circle_filled(c, r_w, col);
    // winding texture: concentric darker / lighter rings
    let ring = |k: f32| Color32::from_rgba_unmultiplied(0, 0, 0, (28.0 + 20.0 * k) as u8);
    let mut rad = r_hub + 3.0;
    let mut i = 0;
    while rad < r_w - 1.0 {
        p.circle_stroke(c, rad, Stroke::new(1.0_f32, if i % 2 == 0 { ring(0.0) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 16) }));
        rad += 3.2;
        i += 1;
    }
    p.circle_stroke(c, r_w, Stroke::new(1.5_f32, if on_color(col) == Color32::WHITE { Color32::from_rgba_unmultiplied(255, 255, 255, 60) } else { Color32::from_rgba_unmultiplied(0, 0, 0, 90) }));
    // hub with spoke cut-outs
    p.circle_filled(c, r_hub, Color32::from_rgb(40, 43, 50));
    for k in 0..3 {
        let a = k as f32 * std::f32::consts::TAU / 3.0 - std::f32::consts::FRAC_PI_2;
        let cc = c + vec2(a.cos(), a.sin()) * r_hub * 0.58;
        p.circle_filled(cc, r_hub * 0.26, Color32::from_rgb(12, 13, 16));
    }
    p.circle_filled(c, r_hub * 0.2, Color32::from_rgb(12, 13, 16));
    p.circle_stroke(c, r_hub, Stroke::new(1.0_f32, Color32::from_rgb(70, 74, 84)));
    // highlight on the flange
    p.add(Shape::line((0..14).map(|k| { let a = -2.5 + k as f32 * 0.07; c + vec2(a.cos(), a.sin()) * (r - 3.0) }).collect(), Stroke::new(2.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 26))));
}

/// Info chip tinted with the filament colour.
pub fn color_chip(ui: &mut Ui, text: &str, col: Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::monospace(12.0), on_color(col));
    let size = galley.size() + vec2(18.0, 8.0);
    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(r, 11.0, col);
    ui.painter().rect_stroke(r, 11.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 50)), StrokeKind::Inside);
    ui.painter().galley(r.center() - galley.size() / 2.0, galley, on_color(col));
}

/// Inline position field: label on the left, the live value on the right and the
/// commanded value in small brackets in the top-right corner.
pub fn axis_field(ui: &mut Ui, w: f32, label: &str, actual: f32, target: f32, dec: usize) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 54.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 9.0, CARD);
    p.rect_stroke(r, 9.0, Stroke::new(1.0_f32, CARD_LINE), StrokeKind::Inside);
    p.text(r.left_center() + vec2(14.0, 4.0), Align2::LEFT_CENTER, label, FontId::proportional(15.0), Color32::from_gray(150));
    p.text(r.right_center() + vec2(-14.0, 6.0), Align2::RIGHT_CENTER, format!("{:.*}", dec, actual), FontId::monospace(17.0), WHITE);
    p.text(r.right_top() + vec2(-12.0, 8.0), Align2::RIGHT_CENTER, format!("[{:.*}]", dec, target), FontId::monospace(10.0), Color32::from_gray(120));
}

// ---------------------------------------------------------------------------------------------
// G-code thumbnails: stand-ins for the preview image a slicer embeds in the file.

fn shade(c: Color32, f: f32) -> Color32 {
    Color32::from_rgb((c.r() as f32 * f).min(255.0) as u8, (c.g() as f32 * f).min(255.0) as u8, (c.b() as f32 * f).min(255.0) as u8)
}

pub fn name_hash(s: &str) -> u32 {
    s.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

/// Isometric-looking block: front, right side and top faces.
fn iso_box(p: &egui::Painter, o: egui::Pos2, w: f32, h: f32, d: f32, col: Color32) {
    let top = vec![o + vec2(0.0, -h), o + vec2(w, -h), o + vec2(w + d, -h - d * 0.55), o + vec2(d, -h - d * 0.55)];
    let side = vec![o + vec2(w, 0.0), o + vec2(w + d, -d * 0.55), o + vec2(w + d, -h - d * 0.55), o + vec2(w, -h)];
    p.add(Shape::convex_polygon(vec![o, o + vec2(w, 0.0), o + vec2(w, -h), o + vec2(0.0, -h)], shade(col, 0.78), Stroke::NONE));
    p.add(Shape::convex_polygon(side, shade(col, 0.52), Stroke::NONE));
    p.add(Shape::convex_polygon(top, shade(col, 1.05), Stroke::NONE));
}

fn gear(p: &egui::Painter, c: egui::Pos2, r: f32, teeth: usize, col: Color32) {
    let n = teeth * 2;
    let pts: Vec<egui::Pos2> = (0..n * 2).map(|k| { let a = k as f32 / (n * 2) as f32 * std::f32::consts::TAU; let rr = if (k / 2) % 2 == 0 { r } else { r * 0.8 }; c + vec2(a.cos(), a.sin()) * rr }).collect();
    p.add(Shape::convex_polygon(pts, shade(col, 0.9), Stroke::NONE));
    p.circle_filled(c, r * 0.55, shade(col, 0.7));
    p.circle_filled(c, r * 0.22, Color32::from_rgb(14, 15, 18));
}

pub fn draw_thumb(p: &egui::Painter, rect: Rect, seed: u32, col: Color32) {
    // very dark filament (black) would vanish on the dark preview: lift it
    let lum = 0.299 * col.r() as f32 + 0.587 * col.g() as f32 + 0.114 * col.b() as f32;
    let col = if lum < 70.0 { Color32::from_rgb(78, 82, 92) } else { col };
    p.rect_filled(rect, 9.0, Color32::from_rgb(16, 18, 23));
    // faint print plate
    let pl = Rect::from_min_max(egui::pos2(rect.left() + rect.width() * 0.08, rect.bottom() - rect.height() * 0.22), egui::pos2(rect.right() - rect.width() * 0.08, rect.bottom() - rect.height() * 0.1));
    p.rect_filled(pl, 4.0, Color32::from_rgb(34, 38, 46));
    let s = rect.width().min(rect.height());
    let base = egui::pos2(rect.left() + rect.width() * 0.5, rect.bottom() - rect.height() * 0.17);
    match seed % 6 {
        0 => {
            // housing: box with a pocket
            iso_box(&p.clone(), base + vec2(-s * 0.3, 0.0), s * 0.46, s * 0.34, s * 0.24, col);
            p.add(Shape::convex_polygon(vec![base + vec2(-s * 0.2, -s * 0.34 - s * 0.03), base + vec2(s * 0.08, -s * 0.34 - s * 0.03), base + vec2(s * 0.16, -s * 0.34 - s * 0.1), base + vec2(-s * 0.12, -s * 0.34 - s * 0.1)], shade(col, 0.45), Stroke::NONE));
        }
        1 => {
            // boat: hull, cabin, funnel
            let hull = vec![base + vec2(-s * 0.34, -s * 0.2), base + vec2(s * 0.34, -s * 0.2), base + vec2(s * 0.24, 0.0), base + vec2(-s * 0.26, 0.0)];
            p.add(Shape::convex_polygon(hull, shade(col, 0.8), Stroke::NONE));
            iso_box(p, base + vec2(-s * 0.14, -s * 0.2), s * 0.26, s * 0.2, s * 0.12, col);
            p.rect_filled(Rect::from_min_size(base + vec2(s * 0.02, -s * 0.52), vec2(s * 0.08, s * 0.14)), 2.0, shade(col, 0.6));
        }
        2 => {
            // flat panel with holes
            iso_box(p, base + vec2(-s * 0.34, 0.0), s * 0.62, s * 0.1, s * 0.3, col);
            for k in 0..3 { p.circle_filled(base + vec2(-s * 0.2 + k as f32 * s * 0.2, -s * 0.2), s * 0.035, Color32::from_rgb(16, 18, 23)); }
        }
        3 => {
            // meshed gears
            gear(p, base + vec2(-s * 0.14, -s * 0.28), s * 0.24, 8, col);
            gear(p, base + vec2(s * 0.19, -s * 0.2), s * 0.16, 6, shade(col, 0.85));
        }
        4 => {
            // calibration tower: stacked, narrowing blocks
            for k in 0..5 {
                let w = s * (0.34 - k as f32 * 0.04);
                iso_box(p, base + vec2(-w / 2.0 - s * 0.04, -(k as f32) * s * 0.11), w, s * 0.11, s * 0.1, shade(col, 1.0 - k as f32 * 0.05));
            }
        }
        _ => {
            // multi-colour set
            let cols = [col, shade(col, 0.6), Color32::from_rgb(220, 224, 230)];
            for (k, cc) in cols.iter().enumerate() {
                iso_box(p, base + vec2(-s * 0.34 + k as f32 * s * 0.24, 0.0), s * 0.18, s * (0.18 + 0.06 * k as f32), s * 0.12, *cc);
            }
        }
    }
    p.rect_stroke(rect, 9.0, Stroke::new(1.0_f32, CARD_LINE), StrokeKind::Inside);
}

pub fn thumb(ui: &mut Ui, size: f32, name: &str, col: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    draw_thumb(&ui.painter_at(r), r, name_hash(name), col);
}

/// Option in a filter row: plain text when off, tinted when chosen (never solid).
pub fn choice_btn(ui: &mut Ui, selected: bool, t: &str) -> bool {
    with_button_style(ui, if selected { BK::Selected } else { BK::Ghost }, |ui| {
        ui.spacing_mut().button_padding = vec2(11.0, 3.0);
        ui.add(Button::new(RichText::new(t).size(12.5)).min_size(vec2(0.0, 26.0))).clicked()
    })
}
