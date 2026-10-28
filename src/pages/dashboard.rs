//! Dashboard: a full machine overview in the spirit of Mainsail - status, camera, toolhead,
//! temperatures, filament, fans, power, motion system, limits, bed mesh, console and history.
use super::control::{run_cmd, state_actions, SHAPERS};
use super::farm::cam_tile;
use crate::data::*;
use crate::fleetsim::eta;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align, DragValue, FontId, Layout, RichText, Ui};

fn split(ui: &mut Ui, m: &mut Model, frac: f32, l: impl FnOnce(&mut Ui, &mut Model), r: impl FnOnce(&mut Ui, &mut Model)) {
    let w = ui.available_width();
    let gap = 8.0;
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

/// Run `f` in a child of fixed width, so an overflowing panel cannot widen the rows below it.
fn fixed(ui: &mut Ui, w: f32, f: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_width(w);
        f(ui)
    });
}

fn big(ui: &mut Ui, cap: &str, val: String) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.label(dim(cap));
        ui.label(mono(val).size(16.0).color(WHITE));
    });
}

fn clock(secs: f32) -> String {
    let s = secs as u32 % 86400;
    format!("{:02}:{:02}", s / 3600, (s / 60) % 60)
}

pub fn dashboard(ui: &mut Ui, m: &mut Model) {
    title(ui, "Dashboard", &format!("{} - live telemetry, mock data", m.name));
    let t = m.t as f32;
    let printing = m.state == MState::Printing;
    let vel = if printing { 180.0 * m.job.speed / 100.0 * (0.55 + 0.45 * (t * 0.9).sin().abs()) } else { 0.0 };
    let accel = if printing { 3000.0 + 2500.0 * (t * 1.3).sin().abs() } else { 0.0 };
    let flow_rate = vel * 0.4 * 0.2 * m.job.flow / 100.0;
    let grams = m.job.progress * 312.0;
    let remaining = (m.job.total - m.job.elapsed).max(0.0);
    let now = 14.0 * 3600.0 + 32.0 * 60.0 + t;
    let cur = m.fleet[m.active].clone();
    let w0 = ui.available_width();

    // ---- row 1: status + camera -------------------------------------------------------------
    split(ui, m, 0.64, |ui, m| {
        panel(ui, "Print status", |ui| {
            ui.horizontal(|ui| {
                status(ui, m.state.status(), m.state.label());
                ui.add_space(8.0);
                ui.label(mono(m.job.file.clone()).size(16.0).color(WHITE));
            });
            ui.add_space(2.0);
            accent_bar(ui, m.job.progress, ui.available_width());
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 30.0;
                big(ui, "Progress", format!("{:.1} %", m.job.progress * 100.0));
                big(ui, "Layer", format!("{} / {}", m.job.layer, m.job.layers));
                big(ui, "Elapsed", fmt_dur(m.job.elapsed));
                big(ui, "Remaining", fmt_dur(remaining));
                big(ui, "Finishes", clock(now + remaining));
                big(ui, "Filament", format!("{:.0} g / {:.1} m", grams, grams / 3.13));
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 30.0;
                big(ui, "Velocity", format!("{:.0} mm/s", vel));
                big(ui, "Acceleration", format!("{:.0} mm/s2", accel));
                big(ui, "Flow rate", format!("{:.1} mm3/s", flow_rate));
                big(ui, "Layer time", "58 s".into());
                big(ui, "Objects", format!("{} / {}", m.job.objects.iter().filter(|o| !o.excluded).count(), m.job.objects.len()));
                big(ui, "Est. cost", format!("${:.2}", grams * 0.026));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    state_actions(ui, m);
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(dim("Z offset (live)"));
                        for (lbl, d) in [("-0.05", -0.05), ("-0.01", -0.01), ("+0.01", 0.01), ("+0.05", 0.05)] {
                            if button(ui, lbl) { m.z_offset += d; }
                        }
                        ui.label(mono(format!("{:+.3} mm", m.z_offset)).color(WHITE));
                    });
                });
                ui.add_space(14.0);
                knob(ui, &mut m.job.speed, (20.0, 300.0), "Speed", "%", 0);
                knob(ui, &mut m.job.flow, (80.0, 120.0), "Flow", "%", 0);
            });
        });
    }, |ui, m| {
        panel(ui, "Camera", |ui| {
            let w = ui.available_width();
            cam_tile(ui, &cur, t, w, 0.0, false);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                status(ui, Status::Ok, "No failure detected");
                ui.label(dim("score 0.04"));
            });
            ui.horizontal(|ui| {
                if button(ui, "Snapshot") { m.log(Sev::Info, "camera", "Snapshot saved"); }
                if button(ui, "Open camera") { m.goto = Some(Page::Camera); }
            });
        });
    });

    // ---- row 2: toolhead + temperatures --------------------------------------------------------
    split(ui, m, 0.5, |ui, m| {
        panel(ui, "Toolhead", |ui| {
            egui::Grid::new("dash_pos").num_columns(4).spacing([22.0, 4.0]).show(ui, |ui| {
                for (k, (ax, v)) in [("X", m.pos[0]), ("Y", m.pos[1]), ("Z", m.pos[2]), ("E", m.pos[3])].iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.label(dim(*ax));
                        ui.add(egui::Label::new(mono(format!("{:.2}", v)).size(17.0).color(WHITE)).extend());
                        if k < 3 { status(ui, if m.home[k] { Status::Ok } else { Status::Warn }, if m.home[k] { "homed" } else { "not homed" }); }
                        else { ui.label(dim("extruder")); }
                    });
                }
            });
            ui.add_space(6.0);
            let pts: Vec<[f32; 2]> = (0..120).map(|k| { let s = t - (120 - k) as f32 * 0.25; [k as f32, if printing { 180.0 * (0.55 + 0.45 * (s * 0.9).sin().abs()) } else { 0.0 }] }).collect();
            plot(ui, 110.0, &[Series::new("velocity", ACCENT, pts)], (0.0, 120.0), (0.0, 260.0), "mm/s");
            ui.add_space(4.0);
            kv(ui, "dash_th", &[
                ("Pressure advance", format!("{:.3} s  (smooth {:.3} s)", m.pa_value, m.pa_smooth)),
                ("Shapers X / Y", format!("{} {:.1} Hz / {} {:.1} Hz", SHAPERS[m.shapers[0].kind], m.shapers[0].freq, SHAPERS[m.shapers[1].kind], m.shapers[1].freq)),
                ("Probe", format!("eddy, {:.1} C, z offset {:+.3} mm", 52.0 + (t * 0.05).sin(), m.z_offset)),
            ]);
        });
    }, |ui, m| {
        panel(ui, "Temperatures", |ui| {
            egui::Grid::new("dash_temps").num_columns(5).spacing([16.0, 6.0]).show(ui, |ui| {
                for h in ["", "Current", "Target", "Output", ""] { th(ui, h); }
                ui.end_row();
                for h in m.heaters.iter_mut() {
                    ui.label(h.name);
                    ui.label(mono(format!("{:6.1}", h.cur)).size(15.0).color(WHITE));
                    ui.add(DragValue::new(&mut h.target).range(0.0..=h.max).suffix(" C"));
                    ui.horizontal(|ui| { bar(ui, h.power / 100.0, 60.0, None); ui.label(mono(format!("{:.0}%", h.power))); });
                    if sbtn(ui, "off").clicked() { h.target = 0.0; }
                    ui.end_row();
                }
            });
            ui.add_space(4.0);
            plot(ui, 120.0, &[
                Series::from_ring("hotend", ACCENT, &m.h_hot),
                Series::from_ring("target", G3, &m.h_t_hot).dashed(),
                Series::from_ring("bed", G1, &m.h_bed),
                Series::from_ring("chamber", G2, &m.h_cham),
            ], (0.0, HIST as f32), (0.0, 300.0), "deg C");
            ui.horizontal(|ui| {
                ui.label(dim("Preset"));
                for p in m.profiles.iter() {
                    if sbtn(ui, p.name).clicked() { m.heaters[0].target = p.nozzle; m.heaters[1].target = p.bed; m.heaters[2].target = p.chamber; }
                }
            });
        });
    });

    // ---- row 3: filament, fans, power --------------------------------------------------------------
    fixed(ui, w0, |ui| {
    ui.columns(3, |c| {
        panel(&mut c[0], "Filament", |ui| {
            let act = m.active;
            let sp = m.loaded(act).first().map(|&i| m.fspools[i].clone());
            ui.horizontal(|ui| { status(ui, if sp.is_some() { Status::Ok } else { Status::Warn }, if sp.is_some() { "Filament present" } else { "No spool loaded" }); ui.label(dim("T0 selected")); });
            let sp = match sp { Some(s) => s, None => { ui.label(dim("Assign a spool from Fleet or Spool Inventory.")); if button(ui, "Open Spool Inventory") { m.goto = Some(Page::Spools); } return; } };
            kv(ui, "dash_fil", &[
                ("Spool", format!("{}  {} {}", crate::spools::code(sp.id), sp.brand, sp.color)),
                ("Material", format!("{} ({})", sp.mat, m.profiles[crate::spools::profile_for(&sp.mat)].name)),
                ("Remaining", format!("{:.0} g of {:.0} g  (~{:.0} m)", sp.left, sp.net, crate::spools::length_m(&sp.mat, sp.dia, sp.left))),
                ("Nozzle", format!("{:.1} mm, hardened", m.fleet[act].nozzle)),
                ("Spool loaded", format!("{} slot(s) in use", m.loaded(act).len())),
            ]);
            bar_full(ui, sp.left / sp.net.max(1.0), None);
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.add(DragValue::new(&mut m.st.extr_len).range(1.0..=200.0).suffix(" mm"));
                let ok = m.heaters[0].cur > 170.0;
                ui.add_enabled_ui(ok, |ui| {
                    if button(ui, "Extrude") { m.pos[3] += m.st.extr_len; }
                    if button(ui, "Retract") { m.pos[3] -= m.st.extr_len; }
                });
                if button(ui, "Load") { m.st.console.push("> LOAD_FILAMENT".into()); m.st.console.push("MMU: loading slot 1 ... ok".into()); }
                if button(ui, "Unload") { m.st.console.push("> UNLOAD_FILAMENT".into()); m.st.console.push("MMU: unloading ... ok".into()); }
            });
        });
        panel(&mut c[1], "Fans and outputs", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                for f in m.fans.iter_mut() {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        knob(ui, &mut f.pct, (0.0, 100.0), f.name.split(' ').next().unwrap_or(f.name), "%", 0);
                        ui.label(mono(format!("{:.0} rpm", f.rpm_max * f.pct / 100.0)).size(10.0).color(if f.pct > 0.5 { DIM } else { FAINT }));
                        ui.label(dim(f.node).size(9.5));
                    });
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    knob(ui, &mut m.st.led, (0.0, 100.0), "LED", "%", 0);
                    ui.label(mono(if m.st.led > 0.5 { "on" } else { "off" }).size(10.0).color(DIM));
                    ui.label(dim("Chamber IO").size(9.5));
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                status(ui, if m.nodes.iter().any(|n| n.class == "CHAMBER_IO" && n.status != Status::Ok) { Status::Warn } else { Status::Ok }, "Door");
                ui.add_space(10.0);
                status(ui, Status::Ok, "Filter");
                ui.add_space(10.0);
                status(ui, Status::Ok, "Enclosure");
            });
        });
        panel(&mut c[2], "Power and system", |ui| {
            let amps = m.drives.iter().map(|d| d.cur).sum::<f32>() * 0.9 + m.heaters.iter().map(|h| h.power / 100.0 * 4.0).sum::<f32>();
            kv(ui, "dash_pow", &[
                ("48 V bus", format!("{:.1} V", 48.0 + (t * 1.7).sin() * 0.12)),
                ("Current", format!("{:.1} A", amps)),
                ("Power", format!("{:.0} W", amps * 48.0)),
                ("5 V rail", format!("{:.2} V", 5.0 + (t * 0.8).sin() * 0.01)),
                ("MCU temps", format!("{:.0} / {:.0} / {:.0} C", 51.0 + (t * 0.1).sin() * 1.5, 47.0 + (t * 0.13).sin(), 44.0 + (t * 0.09).sin())),
                ("CPU / memory", format!("{:.0} % / {:.0} %", 38.0 + (t * 0.7).sin() * 6.0, 54.0)),
                ("API latency", format!("{:.0} ms", 11.0 + (t * 2.1).sin().abs() * 4.0)),
                ("Link", "WiFi -52 dBm".into()),
                ("Uptime", fmt_dur(m.t as f32 + 4360.0)),
                ("SD / flash wear", "11.2 GB / 2 %".into()),
            ]);
        });
    });
    });

    // ---- row 4: motion, limits, mesh ------------------------------------------------------------
    fixed(ui, w0, |ui| {
    ui.columns(3, |c| {
        panel(&mut c[0], "Motion system", |ui| {
            egui::Grid::new("dash_drv").num_columns(5).spacing([12.0, 5.0]).show(ui, |ui| {
                for h in ["Axis", "Current", "Follow err", "Temp", ""] { th(ui, h); }
                ui.end_row();
                for d in m.drives.iter() {
                    ui.label(mono(d.axis));
                    ui.label(mono(format!("{:.2} A", d.cur)));
                    ui.label(mono(format!("{:.1} um", d.follow)));
                    ui.label(mono(format!("{:.0} C", d.temp)));
                    bar(ui, d.follow / 200.0, 44.0, if d.follow > 150.0 { Some(Status::Warn) } else { None });
                    ui.end_row();
                }
            });
        });
        panel(&mut c[1], "Limits and tuning", |ui| {
            kv(ui, "dash_lim", &[
                ("Velocity", format!("{:.0} mm/s", m.vmax)),
                ("Accel X/Y/Z", format!("{:.0}/{:.0}/{:.0}", m.accel[0], m.accel[1], m.accel[2])),
                ("Max jerk", format!("{:.0} mm/s3", m.jerk)),
                ("Corner blend", format!("{:.2} mm", m.blend)),
                ("Mesh", if m.mesh_adaptive { "adaptive".into() } else { "full".into() }),
                ("Gantry tilt", format!("{:+.0} / {:+.0} / {:+.0} um", m.tilt[0] * 1000.0, m.tilt[1] * 1000.0, m.tilt[2] * 1000.0)),
                ("X twist", format!("{:+.3} mm/m", m.xtwist)),
            ]);
            if button(ui, "Open motion") { m.goto = Some(Page::Motion); }
        });
        panel(&mut c[2], "Bed mesh", |ui| {
            let (lo, hi) = m.mesh.iter().flatten().fold((9.0f32, -9.0f32), |a, v| (a.0.min(*v), a.1.max(*v)));
            let w = ui.available_width();
            heatmap(ui, &m.mesh, vec2(w, w * 0.62), (lo, hi), None, (350.0, 350.0));
            ui.label(dim(format!("range {:+.0} .. {:+.0} um - hover for coordinates", lo * 1000.0, hi * 1000.0)));
            if button(ui, "Open calibration") { m.st.tab = 1; m.goto = Some(Page::Calibration); }
        });
    });
    });

    // ---- row 5: console + history -------------------------------------------------------------------
    fixed(ui, w0, |ui| {
    ui.columns(2, |c| {
        panel(&mut c[0], "Console", |ui| {
            egui::ScrollArea::vertical().id_salt("dash_console").max_height(150.0).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
                for l in m.st.console.iter().rev().take(14).rev() {
                    ui.label(mono(l.clone()).color(if l.starts_with('>') { WHITE } else { DIM }));
                }
            });
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut m.st.cmd).desired_width(ui.available_width() - 90.0).font(FontId::monospace(12.5)).hint_text("G28, M104 S250, STATUS"));
                let go = (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || button(ui, "Send");
                if go && !m.st.cmd.trim().is_empty() {
                    let cmd = std::mem::take(&mut m.st.cmd);
                    run_cmd(m, cmd.trim());
                    r.request_focus();
                }
            });
            ui.horizontal_wrapped(|ui| {
                if button(ui, "Home all") { run_cmd(m, "G28"); }
                if button(ui, "Cool down") { for h in m.heaters.iter_mut() { h.target = 0.0; } }
                if button(ui, "Purge") { m.st.console.push("> PURGE (adaptive)".into()); m.st.console.push("ok".into()); }
                if button(ui, "Auto-calibrate") { m.goto = Some(Page::Calibration); }
            });
        });
        panel(&mut c[1], "Print history", |ui| {
            egui::Grid::new("dash_hist").num_columns(4).striped(true).spacing([14.0, 5.0]).show(ui, |ui| {
                for h in ["File", "Result", "Time", "Finished"] { th(ui, h); }
                ui.end_row();
                ui.label(mono(m.job.file.trim_end_matches(".gcode").chars().take(18).collect::<String>()));
                status(ui, Status::Ok, "printing");
                ui.label(mono(fmt_dur(m.job.elapsed)));
                ui.label(dim(format!("ETA {}", clock(now + eta(&cur)))));
                ui.end_row();
                for (f, r, st, tm, g, d) in [
                    ("pa_calib_tower", "complete", Status::Ok, "00:18:04", "6 g", "today 11:02"),
                    ("benchy_calib", "complete", Status::Ok, "01:02:40", "14 g", "today 09:41"),
                    ("enclosure_panel", "complete", Status::Ok, "05:17:12", "186 g", "yesterday"),
                    ("gear_train_x6", "cancelled", Status::Warn, "01:40:55", "47 g", "yesterday"),
                    ("mmu_test_4color", "failed", Status::Err, "00:52:09", "22 g", "2 days ago"),
                    ("housing_set_v6", "complete", Status::Ok, "07:31:20", "308 g", "3 days ago"),
                ] {
                    let _ = g; ui.label(mono(f)); status(ui, st, r); ui.label(mono(tm)); ui.label(dim(d));
                    ui.end_row();
                }
            });
        });
    });
    });

    // ---- row 6: nodes + events --------------------------------------------------------------------------------
    fixed(ui, w0, |ui| {
    ui.columns(2, |c| {
        panel(&mut c[0], "Node health", |ui| {
            egui::Grid::new("dash_nodes").num_columns(6).spacing([16.0, 4.0]).show(ui, |ui| {
                for (k, n) in m.nodes.iter().enumerate() {
                    status(ui, n.status, n.name);
                    ui.label(mono(format!("{:.0} C", n.temp + (t * 0.2 + k as f32).sin())).color(DIM));
                    if k % 2 == 1 { ui.end_row(); }
                }
            });
            if button(ui, "Open nodes") { m.goto = Some(Page::Nodes); }
        });
        panel(&mut c[1], "Recent events", |ui| {
            for l in m.logs.iter().rev().take(8) {
                ui.horizontal(|ui| {
                    ui.label(mono(l.ts.clone()).color(FAINT));
                    let st = match l.sev { Sev::Info => Status::Idle, Sev::Warn => Status::Warn, Sev::Err => Status::Err };
                    dot(ui, st);
                    ui.label(dim(l.src.clone()));
                    ui.label(RichText::new(l.msg.clone()).size(12.5));
                });
            }
            if button(ui, "Open event log") { m.goto = Some(Page::Logs); }
        });
    });
    });
}
