// Release builds on Windows run as a GUI app (no console window).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod builder;
mod data;
mod icons;
mod fleetsim;
mod pages;
mod spools;
mod theme;
mod tour;
mod ui;

use data::{Model, Page};
use eframe::egui::{self, vec2, Align, Color32, Layout, RichText, Sense};
use theme::*;

struct App {
    m: Model,
    page: Page,
    nav_open: bool,
    tour: Option<tour::Tour>,
}

/// Window / taskbar icon, decoded from assets/icon.png.
fn app_icon() -> egui::IconData {
    match image::load_from_memory(include_bytes!("../assets/icon.png")) {
        Ok(img) => {
            let img = img.to_rgba8();
            let (width, height) = img.dimensions();
            egui::IconData { rgba: img.into_raw(), width, height }
        }
        Err(_) => egui::IconData::default(),
    }
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(std::env::var("MACH5_SIZE").ok().and_then(|s| { let (w, h) = s.split_once('x')?; Some([w.parse().ok()?, h.parse().ok()?]) }).unwrap_or([1440.0, 900.0]))
            .with_min_inner_size([960.0, 600.0])
            .with_icon(app_icon())
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
            m.st.farm_tab = m.st.tab;
            if std::env::var("MACH5_EXPAND").is_ok() {
                m.bld.force_open = Some(true);
            }
            m.st.tab2 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(0);
            let tour = if std::env::args().nth(1).as_deref() == Some("tour") {
                Some(tour::Tour::new(std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(0), page, true))
            } else {
                None
            };
            Ok(Box::new(App { m, page, nav_open: true, tour }))
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
        // Pages that live under a hub open as a tab of that hub.
        let hub = self.page.hub();
        if self.page != hub {
            self.m.hub_sel.insert(hub, self.page);
            self.page = hub;
        }

        paint_background(ctx);
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) && self.tour.is_none() {
            self.tour = Some(tour::Tour::new(0, self.page, self.nav_open));
        }
        self.top_bar(ctx);
        // The sidebar is always present: full width normally, an icon rail on narrow windows.
        let rail = ctx.screen_rect().width() < 1200.0;
        egui::SidePanel::left("nav")
            .exact_width(if rail { 64.0 } else { 228.0 })
            .resizable(false)
            .frame(egui::Frame::new().fill(PANEL).stroke(egui::Stroke::new(1.0_f32, LINE)).inner_margin(egui::Margin::symmetric(0, 8)))
            .show(ctx, |ui| self.nav(ui, rail));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(18, 14)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().id_salt(self.page as u8).auto_shrink([false, false]).show(ui, |ui| {
                    let w = ui.available_width().min(1500.0) - 10.0;
                    ui.set_max_width(w);
                    pages::show(ui, self.page, &mut self.m);
                    if std::env::var("MACH5_DEBUG").is_ok() && ui.min_rect().width() > w + 2.0 {
                        eprintln!("OVERFLOW page={:?} tab={} used={:.0} avail={:.0}", self.page, self.m.st.tab, ui.min_rect().width(), w);
                    }
                });
            });
        if let Some(mut tour) = self.tour.take() {
            if tour.show(ctx, &mut self.m, &mut self.page, &mut self.nav_open) {
                self.tour = Some(tour);
            }
        }
    }
}

impl App {
    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top")
            .exact_height(40.0)
            .frame(egui::Frame::new().fill(PANEL).stroke(egui::Stroke::new(1.0_f32, LINE)).inner_margin(egui::Margin::symmetric(10, 6)))
            .show(ctx, |ui| {
                if let Some(s) = ui::frost_shape(ctx, ui.max_rect()) {
                    ui.painter().add(s);
                }
                ui.horizontal_centered(|ui| {
                    let narrow = ctx.screen_rect().width() < 1150.0;
                    ui.label(RichText::new("MACH5").monospace().strong().size(15.0).color(WHITE));
                    ui.label(ui::dim("/"));
                    ui.label(RichText::new(self.page.nav_title()).color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui::danger(ui, "E-STOP") {
                            self.m.estop();
                        }
                        ui.add_space(10.0);
                        if !narrow {
                            ui.label(ui::mono("API v1.4.2 signed").color(DIM));
                            ui.add_space(10.0);
                        }
                        ui::status(ui, self.m.state.status(), self.m.state.label());
                        ui.add_space(10.0);
                        let mut pick = None;
                        let cur = self.m.active;
                        let label = if narrow { self.m.fleet[cur].name.clone() } else { format!("{}  {}", self.m.fleet[cur].name, self.m.fleet[cur].ip) };
                        let cb = egui::ComboBox::from_id_salt("machine_switch").width(if narrow { 130.0 } else { 190.0 }).selected_text(label).show_ui(ui, |ui| {
                            for (i, f) in self.m.fleet.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui::dot(ui, f.state.status());
                                    if ui.selectable_label(i == cur, format!("{}  {}", f.name, f.state.label())).clicked() { pick = Some(i); }
                                });
                            }
                        });
                        ui::set_anchor(ctx, "switcher", cb.response.rect);
                        if let Some(i) = pick { self.m.switch_machine(i); }
                    });
                });
            });
    }

    fn nav(&mut self, ui: &mut egui::Ui, rail: bool) {
        ui::set_anchor(ui.ctx(), "nav", ui.max_rect());
        if let Some(s) = ui::frost_shape(ui.ctx(), ui.max_rect()) {
            ui.painter().add(s);
        }
        if rail {
            ui.vertical_centered(|ui| { ui.label(RichText::new("M5").monospace().strong().size(17.0).color(WHITE)); });
        } else {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(RichText::new("MACH5").monospace().strong().size(17.0).color(WHITE));
                ui.label(ui::dim("control"));
            });
        }
        ui.add_space(6.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(ui.available_height() - 64.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (group, pages) in Page::GROUPS {
                ui.add_space(8.0);
                if rail {
                    let y = ui.cursor().top();
                    ui.painter().line_segment([egui::pos2(ui.max_rect().left() + 14.0, y), egui::pos2(ui.max_rect().right() - 14.0, y)], egui::Stroke::new(1.0_f32, LINE));
                    ui.add_space(4.0);
                } else {
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        ui.label(RichText::new(group.to_uppercase()).monospace().size(10.5).color(FAINT));
                    });
                }
                for p in pages.iter() {
                    let sel = self.page == *p;
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
                    let fill = if sel { C_SEL } else if resp.hovered() { RAISED } else { egui::Color32::TRANSPARENT };
                    ui.painter().rect_filled(rect.shrink2(vec2(6.0, 0.0)), crate::theme::RAD, fill);
                    let col = if sel { WHITE } else { TEXT };
                    let icol = if sel { ACCENT } else if resp.hovered() { WHITE } else { DIM };
                    let resp = if rail { resp.on_hover_text(p.nav_title()) } else { resp };
                    if rail {
                        icons::draw(&ui.painter().clone(), *p, rect.center(), icol);
                    } else {
                        icons::draw(&ui.painter().clone(), *p, rect.left_center() + vec2(26.0, 0.0), icol);
                        ui.painter().text(rect.left_center() + vec2(46.0, 0.0), egui::Align2::LEFT_CENTER, p.nav_title(), egui::FontId::proportional(13.5), col);
                    }
                    if let Some(a) = nav_alert(&self.m, *p) {
                        let (st, _) = a;
                        let at = if rail { rect.right_top() + vec2(-12.0, 8.0) } else { rect.right_center() - vec2(20.0, 0.0) };
                        ui.painter().rect_filled(egui::Rect::from_center_size(at, vec2(7.0, 7.0)), crate::theme::RAD, st.color());
                    }
                    if resp.clicked() {
                        self.page = *p;
                    }
                }
            }
        });
        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.add_space(4.0);
            if rail {
                ui.vertical_centered(|ui| {
                    if ui::primary(ui, "?") {
                        self.tour = Some(tour::Tour::new(0, self.page, self.nav_open));
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    if ui::primary(ui, "Tutorial") {
                        self.tour = Some(tour::Tour::new(0, self.page, self.nav_open));
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(ui::dim("Prototype: mock data"));
                });
            }
        });
    }
}

const C_SEL: egui::Color32 = egui::Color32::from_rgb(44, 44, 44);

/// Small status marker next to nav entries that need attention.
fn nav_alert(m: &Model, p: Page) -> Option<(Status, &'static str)> {
    match p {
        Page::Firmware if m.nodes.iter().any(|n| n.fw != n.want) => Some((Status::Warn, "out of date")),
        Page::CanBus if m.nodes.iter().any(|n| n.status == Status::Err) => Some((Status::Err, "fault")),
        Page::CanBus if m.nodes.iter().any(|n| n.status == Status::Warn) => Some((Status::Warn, "node warning")),
        Page::Dashboard if m.state == data::MState::Fault => Some((Status::Err, "fault")),
        Page::Fleet if m.alerts.iter().any(|a| !a.ack && a.sev == data::Sev::Err) => Some((Status::Err, "alerts")),
        Page::Fleet if m.alerts.iter().any(|a| !a.ack && a.sev == data::Sev::Warn) => Some((Status::Warn, "alerts")),
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
const DIGITS: [&str; 10] = [".", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

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

/// Dark screen. Characters exist only inside turbulence: every ~24 s something fast flies by
/// and its wake lights up digits (peaking at 9) that ripple outward and fade. Cells that make
/// up the MACH5 logo linger much longer, so the logo is revealed as the wake crosses it.
fn paint_background(ctx: &egui::Context) {
    let t = ctx.input(|i| i.time) as f32;
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(screen, 0.0, BG);

    let (cw, ch) = (13.0_f32, 17.0_f32);
    let cols = (screen.width() / cw).ceil() as i32;
    let rows = (screen.height() / ch).ceil() as i32;
    let font = egui::FontId::monospace(12.0);

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

    // Fly-bys: first one shortly after launch, then one every PERIOD seconds.
    const PERIOD: f32 = 24.0;
    const FIRST: f32 = 3.0;
    const LIFE: f32 = 18.0;
    struct Fly { start: egui::Pos2, dir: egui::Vec2, speed: f32, tau: f32 }
    let mut flies: Vec<Fly> = Vec::new();
    let n_now = ((t - FIRST) / PERIOD).floor() as i32;
    for n in (n_now - 1).max(0)..=n_now {
        let tau = t - (FIRST + n as f32 * PERIOD);
        if !(0.0..LIFE).contains(&tau) {
            continue;
        }
        let h = hash(n, 7, 11);
        let from_left = h & 1 == 0;
        let y0 = screen.height() * (0.18 + 0.64 * unit(hash(n, 1, 3)));
        let slope = (unit(hash(n, 2, 5)) - 0.5) * 0.7;
        let dir = egui::vec2(if from_left { 1.0 } else { -1.0 }, slope).normalized();
        let x0 = if from_left { -120.0 } else { screen.width() + 120.0 };
        flies.push(Fly { start: egui::pos2(x0, y0), dir, speed: 1500.0, tau });
    }
    if flies.is_empty() {
        ui::set_frost(ctx, None);
        return;
    }
    let mut grid = vec![0.0_f32; (cols * rows) as usize];

    let frame = (t * 14.0) as i32;
    for r in 0..rows {
        for c in 0..cols {
            let p = egui::pos2(c as f32 * cw + cw * 0.5, r as f32 * ch + ch * 0.5);
            let logo = in_logo(c, r);
            let decay = if logo { 7.0 } else { 1.8 };
            let mut b = 0.0_f32;
            for f in &flies {
                let rel = p - f.start;
                let s = rel.dot(f.dir);
                let tpass = s / f.speed;
                if tpass < 0.0 || tpass > f.tau {
                    continue;
                }
                let d = (rel.x * f.dir.y - rel.y * f.dir.x).abs();
                let age = f.tau - tpass;
                let env = (-age / decay).exp() * (-(d / (46.0 + age * 70.0)).powi(2)).exp();
                let ring = 0.5 + 0.5 * (d / 15.0 - age * 7.0).sin();
                b += env * (0.25 + 0.75 * ring);
                if age < 0.06 && d < 26.0 {
                    b += 1.0 - d / 26.0;
                }
            }
            if logo {
                b *= 1.5;
            }
            b = (b * (0.78 + 0.22 * unit(hash(c, r, frame as u32)))).clamp(0.0, 1.0);
            grid[(r * cols + c) as usize] = b;
            if b < 0.07 {
                continue;
            }
            let idx = ((b * 9.0 + unit(hash(c, r, 1)) * 1.6) as usize).min(9);
            let s = if logo && b > 0.2 { "9" } else { DIGITS[idx] };
            let a = (24.0 + b * 80.0) as u8;
            let col = if idx == 9 || (logo && b > 0.2) { Color32::from_rgba_unmultiplied(ACCENT.r(), ACCENT.g(), ACCENT.b(), a) } else { Color32::from_white_alpha(a) };
            painter.text(p, egui::Align2::CENTER_CENTER, s, font.clone(), col);
        }
    }

    // Coarse blurred copy of the wake for frosted-glass surfaces (3x3 cells per block, then a box blur).
    const K: i32 = 3;
    let (ccols, crows) = ((cols + K - 1) / K, (rows + K - 1) / K);
    let mut coarse = vec![0.0_f32; (ccols * crows) as usize];
    for cj in 0..crows {
        for ci in 0..ccols {
            let mut s = 0.0;
            for dy in 0..K {
                for dx in 0..K {
                    let (c, r) = (ci * K + dx, cj * K + dy);
                    if c < cols && r < rows {
                        s += grid[(r * cols + c) as usize];
                    }
                }
            }
            coarse[(cj * ccols + ci) as usize] = (s / (K * K) as f32 * 2.2).min(1.0);
        }
    }
    let mut blurred = coarse.clone();
    for cj in 0..crows {
        for ci in 0..ccols {
            let (mut s, mut n) = (0.0, 0.0);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (x, y) = (ci + dx, cj + dy);
                    if x >= 0 && y >= 0 && x < ccols && y < crows {
                        s += coarse[(y * ccols + x) as usize];
                        n += 1.0;
                    }
                }
            }
            blurred[(cj * ccols + ci) as usize] = s / n;
        }
    }
    ui::set_frost(ctx, Some(ui::FrostField { cw: cw * K as f32, ch: ch * K as f32, cols: ccols as usize, rows: crows as usize, v: blurred }));
}
