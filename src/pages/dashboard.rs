//! Dashboard laid out as separate tiles, row cards and dials rather than key/value tables.
use super::control::{run_cmd, state_actions, SHAPERS};
use super::farm::cam_tile;
use crate::data::*;
use crate::fleetsim::eta;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, FontId, Frame, Layout, Margin, RichText, Sense, Stroke, Ui};

fn split(ui: &mut Ui, m: &mut Model, frac: f32, l: impl FnOnce(&mut Ui, &mut Model), r: impl FnOnce(&mut Ui, &mut Model)) {
    let w = ui.available_width();
    let gap = 10.0;
    let lw = (w - gap) * frac;
    let rw = w - gap - lw;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_ui_with_layout(vec2(lw, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.set_width(lw);
            l(ui, &mut *m)
        });
        ui.allocate_ui_with_layout(vec2(rw, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.set_width(rw);
            r(ui, &mut *m)
        });
    });
}

fn clock(secs: f32) -> String {
    let s = secs as u32 % 86400;
    format!("{:02}:{:02}", s / 3600, (s / 60) % 60)
}

/// Small bordered row used for lists (nodes, events, history).
fn mini(ui: &mut Ui, w: f32, f: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, CARD_LINE))
        .corner_radius(7)
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(w - 22.0);
                ui.horizontal(f);
            });
        });
}

fn limit_row(ui: &mut Ui, glyph: &str, label: &str, value: String, frac: f32) {
    row_card(ui, |ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
            crate::icons::glyph(&ui.painter().clone(), glyph, r.center(), Color32::from_gray(190), 0.0);
            ui.label(RichText::new(label).size(13.5).color(TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                Frame::new().fill(INSET).stroke(Stroke::new(1.0_f32, CARD_LINE)).corner_radius(6).inner_margin(Margin::symmetric(9, 3)).show(ui, |ui| {
                    ui.label(mono(value).size(13.0).color(WHITE));
                });
            });
        });
        ui.add_space(3.0);
        accent_bar(ui, frac, ui.available_width());
    });
}

pub fn dashboard(ui: &mut Ui, m: &mut Model) {
    title(ui, "Dashboard", &format!("{} - live telemetry, mock data", m.name));
    split(ui, m, 0.60, left, right);
}

fn left(ui: &mut Ui, m: &mut Model) {
    let t = m.t as f32;
    let printing = m.state == MState::Printing;
    let vel = if printing { 180.0 * m.job.speed / 100.0 * (0.55 + 0.45 * (t * 0.9).sin().abs()) } else { 0.0 };
    let accel = if printing { 3000.0 + 2500.0 * (t * 1.3).sin().abs() } else { 0.0 };
    let flow_rate = vel * 0.4 * 0.2 * m.job.flow / 100.0;
    let grams = m.job.progress * 312.0;
    let remaining = (m.job.total - m.job.elapsed).max(0.0);
    let now = 14.0 * 3600.0 + 32.0 * 60.0 + t;
    let cur = m.fleet[m.active].clone();
    let busy = matches!(m.state, MState::Printing | MState::Paused);

    // ---- printer status ----------------------------------------------------------------
    panel(ui, "Printer status", |ui| {
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2(200.0, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 8.0, m.state.status().color());
                    ui.label(RichText::new(m.state.label().split(' ').next().unwrap_or("")).size(30.0).strong().color(WHITE));
                });
                ui.label(dim(if busy { m.job.file.clone() } else { "Ready for a new print".into() }));
                if m.state == MState::Fault { ui.label(RichText::new("Fault latched - clear to continue").color(ERR).size(12.0)); }
            });
            let w = (ui.available_width() - 16.0) / 3.0;
            let h: Vec<(f32, f32)> = m.heaters.iter().map(|h| (h.cur, h.target)).collect();
            stat_tile(ui, w, "thermo", &format!("{:.1} C", h[0].0), "Nozzle");
            stat_tile(ui, w, "bed", &format!("{:.1} C", h[1].0), "Bed");
            stat_tile(ui, w, "cube", &format!("{:.1} C", h[2].0), "Chamber");
        });
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
        ], 128.0);
        ui.add_space(10.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                state_actions(ui, m);
                ui.add_space(8.0);
                ui.label(dim("Live Z offset"));
                ui.horizontal(|ui| {
                    let mut z = m.z_offset * 1000.0;
                    if stepper(ui, &mut z, (-2000.0, 2000.0), 10.0, " um") { m.z_offset = z / 1000.0; }
                });
            });
            ui.add_space(12.0);
            let rot = t * m.job.speed / 100.0 * 4.0;
            dial(ui, &mut m.job.speed, (20.0, 300.0), "Speed", "gauge", rot, "%");
            dial(ui, &mut m.job.flow, (80.0, 120.0), "Flow", "drop", 0.0, "%");
        });
    });

    // ---- temperatures ---------------------------------------------------------------------
    panel(ui, "Temperatures", |ui| {
        plot(ui, 190.0, &[
            Series::from_ring("nozzle", ACCENT, &m.h_hot),
            Series::from_ring("target", G3, &m.h_t_hot).dashed(),
            Series::from_ring("bed", G1, &m.h_bed),
            Series::from_ring("chamber", G2, &m.h_cham),
        ], (0.0, HIST as f32), (0.0, 300.0), "deg C  -  last 60 s");
    });

    // ---- filament | power ------------------------------------------------------------------
    split(ui, m, 0.5, |ui, m| {
        panel(ui, "Filament", |ui| {
            let act = m.active;
            let sp = m.loaded(act).first().map(|&i| m.fspools[i].clone());
            match sp {
                None => {
                    ui.horizontal(|ui| { status(ui, Status::Warn, "No spool loaded"); });
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
                    ], 120.0);
                    ui.add_space(4.0);
                    accent_bar(ui, sp.left / sp.net.max(1.0), ui.available_width());
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        let ok = m.heaters[0].cur > 170.0;
                        ui.add_enabled_ui(ok, |ui| {
                            if button(ui, "Extrude") { m.pos[3] += m.st.extr_len; }
                            if button(ui, "Retract") { m.pos[3] -= m.st.extr_len; }
                        });
                        if button(ui, "Load") { m.st.console.push("> LOAD_FILAMENT".into()); m.st.console.push("MMU: loading slot 1 ... ok".into()); }
                        if button(ui, "Unload") { m.st.console.push("> UNLOAD_FILAMENT".into()); m.st.console.push("MMU: unloading ... ok".into()); }
                    });
                }
            }
        });
    }, |ui, m| {
        panel(ui, "Power and system", |ui| {
            let t = m.t as f32;
            let amps = m.drives.iter().map(|d| d.cur).sum::<f32>() * 0.9 + m.heaters.iter().map(|h| h.power / 100.0 * 4.0).sum::<f32>();
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
            ], 112.0);
        });
    });

    // ---- limits & tuning --------------------------------------------------------------------
    panel(ui, "Limits and tuning", |ui| {
        ui.columns(2, |c| {
            limit_row(&mut c[0], "gauge", "Velocity", format!("{:.0} mm/s", m.vmax), m.vmax / 1500.0);
            limit_row(&mut c[0], "motion", "Accel X / Y / Z", format!("{:.0} / {:.0} / {:.0}", m.accel[0], m.accel[1], m.accel[2]), m.accel[0] / 20000.0);
            limit_row(&mut c[0], "list", "Shapers X / Y", format!("{} {:.0} / {} {:.0} Hz", SHAPERS[m.shapers[0].kind], m.shapers[0].freq, SHAPERS[m.shapers[1].kind], m.shapers[1].freq), 0.5);
            limit_row(&mut c[1], "hash", "Max jerk", format!("{:.0} mm/s3", m.jerk), m.jerk / 200000.0);
            limit_row(&mut c[1], "drop", "Pressure advance", format!("{:.3} s", m.pa_value), m.pa_value / 0.2);
            limit_row(&mut c[1], "grid", "Mesh", if m.mesh_adaptive { "adaptive".into() } else { "full".into() }, if m.mesh_adaptive { 0.45 } else { 1.0 });
        });
        if button(ui, "Open motion") { m.goto = Some(Page::Motion); }
    });

    // ---- console | history ----------------------------------------------------------------------
    split(ui, m, 0.5, |ui, m| {
        panel(ui, "Console", |ui| {
            Frame::new().fill(INSET).corner_radius(7).inner_margin(Margin::same(8)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical().id_salt("dash_console").max_height(130.0).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
                    for l in m.st.console.iter().rev().take(14).rev() {
                        ui.label(mono(l.clone()).size(12.0).color(if l.starts_with('>') { WHITE } else { DIM }));
                    }
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut m.st.cmd).desired_width(ui.available_width() - 90.0).font(FontId::monospace(12.5)).hint_text("G28, M104 S250, STATUS"));
                let go = (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || button(ui, "Send");
                if go && !m.st.cmd.trim().is_empty() {
                    let cmd = std::mem::take(&mut m.st.cmd);
                    run_cmd(m, cmd.trim());
                    r.request_focus();
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if button(ui, "Home all") { run_cmd(m, "G28"); }
                if button(ui, "Cool down") { for h in m.heaters.iter_mut() { h.target = 0.0; } }
                if button(ui, "Auto-calibrate") { m.goto = Some(Page::Calibration); }
            });
        });
    }, |ui, m| {
        panel(ui, "Print history", |ui| {
            let (now, cur) = (14.0 * 3600.0 + 32.0 * 60.0 + m.t as f32, m.fleet[m.active].clone());
            let w = ui.available_width();
            let mut rows: Vec<(String, Status, String, String)> = Vec::new();
            if matches!(m.state, MState::Printing | MState::Paused) {
                rows.push((m.job.file.trim_end_matches(".gcode").to_string(), Status::Ok, fmt_dur(m.job.elapsed), format!("ETA {}", clock(now + eta(&cur)))));
            }
            for (f, st, r, tm, d) in [
                ("pa_calib_tower", Status::Ok, "complete", "00:18:04", "today 11:02"),
                ("benchy_calib", Status::Ok, "complete", "01:02:40", "today 09:41"),
                ("enclosure_panel", Status::Ok, "complete", "05:17:12", "yesterday"),
                ("gear_train_x6", Status::Warn, "cancelled", "01:40:55", "yesterday"),
                ("mmu_test_4color", Status::Err, "failed", "00:52:09", "2 days ago"),
            ] {
                rows.push((f.to_string(), st, format!("{}  {}", r, tm), d.to_string()));
            }
            for (f, st, mid, d) in rows {
                mini(ui, w, |ui| {
                    dot(ui, st);
                    let short: String = f.chars().take(15).collect();
                    ui.label(RichText::new(short).size(12.5));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| { ui.label(dim(d)); ui.label(mono(mid.split("  ").last().unwrap_or("").to_string()).size(11.5).color(DIM)); });
                });
                ui.add_space(4.0);
            }
        });
    });

    // ---- nodes | events -----------------------------------------------------------------------------
    split(ui, m, 0.5, |ui, m| {
        panel(ui, "Node health", |ui| {
            let t = m.t as f32;
            let gap = 6.0;
            let w = (ui.available_width() - gap) / 2.0;
            let names: Vec<(usize, &Node)> = m.nodes.iter().enumerate().collect();
            for pair in names.chunks(2) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (k, n) in pair {
                        mini(ui, w, |ui| {
                            status(ui, n.status, n.name);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| { ui.label(mono(format!("{:.0} C", n.temp + (t * 0.2 + *k as f32).sin())).size(11.5).color(DIM)); });
                        });
                    }
                });
                ui.add_space(4.0);
            }
            ui.add_space(6.0);
            if button(ui, "Open nodes") { m.goto = Some(Page::Nodes); }
        });
    }, |ui, m| {
        panel(ui, "Recent events", |ui| {
            let w = ui.available_width();
            for l in m.logs.iter().rev().take(7) {
                mini(ui, w, |ui| {
                    let st = match l.sev { Sev::Info => Status::Idle, Sev::Warn => Status::Warn, Sev::Err => Status::Err };
                    dot(ui, st);
                    ui.label(mono(l.ts.clone()).size(11.0).color(FAINT));
                    ui.add(egui::Label::new(RichText::new(l.msg.clone()).size(12.0)).truncate());
                });
                ui.add_space(4.0);
            }
            if button(ui, "Open event log") { m.goto = Some(Page::Logs); }
        });
    });
}

fn right(ui: &mut Ui, m: &mut Model) {
    let t = m.t as f32;
    let cur = m.fleet[m.active].clone();

    panel(ui, "Camera", |ui| {
        let w = ui.available_width();
        cam_tile(ui, &cur, t, w, 0.0, false);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            status(ui, Status::Ok, "No failure detected");
            ui.label(dim("score 0.04"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, "Open") { m.goto = Some(Page::Camera); }
                if button(ui, "Snapshot") { m.log(Sev::Info, "camera", "Snapshot saved"); }
            });
        });
    });

    panel(ui, "Quick controls", |ui| {
        for (k, glyph) in ["thermo", "bed", "cube"].iter().enumerate() {
            row_card(ui, |ui| {
                ui.horizontal(|ui| {
                    icon_box(ui, glyph);
                    ui.label(RichText::new(m.heaters[k].name).size(14.5).strong().color(WHITE));
                    ui.add_space(10.0);
                    ui.vertical(|ui| { ui.spacing_mut().item_spacing.y = 0.0; ui.label(dim("Current")); ui.label(mono(format!("{:.1} C", m.heaters[k].cur)).size(14.0).color(WHITE)); });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, "OFF") { m.heaters[k].target = 0.0; }
                        ui.add_space(4.0);
                        let max = m.heaters[k].max;
                        stepper(ui, &mut m.heaters[k].target, (0.0, max), 5.0, " C");
                    });
                });
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(dim("Preset"));
            for p in m.profiles.iter() {
                if sbtn(ui, p.name).clicked() { m.heaters[0].target = p.nozzle; m.heaters[1].target = p.bed; m.heaters[2].target = p.chamber; }
            }
        });
    });

    panel(ui, "Fans and outputs", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
            for f in m.fans.iter_mut() {
                let rot = t * f.pct / 100.0 * 12.0;
                let short = f.name.replace(" fan", "").replace("Heatsink", "Hotend");
                dial(ui, &mut f.pct, (0.0, 100.0), &short, "fan", rot, "%");
            }
            dial(ui, &mut m.st.led, (0.0, 100.0), "LED", "bulb", 0.0, "%");
        });
        ui.add_space(4.0);
        let door_ok = !m.nodes.iter().any(|n| n.class == "CHAMBER_IO" && n.status != Status::Ok);
        ui.horizontal(|ui| {
            status(ui, if door_ok { Status::Ok } else { Status::Warn }, "Door");
            ui.add_space(14.0);
            status(ui, Status::Ok, "Filter");
            ui.add_space(14.0);
            status(ui, Status::Ok, "Enclosure");
        });
    });

    panel(ui, "Motion system", |ui| {
        for d in m.drives.iter() {
            row_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized(vec2(34.0, 20.0), egui::Label::new(RichText::new(d.axis).size(15.0).strong().color(WHITE)));
                    ui.add_sized(vec2(56.0, 20.0), egui::Label::new(mono(format!("{:.2} A", d.cur)).size(12.5)));
                    ui.add_sized(vec2(62.0, 20.0), egui::Label::new(mono(format!("{:.1} um", d.follow)).size(12.5).color(DIM)));
                    let w = (ui.available_width() - 64.0).max(30.0);
                    bar(ui, d.follow / 200.0, w, if d.follow > 150.0 { Some(Status::Warn) } else { None });
                    ui.label(mono(format!("{:.0} C", d.temp)).size(12.5).color(DIM));
                });
            });
        }
    });

    panel(ui, "Toolhead", |ui| {
        let tf = m.t as f32;
        let vel = |s: f32| if m.state == MState::Printing { 180.0 * (0.55 + 0.45 * (s * 0.9).sin().abs()) } else { 0.0 };
        tiles_min(ui, &[
            ("X", format!("{:.2}", m.pos[0])),
            ("Y", format!("{:.2}", m.pos[1])),
            ("Z", format!("{:.2}", m.pos[2])),
            ("E", format!("{:.1}", m.pos[3])),
        ], 70.0);
        ui.add_space(6.0);
        let pts: Vec<[f32; 2]> = (0..120).map(|k| [k as f32, vel(tf - (120 - k) as f32 * 0.25)]).collect();
        plot(ui, 100.0, &[Series::new("velocity", ACCENT, pts)], (0.0, 120.0), (0.0, 260.0), "mm/s");
        ui.add_space(6.0);
        tiles_min(ui, &[
            ("Homed", if m.home.iter().all(|h| *h) { "X Y Z".into() } else { "not all".into() }),
            ("Probe", format!("{:.1} C", 52.0 + (tf * 0.05).sin())),
            ("Z offset", format!("{:+.3} mm", m.z_offset)),
        ], 90.0);
    });

    panel(ui, "Bed mesh", |ui| {
        let (lo, hi) = m.mesh.iter().flatten().fold((9.0f32, -9.0f32), |a, v| (a.0.min(*v), a.1.max(*v)));
        let w = ui.available_width();
        heatmap(ui, &m.mesh, vec2(w, w * 0.58), (lo, hi), None, (350.0, 350.0));
        ui.add_space(4.0);
        ramp_legend(ui, w, (lo, hi));
        ui.horizontal(|ui| {
            ui.label(dim("Hover for coordinates"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| { if button(ui, "Open calibration") { m.st.tab = 1; m.goto = Some(Page::Calibration); } });
        });
    });
}
