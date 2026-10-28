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

/// Soft, out-of-focus greyscale glows drifting slowly behind every page.
fn paint_background(ctx: &egui::Context) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    let t = ctx.input(|i| i.time) as f32;
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(screen, 0.0, BG);
    // (phase x, phase y, speed, radius as fraction of the larger side, alpha)
    let blobs: [(f32, f32, f32, f32, u8); 5] = [
        (0.0, 1.3, 0.050, 0.55, 18),
        (2.1, 0.4, 0.037, 0.45, 14),
        (4.2, 2.6, 0.061, 0.60, 12),
        (1.0, 5.1, 0.043, 0.38, 15),
        (3.3, 3.9, 0.029, 0.50, 11),
    ];
    let big = screen.width().max(screen.height());
    for (px, py, sp, rad, a) in blobs {
        let cx = screen.center().x + (t * sp * 6.0 + px).sin() * screen.width() * 0.42;
        let cy = screen.center().y + (t * sp * 5.0 + py).cos() * screen.height() * 0.42;
        let r = big * rad;
        let mut mesh = Mesh::default();
        mesh.vertices.push(Vertex { pos: egui::pos2(cx, cy), uv: WHITE_UV, color: egui::Color32::from_white_alpha(a) });
        const N: u32 = 40;
        for k in 0..N {
            let ang = k as f32 / N as f32 * std::f32::consts::TAU;
            mesh.vertices.push(Vertex { pos: egui::pos2(cx + ang.cos() * r, cy + ang.sin() * r), uv: WHITE_UV, color: egui::Color32::TRANSPARENT });
        }
        for k in 0..N {
            mesh.indices.extend_from_slice(&[0, 1 + k, 1 + (k + 1) % N]);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
}
