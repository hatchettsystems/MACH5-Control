//! Dashboard: a uniform grid of equal-height cards (two columns, one on narrow windows).
use super::control::{run_cmd, state_actions, SHAPERS};
use super::farm::cam_tile;
use crate::data::*;
use crate::fleetsim::eta;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, FontId, Frame, Layout, Margin, RichText, Sense, Stroke, Ui};

fn clock(secs: f32) -> String {
    let s = secs as u32 % 86400;
    format!("{:02}:{:02}", s / 3600, (s / 60) % 60)
}

/// Small bordered row used for lists. `right` is a short fixed-width value; the left text truncates.
fn mini(ui: &mut Ui, w: f32, left: impl FnOnce(&mut Ui), right: &str) {
    Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32, CARD_LINE)).corner_radius(7).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
        ui.set_width(w - 22.0);
        ui.horizontal(|ui| {
            let right_w = 52.0;
            ui.allocate_ui_with_layout(vec2((w - 22.0 - right_w - 6.0).max(20.0), 18.0), Layout::left_to_right(Align::Center), |ui| {
                ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                left(ui);
            });
            ui.allocate_ui_with_layout(vec2(right_w, 18.0), Layout::right_to_left(Align::Center), |ui| {
                ui.label(mono(right.to_string()).size(11.5).color(DIM));
            });
        });
    });
}


/// (title, span) per dashboard card id; the order shown is `Model::dash_order`.
const CARD_INFO: [(&str, Span); DASH_CARDS] = [
    ("Printer status", Span::Half), ("Camera", Span::Half), ("Temperatures", Span::Half), ("Movement", Span::Half),
    ("Print parameters", Span::Half), ("Filament", Span::Quarter), ("Power and system", Span::Quarter), ("Fans and outputs", Span::Half),
    ("Motion system", Span::Half), ("Bed mesh", Span::Half), ("Node health", Span::Half), ("Console", Span::Full),
    ("Print history", Span::Half), ("Recent events", Span::Half),
];

pub fn dashboard(ui: &mut Ui, m: &mut Model) {
    title(ui, "Dashboard", &format!("{} - live telemetry, mock data", m.name));
    ui.horizontal_wrapped(|ui| {
        ui.label(dim("Drag a card by its header to rearrange. Cards pack together automatically and the layout is saved."));
        if sbtn(ui, "Reset layout").clicked() { m.dash_order = default_dash_order(); save_dash_order(&m.dash_order); }
    });
    ui.add_space(6.0);
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("card_rects"), Vec::<(u8, egui::Rect)>::new()));
    let items: Vec<(u8, Span)> = m.dash_order.iter().map(|&id| (id, CARD_INFO[id as usize].1)).collect();
    masonry(ui, &items, |ui, id| {
        set_card(ui.ctx(), id);
        match id {
            0 => p_status(ui, m, 0.0),
            1 => p_camera(ui, m, 0.0),
            2 => p_temps(ui, m),
            3 => p_move(ui, m),
            4 => p_params(ui, m),
            5 => p_filament(ui, m, 0.0),
            6 => p_power(ui, m, 0.0),
            7 => p_fans(ui, m, 0.0),
            8 => p_motion(ui, m, 0.0),
            9 => p_mesh(ui, m, 0.0),
            10 => p_nodes(ui, m, 0.0),
            11 => p_console(ui, m, 520.0),
            12 => p_history(ui, m, 0.0),
            _ => p_events(ui, m, 0.0),
        }
    });
    handle_drag(ui, m);
}

/// Drop target for a card dragged to `p`: (target id, its rect, drop after it).
fn drop_target(rects: &[(u8, egui::Rect)], src: u8, p: egui::Pos2) -> Option<(u8, egui::Rect, bool)> {
    let best = rects.iter().filter(|(id, _)| *id != src).min_by(|a, b| {
        let d = |r: &egui::Rect| { let c = r.clamp(p); (c - p).length() };
        d(&a.1).partial_cmp(&d(&b.1)).unwrap()
    })?;
    let r = best.1;
    let left = rects.iter().map(|(_, r)| r.left()).fold(f32::MAX, f32::min);
    let right = rects.iter().map(|(_, r)| r.right()).fold(f32::MIN, f32::max);
    let full_width = r.width() > (right - left) * 0.9;
    let in_x = p.x >= r.left() && p.x <= r.right();
    let after = if full_width || in_x { p.y > r.center().y } else { p.x > r.center().x };
    Some((best.0, r, after))
}

fn handle_drag(ui: &mut Ui, m: &mut Model) {
    let ctx = ui.ctx().clone();
    let rects: Vec<(u8, egui::Rect)> = ctx.data(|d| d.get_temp(egui::Id::new("card_rects"))).unwrap_or_default();
    let drag: Option<u8> = ctx.data(|d| d.get_temp(egui::Id::new("dash_drag")));
    let drop: Option<(u8, egui::Pos2)> = ctx.data_mut(|d| d.remove_temp(egui::Id::new("drag_drop")));
    if let (Some(src), Some(p)) = (drag, ctx.input(|i| i.pointer.interact_pos())) {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drag_overlay")));
        if let Some((_, r, after)) = drop_target(&rects, src, p) {
            painter.rect_stroke(r, 8.0, Stroke::new(2.0_f32, ACCENT_DIM), egui::StrokeKind::Outside);
            let full = r.width() > ui.available_width() * 0.9;
            let bar = if full {
                let y = if after { r.bottom() + 4.0 } else { r.top() - 4.0 };
                egui::Rect::from_min_max(egui::pos2(r.left(), y - 2.0), egui::pos2(r.right(), y + 2.0))
            } else {
                let x = if after { r.right() + 4.0 } else { r.left() - 4.0 };
                egui::Rect::from_min_max(egui::pos2(x - 2.0, r.top()), egui::pos2(x + 2.0, r.bottom()))
            };
            painter.rect_filled(bar, 2.0, ACCENT);
        }
        let label = CARD_INFO[src as usize].0;
        let box_r = egui::Rect::from_min_size(p + vec2(14.0, 10.0), vec2(150.0, 28.0));
        painter.rect_filled(box_r, 7.0, Color32::from_rgb(18, 40, 58));
        painter.rect_stroke(box_r, 7.0, Stroke::new(1.0_f32, ACCENT), egui::StrokeKind::Inside);
        painter.text(box_r.center(), egui::Align2::CENTER_CENTER, label, FontId::proportional(13.0), WHITE);
    }
    if let Some((src, p)) = drop {
        if let Some((tgt, _, after)) = drop_target(&rects, src, p) {
            m.dash_order.retain(|x| *x != src);
            if let Some(pos) = m.dash_order.iter().position(|x| *x == tgt) {
                m.dash_order.insert(if after { pos + 1 } else { pos }, src);
                save_dash_order(&m.dash_order);
            }
        }
    }
}

fn p_status(ui: &mut Ui, m: &mut Model, h: f32) {
    let t = m.t as f32;
    let printing = m.state == MState::Printing;
    let vel = if printing { 180.0 * m.job.speed / 100.0 * (0.55 + 0.45 * (t * 0.9).sin().abs()) } else { 0.0 };
    let accel = if printing { 3000.0 + 2500.0 * (t * 1.3).sin().abs() } else { 0.0 };
    let flow_rate = vel * 0.4 * 0.2 * m.job.flow / 100.0;
    let grams = m.job.progress * 312.0;
    let remaining = (m.job.total - m.job.elapsed).max(0.0);
    let now = 14.0 * 3600.0 + 32.0 * 60.0 + t;
    let busy = matches!(m.state, MState::Printing | MState::Paused);
    panel_min(ui, "Printer status", h, |ui| {
        let state_block = |ui: &mut Ui, m: &Model| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 8.0, m.state.status().color());
                ui.label(RichText::new(m.state.label().split(' ').next().unwrap_or("")).size(30.0).strong().color(WHITE));
            });
            ui.label(dim(if busy { m.job.file.clone() } else { "Ready for a new print".into() }));
            if m.state == MState::Fault { ui.label(RichText::new("Fault latched - clear to continue").color(ERR).size(12.0)); }
        };
        let temps: Vec<f32> = m.heaters.iter().map(|h| h.cur).collect();
        if ui.available_width() > 560.0 {
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(190.0, 0.0), Layout::top_down(Align::Min), |ui| state_block(ui, m));
                let w = (ui.available_width() - 16.0) / 3.0;
                stat_tile(ui, w, "thermo", &format!("{:.1} C", temps[0]), "Nozzle");
                stat_tile(ui, w, "bed", &format!("{:.1} C", temps[1]), "Bed");
                stat_tile(ui, w, "cube", &format!("{:.1} C", temps[2]), "Chamber");
            });
        } else {
            state_block(ui, m);
            ui.add_space(6.0);
            let w = (ui.available_width() - 16.0) / 3.0;
            ui.horizontal(|ui| {
                stat_tile(ui, w, "thermo", &format!("{:.0} C", temps[0]), "Nozzle");
                stat_tile(ui, w, "bed", &format!("{:.0} C", temps[1]), "Bed");
                stat_tile(ui, w, "cube", &format!("{:.0} C", temps[2]), "Chamber");
            });
        }
        ui.add_space(10.0);
        if busy {
            accent_bar(ui, m.job.progress, ui.available_width());
            ui.add_space(8.0);
        }
        tiles_min(ui, &[
            ("Progress", format!("{:.1} %", m.job.progress * 100.0)),
            ("Layer", format!("{} / {}", m.job.layer, m.job.layers)),
            ("Elapsed", fmt_dur(m.job.elapsed)),
            ("Remaining", fmt_dur(remaining)),
            ("Finishes", clock(now + remaining)),
            ("Filament", format!("{:.0} g / {:.1} m", grams, grams / 3.13)),
            ("Velocity", format!("{:.0} mm/s", vel)),
            ("Acceleration", format!("{:.0} mm/s2", accel)),
            ("Flow rate", format!("{:.1} mm3/s", flow_rate)),
        ], 120.0);
        ui.add_space(6.0);
        state_actions(ui, m);
    });
}

fn p_camera(ui: &mut Ui, m: &mut Model, h: f32) {
    let t = m.t as f32;
    let cur = m.fleet[m.active].clone();
    panel_min(ui, "Camera", h, |ui| {
        let w = ui.available_width();
        cam_tile(ui, &cur, t, w, 0.0, false);
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            status(ui, Status::Ok, "No failure detected");
            ui.label(dim("score 0.04"));
        });
        ui.horizontal(|ui| {
            if button(ui, "Snapshot") { m.log(Sev::Info, "camera", "Snapshot saved"); }
            if button(ui, "Open camera") { m.goto = Some(Page::Camera); }
        });
    });
}

fn p_filament(ui: &mut Ui, m: &mut Model, h: f32) {
    panel_min(ui, "Filament", h, |ui| {
        let act = m.active;
        let sp = m.loaded(act).first().map(|&i| m.fspools[i].clone());
        match sp {
            None => {
                status(ui, Status::Warn, "No spool loaded");
                ui.label(dim("Assign a spool from Fleet or Spool Inventory."));
                ui.add_space(6.0);
                if button(ui, "Open Spool Inventory") { m.goto = Some(Page::Spools); }
            }
            Some(sp) => {
                ui.horizontal(|ui| { status(ui, Status::Ok, "Filament present"); ui.label(dim("T0 selected")); });
                ui.add_space(4.0);
                tiles_min(ui, &[
                    ("Spool", crate::spools::code(sp.id)),
                    ("Material", format!("{} {}", sp.mat, sp.color)),
                    ("Remaining", format!("{:.0} g (~{:.0} m)", sp.left, crate::spools::length_m(&sp.mat, sp.dia, sp.left))),
                    ("Nozzle", format!("{:.1} mm", m.fleet[act].nozzle)),
                ], 104.0);
                accent_bar(ui, sp.left / sp.net.max(1.0), ui.available_width());
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let ok = m.heaters[0].cur > 170.0;
                    if button_en(ui, ok, "Extrude") { m.pos[3] += m.st.extr_len; }
                    if button_en(ui, ok, "Retract") { m.pos[3] -= m.st.extr_len; }
                    if button(ui, "Load") { run_cmd(m, "LOAD_FILAMENT"); }
                    if button(ui, "Unload") { run_cmd(m, "UNLOAD_FILAMENT"); }
                });
            }
        }
    });
}

fn p_power(ui: &mut Ui, m: &mut Model, h: f32) {
    let t = m.t as f32;
    let amps = m.drives.iter().map(|d| d.cur).sum::<f32>() * 0.9 + m.heaters.iter().map(|h| h.power / 100.0 * 4.0).sum::<f32>();
    panel_min(ui, "Power and system", h, |ui| {
        tiles_min(ui, &[
            ("48 V bus", format!("{:.1} V", 48.0 + (t * 1.7).sin() * 0.12)),
            ("Current", format!("{:.1} A", amps)),
            ("Power", format!("{:.0} W", amps * 48.0)),
            ("5 V rail", format!("{:.2} V", 5.0 + (t * 0.8).sin() * 0.01)),
            ("MCU temps", format!("{:.0} / {:.0} / {:.0} C", 51.0 + (t * 0.1).sin() * 1.5, 47.0 + (t * 0.13).sin(), 44.0 + (t * 0.09).sin())),
            ("CPU / memory", format!("{:.0} % / {:.0} %", 38.0 + (t * 0.7).sin() * 6.0, 54.0)),
            ("API latency", format!("{:.0} ms", 11.0 + (t * 2.1).sin().abs() * 4.0)),
            ("Link", "WiFi -52 dBm".into()),
            ("Uptime", fmt_dur(m.t as f32 + 4360.0)),
        ], 120.0);
    });
}

fn p_fans(ui: &mut Ui, m: &mut Model, h: f32) {
    let t = m.t as f32;
    panel_min(ui, "Fans and outputs", h, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 8.0);
            for f in m.fans.iter_mut() {
                let rot = t * f.pct / 100.0 * 12.0;
                let short = f.name.replace(" fan", "").replace("Heatsink", "Hotend");
                dial(ui, &mut f.pct, (0.0, 100.0), &short, "fan", rot, "%");
            }
            dial(ui, &mut m.st.led, (0.0, 100.0), "LED", "bulb", 0.0, "%");
        });
        ui.add_space(6.0);
        let door_ok = !m.nodes.iter().any(|n| n.class == "CHAMBER_IO" && n.status != Status::Ok);
        ui.horizontal_wrapped(|ui| {
            status(ui, if door_ok { Status::Ok } else { Status::Warn }, "Door");
            ui.add_space(12.0);
            status(ui, Status::Ok, "Filter");
            ui.add_space(12.0);
            status(ui, Status::Ok, "Enclosure");
        });
        ui.add_space(4.0);
        ui.label(dim("Drag a dial up or down. Shift for fine control."));
    });
}

fn p_motion(ui: &mut Ui, m: &mut Model, h: f32) {
    panel_min(ui, "Motion system", h, |ui| {
        for d in m.drives.iter() {
            row_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized(vec2(32.0, 20.0), egui::Label::new(RichText::new(d.axis).size(15.0).strong().color(WHITE)));
                    ui.add_sized(vec2(58.0, 20.0), egui::Label::new(mono(format!("{:.2} A", d.cur)).size(12.5)));
                    ui.add_sized(vec2(66.0, 20.0), egui::Label::new(mono(format!("{:.1} um", d.follow)).size(12.5).color(DIM)));
                    let w = (ui.available_width() - 50.0).max(24.0);
                    bar(ui, d.follow / 200.0, w, if d.follow > 150.0 { Some(Status::Warn) } else { None });
                    ui.label(mono(format!("{:.0} C", d.temp)).size(12.5).color(DIM));
                });
            });
        }
    });
}

fn p_mesh(ui: &mut Ui, m: &mut Model, h: f32) {
    panel_min(ui, "Bed mesh", h, |ui| {
        let (lo, hi) = m.mesh.iter().flatten().fold((9.0f32, -9.0f32), |a, v| (a.0.min(*v), a.1.max(*v)));
        let w = ui.available_width();
        heatmap(ui, &m.mesh, vec2(w, (w * 0.62).min(360.0)), (lo, hi), None, (350.0, 350.0));
        ui.add_space(4.0);
        ramp_legend(ui, w, (lo, hi));
        ui.horizontal(|ui| {
            if button(ui, "Open calibration") { m.st.tab = 1; m.goto = Some(Page::Calibration); }
            ui.label(dim("Hover for coordinates"));
        });
    });
}

fn p_nodes(ui: &mut Ui, m: &mut Model, h: f32) {
    let t = m.t as f32;
    panel_min(ui, "Node health", h, |ui| {
        let gap = 6.0;
        let avail = ui.available_width();
        let n = (((avail + gap) / (190.0 + gap)).floor() as usize).clamp(1, 3);
        let w = (avail - gap * (n as f32 - 1.0)) / n as f32;
        let items: Vec<(usize, Status, &str, f32)> = m.nodes.iter().enumerate().map(|(k, nd)| (k, nd.status, nd.name, nd.temp + (t * 0.2 + k as f32).sin())).collect();
        for row in items.chunks(n) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (_, st, name, temp) in row {
                    let (st, name, temp) = (*st, *name, *temp);
                    mini(ui, w, |ui| { dot(ui, st); ui.add(egui::Label::new(RichText::new(name).size(12.5)).truncate()); }, &format!("{:.0} C", temp));
                }
            });
            ui.add_space(4.0);
        }
        if button(ui, "Open nodes") { m.goto = Some(Page::Nodes); }
    });
}

fn p_console(ui: &mut Ui, m: &mut Model, h: f32) {
    panel_min(ui, "Console", h, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, n) in ["All", "Commands", "Warnings and errors"].iter().enumerate() {
                if ui.selectable_label(m.st.con_filter == i, *n).clicked() { m.st.con_filter = i; }
            }
            if sbtn(ui, "Clear").clicked() { m.st.console.clear(); }
            if sbtn(ui, "Copy").clicked() { ui.ctx().copy_text(m.st.console.join("\n")); }
        });
        ui.add_space(4.0);
        Frame::new().fill(INSET).stroke(Stroke::new(1.0_f32, CARD_LINE)).corner_radius(8).inner_margin(Margin::same(10)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height((h - 190.0).max(260.0));
            egui::ScrollArea::vertical().id_salt("dash_console").max_height((h - 190.0).max(260.0)).stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for l in m.st.console.iter() {
                    let low = l.to_lowercase();
                    let show = match m.st.con_filter { 1 => l.starts_with("> "), 2 => low.starts_with("warning") || low.starts_with("error") || l.starts_with("!!"), _ => true };
                    if show { ui.add(egui::Label::new(console_job(l)).wrap_mode(egui::TextWrapMode::Wrap)); }
                }
            });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(">").monospace().color(ACCENT).size(15.0));
            let r = ui.add(egui::TextEdit::singleline(&mut m.st.cmd).desired_width((ui.available_width() - 96.0).max(60.0)).font(FontId::monospace(14.0)).hint_text("G28, M104 S250, G1 X100 Y100, LOAD_FILAMENT, HELP"));
            if r.has_focus() {
                let n = m.st.hist.len();
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) && n > 0 { m.st.hist_idx = (m.st.hist_idx + 1).min(n); m.st.cmd = m.st.hist[n - m.st.hist_idx].clone(); }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) && n > 0 {
                    m.st.hist_idx = m.st.hist_idx.saturating_sub(1);
                    m.st.cmd = if m.st.hist_idx == 0 { String::new() } else { m.st.hist[n - m.st.hist_idx].clone() };
                }
            }
            let go = (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || primary(ui, "Send");
            if go && !m.st.cmd.trim().is_empty() {
                let cmd = std::mem::take(&mut m.st.cmd);
                run_cmd(m, cmd.trim());
                r.request_focus();
            }
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(dim("Macros"));
            for (lbl, c) in [("Home all", "G28"), ("Mesh", "G29"), ("Level", "G32"), ("Pause", "PAUSE"), ("Resume", "RESUME"), ("Shaper test", "SHAPER_CALIBRATE"), ("Help", "HELP")] {
                if sbtn(ui, lbl).clicked() { run_cmd(m, c); }
            }
            if sbtn(ui, "Cool down").clicked() { for h in m.heaters.iter_mut() { h.target = 0.0; } m.st.console.push("> cool down (all heaters off)".into()); m.st.console.push("ok".into()); }
        });
    });
}

fn p_history(ui: &mut Ui, m: &mut Model, h: f32) {
    let (now, cur) = (14.0 * 3600.0 + 32.0 * 60.0 + m.t as f32, m.fleet[m.active].clone());
    panel_min(ui, "Print history", h, |ui| {
        let w = ui.available_width();
        let mut rows: Vec<(String, Status, String, String)> = Vec::new();
        if matches!(m.state, MState::Printing | MState::Paused) {
            rows.push((m.job.file.trim_end_matches(".gcode").to_string(), Status::Ok, fmt_dur(m.job.elapsed), format!("ETA {}", clock(now + eta(&cur)))));
        }
        for (f, st, tm, d) in [
            ("pa_calib_tower", Status::Ok, "00:18:04", "today 11:02"),
            ("benchy_calib", Status::Ok, "01:02:40", "today 09:41"),
            ("enclosure_panel", Status::Ok, "05:17:12", "yesterday"),
            ("gear_train_x6", Status::Warn, "01:40:55", "yesterday"),
            ("mmu_test_4color", Status::Err, "00:52:09", "2 days ago"),
            ("housing_set_v6", Status::Ok, "07:31:20", "3 days ago"),
        ] {
            rows.push((f.to_string(), st, tm.to_string(), d.to_string()));
        }
        for (f, st, tm, d) in rows {
            Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32, CARD_LINE)).corner_radius(7).inner_margin(Margin::symmetric(10, 7)).show(ui, |ui| {
                ui.set_width(w - 22.0);
                ui.horizontal(|ui| {
                    dot(ui, st);
                    ui.label(RichText::new(f).size(12.5));
                });
                ui.horizontal(|ui| {
                    ui.label(mono(tm).size(11.5).color(DIM));
                    ui.label(dim(d));
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn p_events(ui: &mut Ui, m: &mut Model, h: f32) {
    panel_min(ui, "Recent events", h, |ui| {
        let w = ui.available_width();
        for l in m.logs.iter().rev().take(7) {
            Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32, CARD_LINE)).corner_radius(7).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.set_width(w - 22.0);
                ui.horizontal(|ui| {
                    let st = match l.sev { Sev::Info => Status::Idle, Sev::Warn => Status::Warn, Sev::Err => Status::Err };
                    dot(ui, st);
                    ui.label(mono(l.ts.clone()).size(11.0).color(FAINT));
                    ui.label(dim(l.src.clone()));
                });
                ui.add(egui::Label::new(RichText::new(l.msg.clone()).size(12.0)).wrap());
            });
            ui.add_space(4.0);
        }
        if button(ui, "Open event log") { m.goto = Some(Page::Logs); }
    });
}

fn heater_row(ui: &mut Ui, m: &mut Model, k: usize, glyph: &str, col: Color32) {
    row_card(ui, |ui| {
        ui.horizontal(|ui| {
            icon_box(ui, glyph);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(m.heaters[k].name).size(13.5).strong().color(col));
                ui.horizontal(|ui| {
                    ui.label(mono(format!("{:.1} C", m.heaters[k].cur)).size(16.0).color(WHITE));
                    bar(ui, m.heaters[k].power / 100.0, 46.0, None);
                    ui.label(mono(format!("{:.0}%", m.heaters[k].power)).size(11.0).color(DIM));
                });
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, "OFF") { m.heaters[k].target = 0.0; }
                let max = m.heaters[k].max;
                stepper(ui, &mut m.heaters[k].target, (0.0, max), 5.0, " C");
            });
        });
    });
}

fn p_temps(ui: &mut Ui, m: &mut Model) {
    panel(ui, "Temperatures", |ui| {
        let hi = m.h_hot.iter().chain(m.h_t_hot.iter()).cloned().fold(100.0_f32, f32::max);
        let top = ((hi * 1.1) / 50.0).ceil() * 50.0;
        plot(ui, 190.0, &[
            Series::from_ring("hotend", SENS_HOT, &m.h_hot).filled(),
            Series::from_ring("target", SENS_HOT, &m.h_t_hot).dashed(),
            Series::from_ring("bed", SENS_BED, &m.h_bed).filled(),
            Series::from_ring("chamber", SENS_CHAM, &m.h_cham).filled(),
        ], (0.0, HIST as f32), (0.0, top), "deg C  -  last 60 s");
        ui.add_space(8.0);
        heater_row(ui, m, 0, "thermo", SENS_HOT);
        heater_row(ui, m, 1, "bed", SENS_BED);
        heater_row(ui, m, 2, "cube", SENS_CHAM);
        ui.horizontal_wrapped(|ui| {
            ui.label(dim("Preset"));
            for p in m.profiles.iter() {
                if sbtn(ui, p.name).clicked() { m.heaters[0].target = p.nozzle; m.heaters[1].target = p.bed; m.heaters[2].target = p.chamber; }
            }
            if sbtn(ui, "All off").clicked() { for h in m.heaters.iter_mut() { h.target = 0.0; } }
        });
    });
}

fn p_move(ui: &mut Ui, m: &mut Model) {
    panel(ui, "Movement", |ui| {
        tiles_min(ui, &[
            ("X", format!("{:.2}", m.pos[0])),
            ("Y", format!("{:.2}", m.pos[1])),
            ("Z", format!("{:.2}", m.pos[2])),
            ("E", format!("{:.1}", m.pos[3])),
        ], 74.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(dim("Step (mm)"));
            let steps = [0.1_f32, 1.0, 10.0, 50.0, 100.0];
            for (k, s) in steps.iter().enumerate() {
                if choice(ui, m.st.step == k, &format!("{}", s)) { m.st.step = k; }
            }
        });
        ui.add_space(6.0);
        let st = [0.1_f32, 1.0, 10.0, 50.0, 100.0][m.st.step.min(4)];
        let bs = vec2(54.0, 40.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                ui.horizontal(|ui| { ui.add_space(bs.x + 4.0); if btn_sized(ui, "Y+", bs) { m.pos[1] += st; } });
                ui.horizontal(|ui| {
                    if btn_sized(ui, "X-", bs) { m.pos[0] -= st; }
                    if btn_sized(ui, "Home", bs) { m.pos[0] = 0.0; m.pos[1] = 0.0; m.home[0] = true; m.home[1] = true; }
                    if btn_sized(ui, "X+", bs) { m.pos[0] += st; }
                });
                ui.horizontal(|ui| { ui.add_space(bs.x + 4.0); if btn_sized(ui, "Y-", bs) { m.pos[1] -= st; } });
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                if btn_sized(ui, "Z+", bs) { m.pos[2] += st; }
                if btn_sized(ui, "Home Z", bs) { m.pos[2] = 0.0; m.home[2] = true; }
                if btn_sized(ui, "Z-", bs) { m.pos[2] -= st; }
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
                if button(ui, "Home all") { m.pos[..3].fill(0.0); m.home = [true; 3]; m.log(Sev::Info, "motion", "Homed X Y Z"); }
                if button(ui, "Park") { m.pos[0] = 175.0; m.pos[1] = 340.0; }
                if button(ui, "Motors off") { m.home = [false; 3]; m.log(Sev::Info, "motion", "Drives disabled (EN low)"); }
            });
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(dim("Extruder"));
            stepper(ui, &mut m.st.extr_len, (1.0, 200.0), 5.0, " mm");
            let ok = m.heaters[0].cur > 170.0;
            if button_en(ui, ok, "Extrude") { m.pos[3] += m.st.extr_len; }
            if button_en(ui, ok, "Retract") { m.pos[3] -= m.st.extr_len; }
            if !ok { ui.label(dim("hotend below 170 C")); }
        });
        ui.horizontal_wrapped(|ui| {
            status(ui, if m.home.iter().all(|h| *h) { Status::Ok } else { Status::Warn }, if m.home.iter().all(|h| *h) { "X Y Z homed" } else { "not homed" });
        });
    });
}

fn p_params(ui: &mut Ui, m: &mut Model) {
    panel(ui, "Print parameters", |ui| {
        let gap = 10.0;
        let avail = ui.available_width();
        let n = (((avail + gap) / (150.0 + gap)).floor() as usize).clamp(1, 3);
        let total = 9;
        let mut jerk = m.jerk / 1000.0;
        for row in 0..(total + n - 1) / n {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for col in 0..n {
                    match row * n + col {
                        0 => { param(ui, "Speed", &mut m.job.speed, (20.0, 300.0), 5.0, 0, " %"); }
                        1 => { param(ui, "Flow", &mut m.job.flow, (80.0, 120.0), 1.0, 0, " %"); }
                        2 => { param(ui, "Z offset", &mut m.z_offset, (-2.0, 2.0), 0.005, 3, " mm"); }
                        3 => { param(ui, "Pressure advance", &mut m.pa_value, (0.0, 0.5), 0.002, 3, " s"); }
                        4 => { param(ui, "PA smooth time", &mut m.pa_smooth, (0.0, 0.2), 0.005, 3, " s"); }
                        5 => { param(ui, "Max velocity", &mut m.vmax, (10.0, 1500.0), 10.0, 0, " mm/s"); }
                        6 => { if param(ui, "Max accel", &mut m.accel[0], (100.0, 40000.0), 100.0, 0, " mm/s2") { m.accel[1] = m.accel[0]; } }
                        7 => { param(ui, "Max jerk", &mut jerk, (1.0, 1000.0), 5.0, 0, " k"); }
                        8 => { param(ui, "Corner blend", &mut m.blend, (0.0, 1.0), 0.01, 2, " mm"); }
                        _ => {}
                    }
                }
            });
            ui.add_space(8.0);
        }
        m.jerk = jerk * 1000.0;
        tiles_min(ui, &[
            ("Shaper X", format!("{} {:.1} Hz", SHAPERS[m.shapers[0].kind], m.shapers[0].freq)),
            ("Shaper Y", format!("{} {:.1} Hz", SHAPERS[m.shapers[1].kind], m.shapers[1].freq)),
        ], 130.0);
        if button(ui, "Open tuning") { m.goto = Some(Page::Motion); }
    });
}
