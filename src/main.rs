// Release builds on Windows run as a GUI app (no console window).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod data;
mod pages;
mod theme;
mod ui;

use data::{Model, Page};
use eframe::egui::{self, vec2, Align, Layout, RichText, Sense};
use theme::*;

struct App {
    m: Model,
    page: Page,
    nav_open: bool,
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("MACH5 Management Interface"),
        ..Default::default()
    };
    eframe::run_native(
        "MACH5",
        opts,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            // Optional: `mach5-control <page-key|title>` opens straight to a page.
            let arg = std::env::args().nth(1).unwrap_or_default().to_lowercase();
            let page = Page::GROUPS
                .iter()
                .flat_map(|(_, ps)| ps.iter())
                .find(|p| p.key() == arg || p.title().to_lowercase() == arg)
                .copied()
                .unwrap_or(Page::Dashboard);
            let mut m = Model::new();
            // Optional extra args pre-select tabs: `mach5-control calibration 3 0`
            m.st.tab = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            m.st.tab2 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(0);
            Ok(Box::new(App { m, page, nav_open: true }))
        }),
    )
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt).min(0.2) as f64;
        self.m.tick(dt);
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        if let Some(p) = self.m.goto.take() {
            self.page = p;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::B)) {
            self.nav_open = !self.nav_open;
        }

        paint_background(ctx);
        self.top_bar(ctx);
        egui::SidePanel::left("nav")
            .exact_width(228.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(PANEL).stroke(egui::Stroke::new(1.0_f32, LINE)).inner_margin(egui::Margin::symmetric(0, 8)))
            .show_animated(ctx, self.nav_open, |ui| self.nav(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(18, 14)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let w = ui.available_width().min(1500.0) - 10.0;
                    ui.set_max_width(w);
                    pages::show(ui, self.page, &mut self.m);
                });
            });
    }
}

impl App {
    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top")
            .exact_height(40.0)
            .frame(egui::Frame::new().fill(PANEL).stroke(egui::Stroke::new(1.0_f32, LINE)).inner_margin(egui::Margin::symmetric(10, 6)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let icon = if self.nav_open { "<<" } else { "=" };
                    if ui.add(egui::Button::new(RichText::new(icon).monospace().strong()).min_size(vec2(30.0, 26.0))).on_hover_text("Toggle navigation (Ctrl+B)").clicked() {
                        self.nav_open = !self.nav_open;
                    }
                    ui.label(RichText::new("MACH5").monospace().strong().size(15.0).color(WHITE));
                    ui.label(ui::dim("/"));
                    ui.label(RichText::new(self.page.title()).color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui::danger(ui, "E-STOP") {
                            self.m.estop();
                        }
                        ui.add_space(10.0);
                        ui.label(ui::mono("API v1.4.2 signed").color(DIM));
                        ui.add_space(10.0);
                        ui::status(ui, self.m.state.status(), self.m.state.label());
                        ui.add_space(10.0);
                        ui.label(ui::mono(format!("{}  10.0.4.21", self.m.name)));
                    });
                });
            });
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            ui.label(RichText::new("MACH5").monospace().strong().size(17.0).color(WHITE));
            ui.label(ui::dim("control"));
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(ui.available_height() - 64.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (group, pages) in Page::GROUPS {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(RichText::new(group.to_uppercase()).monospace().size(10.5).color(FAINT));
                });
                for p in pages.iter() {
                    let sel = self.page == *p;
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
                    let fill = if sel { C_SEL } else if resp.hovered() { RAISED } else { egui::Color32::TRANSPARENT };
                    ui.painter().rect_filled(rect.shrink2(vec2(6.0, 0.0)), crate::theme::RAD, fill);
                    let col = if sel { WHITE } else { TEXT };
                    ui.painter().text(rect.left_center() + vec2(16.0, 0.0), egui::Align2::LEFT_CENTER, p.key(), egui::FontId::monospace(10.5), FAINT);
                    ui.painter().text(rect.left_center() + vec2(42.0, 0.0), egui::Align2::LEFT_CENTER, p.title(), egui::FontId::proportional(13.5), col);
                    if let Some(a) = nav_alert(&self.m, *p) {
                        let (st, _) = a;
                        ui.painter().rect_filled(egui::Rect::from_center_size(rect.right_center() - vec2(20.0, 0.0), vec2(7.0, 7.0)), crate::theme::RAD, st.color());
                    }
                    if resp.clicked() {
                        self.page = *p;
                    }
                }
            }
        });
        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                if ui.button("Close nav").clicked() {
                    self.nav_open = false;
                }
                ui.label(ui::dim("Ctrl+B"));
            });
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(ui::dim("Prototype: mock data"));
            });
        });
    }
}

const C_SEL: egui::Color32 = egui::Color32::from_rgb(44, 44, 44);

/// Small status marker next to nav entries that need attention.
fn nav_alert(m: &Model, p: Page) -> Option<(Status, &'static str)> {
    match p {
        Page::Nodes | Page::Firmware if m.nodes.iter().any(|n| n.fw != n.want) => Some((Status::Warn, "out of date")),
        Page::Nodes if m.nodes.iter().any(|n| n.status == Status::Err) => Some((Status::Err, "fault")),
        Page::Dashboard if m.state == data::MState::Fault => Some((Status::Err, "fault")),
        _ => None,
    }
}

const GLYPHS: [[&str; 7]; 5] = [
    ["10001", "11011", "10101", "10101", "10001", "10001", "10001"], // M
    ["01110", "10001", "10001", "11111", "10001", "10001", "10001"], // A
    ["01111", "10000", "10000", "10000", "10000", "10000", "01111"], // C
    ["10001", "10001", "10001", "11111", "10001", "10001", "10001"], // H
    ["11111", "10000", "11110", "00001", "00001", "10001", "01110"], // 5
];
const RAMP: [&str; 10] = [".", ":", "-", "=", "+", "*", "x", "#", "%", "@"];
const FIELD: [&str; 6] = [".", ".", ":", "'", "`", "+"];

fn hash(x: i32, y: i32, s: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ s.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h
}
fn unit(h: u32) -> f32 {
    (h & 0xFFFF) as f32 / 65535.0
}

/// Dark ASCII field with a MACH5 logo. Every few seconds something fast flies
/// past and leaves a rippling, fading wake of turbulence in the characters.
fn paint_background(ctx: &egui::Context) {
    let t = ctx.input(|i| i.time) as f32;
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(screen, 0.0, BG);

    let (cw, ch) = (13.0_f32, 17.0_f32);
    let cols = (screen.width() / cw).ceil() as i32;
    let rows = (screen.height() / ch).ceil() as i32;
    let font = egui::FontId::monospace(12.0);

    // Logo placement (in cells).
    let wide = screen.width() >= 1250.0;
    let (sx, sy) = if wide { (3, 2) } else { (2, 1) };
    let logo_w = 5 * 5 * sx + 4 * sx;
    let logo_h = 7 * sy;
    let (lc, lr) = ((cols - logo_w) / 2, (rows - logo_h) / 2);
    let in_logo = |c: i32, r: i32| -> bool {
        let (x, y) = (c - lc, r - lr);
        if x < 0 || y < 0 || x >= logo_w || y >= logo_h {
            return false;
        }
        let px = x / sx;
        let g = px / 6;
        if g > 4 || px % 6 == 5 {
            return false;
        }
        GLYPHS[g as usize][(y / sy) as usize].as_bytes()[(px % 6) as usize] == b'1'
    };

    // Fly-bys: event n starts at n * PERIOD, crosses the screen in about a second.
    const PERIOD: f32 = 6.5;
    const LIFE: f32 = 8.0;
    struct Fly { start: egui::Pos2, dir: egui::Vec2, speed: f32, tau: f32 }
    let mut flies: Vec<Fly> = Vec::new();
    let n_now = (t / PERIOD).floor() as i32;
    for n in (n_now - 1).max(0)..=n_now {
        let tau = t - n as f32 * PERIOD;
        if !(0.0..LIFE).contains(&tau) {
            continue;
        }
        let h = hash(n, 7, 11);
        let from_left = h & 1 == 0;
        let y0 = screen.height() * (0.12 + 0.76 * unit(hash(n, 1, 3)));
        let slope = (unit(hash(n, 2, 5)) - 0.5) * 0.9;
        let dir = egui::vec2(if from_left { 1.0 } else { -1.0 }, slope).normalized();
        let x0 = if from_left { -120.0 } else { screen.width() + 120.0 };
        flies.push(Fly { start: egui::pos2(x0, y0), dir, speed: 1500.0, tau });
    }

    let frame = (t * 14.0) as i32;
    for r in 0..rows {
        for c in 0..cols {
            let p = egui::pos2(c as f32 * cw + cw * 0.5, r as f32 * ch + ch * 0.5);
            let mut b = 0.0_f32;
            for f in &flies {
                let rel = p - f.start;
                let s = rel.dot(f.dir);
                let d = (rel.x * f.dir.y - rel.y * f.dir.x).abs();
                let tpass = s / f.speed;
                if tpass > f.tau {
                    continue;
                }
                if tpass < 0.0 {
                    continue;
                }
                let age = f.tau - tpass;
                let env = (-age / 2.4).exp() * (-(d / (70.0 + age * 110.0)).powi(2)).exp();
                let ring = 0.5 + 0.5 * (d / 15.0 - age * 7.0).sin();
                b += env * (0.25 + 0.75 * ring);
                // Bright head just behind the object itself.
                if age < 0.06 && d < 26.0 {
                    b += 1.0 - d / 26.0;
                }
            }
            let logo = in_logo(c, r);
            let noise = unit(hash(c, r, 1));
            b = (b * (0.75 + 0.25 * unit(hash(c, r, frame as u32)))).clamp(0.0, 1.0);
            let breathe = 0.5 + 0.5 * (t * 0.35 + c as f32 * 0.05).sin();
            let base = if logo { 46.0 + 12.0 * breathe } else { 9.0 + 5.0 * noise };
            let a = (base + b * 130.0).min(150.0);
            if a < 7.0 {
                continue;
            }
            let a = ((a / 8.0).round() * 8.0) as u8;
            let ch_str = if b > 0.07 {
                RAMP[((b * 9.0 + noise * 2.0) as usize).min(9)]
            } else if logo {
                "#"
            } else {
                FIELD[(hash(c, r, 2) % 6) as usize]
            };
            painter.text(p, egui::Align2::CENTER_CENTER, ch_str, font.clone(), egui::Color32::from_white_alpha(a));
        }
    }
}
