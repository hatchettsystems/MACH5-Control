use crate::data::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align2, Color32, DragValue, FontId, RichText, Sense, Stroke, StrokeKind, Ui};

pub fn state_actions(ui: &mut Ui, m: &mut Model) {
    ui.horizontal(|ui| match m.state {
        MState::Printing => {
            if button(ui, "Pause") {
                m.state = MState::Paused;
                m.log(Sev::Info, "job", "Paused by operator");
            }
            if button(ui, "Cancel") {
                m.state = MState::Idle;
                m.log(Sev::Warn, "job", "Cancelled by operator");
            }
        }
        MState::Paused => {
            if primary(ui, "Resume") {
                m.state = MState::Printing;
                m.log(Sev::Info, "job", "Resumed");
            }
            if button(ui, "Cancel") {
                m.state = MState::Idle;
            }
        }
        MState::Fault => {
            if primary(ui, "Clear faults (host command)") {
                m.clear_faults();
            }
        }
        _ => {
            if primary(ui, "Start job") {
                m.state = MState::Printing;
                m.log(Sev::Info, "job", "Start requested");
            }
        }
    });
}

pub const SHAPERS: [&str; 5] = ["ZV", "MZV", "ZVD", "EI", "2HUMP_EI"];

pub fn control(ui: &mut Ui, m: &mut Model) {
    title(ui, "Control", "Manual motion, temperatures, extrusion and console");
    adaptive(ui, 2, |c| {
        panel(c.at(0), "Jog", |ui| {
            let steps = [0.1_f32, 1.0, 10.0, 50.0, 100.0];
            ui.horizontal(|ui| {
                ui.label(dim("Step (mm)"));
                for (k, s) in steps.iter().enumerate() {
                    if ui.selectable_label(m.st.step == k, format!("{}", s)).clicked() {
                        m.st.step = k;
                    }
                }
            });
            let st = steps[m.st.step];
            ui.add_space(6.0);
            let bs = vec2(76.0, 34.0);
            ui.horizontal(|ui| {
                egui::Grid::new("jog_xy").spacing([4.0, 4.0]).show(ui, |ui| {
                    ui.label("");
                    if ui.add_sized(bs, egui::Button::new("Y+")).clicked() { m.pos[1] += st; }
                    ui.label("");
                    ui.end_row();
                    if ui.add_sized(bs, egui::Button::new("X-")).clicked() { m.pos[0] -= st; }
                    if ui.add_sized(bs, egui::Button::new("Home")).clicked() { m.pos[0] = 0.0; m.pos[1] = 0.0; m.home[0] = true; m.home[1] = true; }
                    if ui.add_sized(bs, egui::Button::new("X+")).clicked() { m.pos[0] += st; }
                    ui.end_row();
                    ui.label("");
                    if ui.add_sized(bs, egui::Button::new("Y-")).clicked() { m.pos[1] -= st; }
                    ui.label("");
                    ui.end_row();
                });
                ui.add_space(24.0);
                egui::Grid::new("jog_z").spacing([4.0, 4.0]).show(ui, |ui| {
                    if ui.add_sized(bs, egui::Button::new("Z+")).clicked() { m.pos[2] += st; }
                    ui.end_row();
                    if ui.add_sized(bs, egui::Button::new("Home Z")).clicked() { m.pos[2] = 0.0; m.home[2] = true; }
                    ui.end_row();
                    if ui.add_sized(bs, egui::Button::new("Z-")).clicked() { m.pos[2] -= st; }
                    ui.end_row();
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if button(ui, "Home all") {
                    m.pos[0] = 0.0; m.pos[1] = 0.0; m.pos[2] = 0.0;
                    m.home = [true; 3];
                    m.log(Sev::Info, "motion", "Homed X Y Z");
                }
                if button(ui, "Disable motors") {
                    m.home = [false; 3];
                    m.log(Sev::Info, "motion", "Drives disabled (EN low)");
                }
                if button(ui, "Park") { m.pos[0] = 175.0; m.pos[1] = 340.0; }
            });
            ui.add_space(4.0);
            kv(ui, "ctl_pos", &[("X / Y / Z", format!("{:.2} / {:.2} / {:.2}", m.pos[0], m.pos[1], m.pos[2]))]);
        });
        panel(c.at(1), "Temperatures and extrusion", |ui| {
            egui::Grid::new("ctl_temps").num_columns(4).spacing([14.0, 6.0]).show(ui, |ui| {
                for h in m.heaters.iter_mut() {
                    ui.label(dim(h.name));
                    ui.label(mono(format!("{:.1}", h.cur)).color(WHITE));
                    ui.add(DragValue::new(&mut h.target).range(0.0..=h.max).suffix(" C"));
                    if crate::ui::sbtn(ui, "off").clicked() { h.target = 0.0; }
                    ui.end_row();
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(dim("Preset"));
                for p in m.profiles.iter() {
                    if crate::ui::sbtn(ui, p.name).clicked() {
                        m.heaters[0].target = p.nozzle;
                        m.heaters[1].target = p.bed;
                        m.heaters[2].target = p.chamber;
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(dim("Length"));
                ui.add(DragValue::new(&mut m.st.extr_len).range(1.0..=200.0).suffix(" mm"));
                let ok = m.heaters[0].cur > 170.0;
                ui.add_enabled_ui(ok, |ui| {
                    if button(ui, "Extrude") { m.pos[3] += m.st.extr_len; }
                    if button(ui, "Retract") { m.pos[3] -= m.st.extr_len; }
                });
                if !ok { ui.label(dim("hotend below 170 C")); }
            });
        });
    });

    panel(ui, "Macros", |ui| {
        ui.horizontal_wrapped(|ui| {
            if button(ui, "Auto-calibrate") { m.goto = Some(Page::Calibration); }
            if button(ui, "Load filament") { m.st.console.push("> LOAD_FILAMENT".into()); m.st.console.push("MMU: loading slot 1 ... ok".into()); }
            if button(ui, "Unload filament") { m.st.console.push("> UNLOAD_FILAMENT".into()); m.st.console.push("MMU: unloading ... ok".into()); }
            if button(ui, "Purge") { m.st.console.push("> PURGE (adaptive)".into()); m.st.console.push("ok".into()); }
            if button(ui, "Cool down") { for h in m.heaters.iter_mut() { h.target = 0.0; } }
        });
        ui.label(dim("Macros are rules in the sandboxed engine, not shell commands."));
    });

    panel(ui, "Console", |ui| {
        egui::ScrollArea::vertical().max_height(180.0).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
            for l in &m.st.console {
                ui.label(mono(l.clone()).color(if l.starts_with('>') { WHITE } else { DIM }));
            }
        });
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut m.st.cmd).desired_width(ui.available_width() - 70.0).font(FontId::monospace(12.5)).hint_text("G28, M104 S250, M140 S90, M112, HELP"));
            let go = (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || button(ui, "Send");
            if go && !m.st.cmd.trim().is_empty() {
                let cmd = std::mem::take(&mut m.st.cmd);
                run_cmd(m, cmd.trim());
                r.request_focus();
            }
        });
    });
}

pub fn run_cmd(m: &mut Model, cmd: &str) {
    let cmd = cmd.trim();
    m.st.console.push(format!("> {}", cmd));
    if m.st.hist.last().map(|l| l != cmd).unwrap_or(true) {
        m.st.hist.push(cmd.to_string());
    }
    if m.st.hist.len() > 60 { m.st.hist.remove(0); }
    m.st.hist_idx = 0;
    let up = cmd.to_uppercase();
    let word = up.split_whitespace().next().unwrap_or("").to_string();
    let num = |p: &str| up.split_whitespace().skip(1).find_map(|w| w.strip_prefix(p)).and_then(|v| v.parse::<f32>().ok());
    let say = |m: &mut Model, s: &str| m.st.console.push(s.to_string());
    let act = m.active;
    match word.as_str() {
        "G28" => {
            m.home = [true; 3];
            m.pos[..3].fill(0.0);
            say(m, "// homing X Y Z (eddy trigger on Z)");
            say(m, "ok");
        }
        "G0" | "G1" => {
            if !m.home.iter().all(|h| *h) {
                say(m, "error: Must home axis first (run G28)");
            } else {
                let mut clamped = false;
                for (k, ax) in ["X", "Y", "Z"].iter().enumerate() {
                    if let Some(v) = num(ax) {
                        let lim = if k == 2 { 330.0 } else { 350.0 };
                        let c = v.clamp(0.0, lim);
                        clamped |= (c - v).abs() > 0.001;
                        m.pos[k] = c;
                    }
                }
                if clamped { say(m, "warning: target outside the build volume, clamped"); }
                say(m, "ok");
            }
        }
        "G90" => say(m, "ok - absolute positioning"),
        "G91" => say(m, "ok - relative positioning"),
        "G29" => {
            m.remesh();
            say(m, "// adaptive mesh: probing 36 points");
            say(m, "ok - bed mesh calibrated");
        }
        "G32" => {
            m.tilt = [0.001, -0.001, 0.0];
            say(m, "// gantry level: 3 Z motors, 2 iterations");
            say(m, "ok - residual < 2 um");
        }
        "M104" | "M109" | "M140" | "M190" | "M141" => {
            let (k, name) = match word.as_str() { "M104" | "M109" => (0, "hotend"), "M140" | "M190" => (1, "bed"), _ => (2, "chamber") };
            let want = num("S").unwrap_or(0.0);
            let max = m.heaters[k].max;
            if want > max {
                say(m, &format!("warning: S{:.0} exceeds the {} limit of {:.0} C, clamped", want, name, max));
            }
            m.heaters[k].target = want.clamp(0.0, max);
            say(m, &format!("ok - {} target {:.0} C", name, m.heaters[k].target));
        }
        "M105" => {
            let l = format!("ok T:{:.1} /{:.1} B:{:.1} /{:.1} C:{:.1} /{:.1}", m.heaters[0].cur, m.heaters[0].target, m.heaters[1].cur, m.heaters[1].target, m.heaters[2].cur, m.heaters[2].target);
            say(m, &l);
        }
        "M106" => { m.fans[0].pct = (num("S").unwrap_or(255.0) / 255.0 * 100.0).clamp(0.0, 100.0); say(m, "ok"); }
        "M107" => { m.fans[0].pct = 0.0; say(m, "ok"); }
        "M220" => { m.job.speed = num("S").unwrap_or(100.0).clamp(20.0, 300.0); say(m, &format!("ok - speed {:.0} %", m.job.speed)); }
        "M221" => { m.job.flow = num("S").unwrap_or(100.0).clamp(80.0, 120.0); say(m, &format!("ok - flow {:.0} %", m.job.flow)); }
        "M84" => { m.home = [false; 3]; say(m, "ok - drives disabled (EN low)"); }
        "M112" => { m.estop(); say(m, "!! emergency stop: heaters off, fault latched"); }
        "M114" => { let l = format!("ok X:{:.2} Y:{:.2} Z:{:.2} E:{:.1}", m.pos[0], m.pos[1], m.pos[2], m.pos[3]); say(m, &l); }
        "M115" => {
            say(m, "ok FIRMWARE_NAME:MACH5 1.4.2 MACHINE:Tomahawk");
            say(m, "// host ESP32-P4, motion bridge STM32, schema 1.4.2 (signed)");
        }
        "PAUSE" => {
            if m.state == MState::Printing { m.m_pause(act); say(m, "ok - print paused"); } else { say(m, "error: no print is running"); }
        }
        "RESUME" => {
            if m.state == MState::Paused { m.m_resume(act); say(m, "ok - print resumed"); } else { say(m, "error: nothing to resume"); }
        }
        "CANCEL_PRINT" => {
            if matches!(m.state, MState::Printing | MState::Paused) { m.m_cancel(act); say(m, "warning: print cancelled"); say(m, "ok"); } else { say(m, "error: no print to cancel"); }
        }
        "LOAD_FILAMENT" => {
            if m.heaters[0].cur < 170.0 { say(m, "error: hotend below minimum extrusion temperature (170 C)"); } else { say(m, "// MMU: selecting slot T0, feeding 640 mm"); say(m, "ok - filament loaded"); }
        }
        "UNLOAD_FILAMENT" => { say(m, "// MMU: retracting 640 mm"); say(m, "ok - filament unloaded"); }
        "SHAPER_CALIBRATE" => {
            say(m, "// resonance sweep 5-135 Hz on the probe ADXL");
            say(m, "ok - X: MZV 48.2 Hz, Y: EI 41.6 Hz (applied)");
        }
        "CLEAR_FAULT" => {
            if m.state == MState::Fault { m.clear_faults(); say(m, "ok - fault cleared by host command"); } else { say(m, "warning: no fault is latched"); }
        }
        "STATUS" => { let l = format!("ok state={} nodes={} schema=1.4.2", m.state.label(), m.nodes.len()); say(m, &l); }
        "HELP" => {
            say(m, "// motion: G28 G0 G1 G90 G91 G29 G32 M84 M114");
            say(m, "// heat: M104 M109 M140 M190 M141 M105 M106 M107");
            say(m, "// job: PAUSE RESUME CANCEL_PRINT M220 M221 M112");
            say(m, "// macros: LOAD_FILAMENT UNLOAD_FILAMENT SHAPER_CALIBRATE CLEAR_FAULT STATUS");
        }
        _ => say(m, &format!("error: Unknown command '{}' - not in the API schema (try HELP)", cmd)),
    }
}

pub fn print(ui: &mut Ui, m: &mut Model) {
    title(ui, "Print", "Files on SD, queue, object exclusion and power-loss recovery");
    adaptive(ui, 2, |c| {
        panel(c.at(0), "Files (SD)", |ui| {
            for (k, f) in m.files.iter().enumerate() {
                let sel = m.st.sel_file == k;
                let mut hit = false;
                card_frame(ui, ui.available_width(), sel, |ui| {
                    if ui.selectable_label(sel, RichText::new(f.name).strong().size(13.5)).clicked() { hit = true; }
                    ui.label(dim(format!("{}  -  {}h{:02}  -  {:.0} g  -  {} object(s)", f.mat, f.mins / 60, f.mins % 60, f.grams, f.objects)));
                });
                ui.add_space(4.0);
                if hit { m.st.sel_file = k; }
            }
            ui.add_space(6.0);
            let (fname, fsize, fslicer) = {
                let f = &m.files[m.st.sel_file];
                (f.name, f.size_mb, f.slicer)
            };
            kv(ui, "file_meta", &[
                ("Size", format!("{:.1} MB", fsize)),
                ("Slicer", fslicer.to_string()),
                ("Adaptive mesh", "footprint from parsed G-code".into()),
            ]);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if primary(ui, "Print") {
                    let n = fname;
                    m.job.file = n.to_string();
                    m.job.progress = 0.0;
                    m.job.elapsed = 0.0;
                    m.job.layer = 0;
                    m.state = MState::Printing;
                    m.log(Sev::Info, "job", &format!("Start {}", n));
                }
                button(ui, "Add to queue");
                button(ui, "Upload...");
                button(ui, "Delete");
            });
        });
        panel(c.at(1), "Queue", |ui| {
            egui::Grid::new("queue").num_columns(3).striped(true).spacing([14.0, 5.0]).show(ui, |ui| {
                for h in ["#", "File", "Est."] { th(ui, h); }
                ui.end_row();
                for (k, (n, e)) in [("enclosure_panel_left.gcode", "5h18"), ("gear_train_x6.gcode", "3h25")].iter().enumerate() {
                    ui.label(mono((k + 1).to_string()));
                    ui.label(*n);
                    ui.label(mono(*e));
                    ui.end_row();
                }
            });
        });
    });

    adaptive(ui, 2, |c| {
        panel(c.at(0), "Object exclusion - click an object to cancel it mid-print", |ui| {
            let side = 320.0;
            let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
            let p = ui.painter_at(rect);
            p.rect_filled(rect, crate::theme::RAD, Color32::BLACK);
            let sc = side / 350.0;
            for g in 1..7 {
                let o = g as f32 * 50.0 * sc;
                p.line_segment([rect.min + vec2(o, 0.0), rect.min + vec2(o, side)], Stroke::new(1.0_f32, Color32::from_gray(22)));
                p.line_segment([rect.min + vec2(0.0, o), rect.min + vec2(side, o)], Stroke::new(1.0_f32, Color32::from_gray(22)));
            }
            let click = if resp.clicked() { resp.interact_pointer_pos() } else { None };
            let mut toggled = None;
            for o in m.job.objects.iter() {
                let r = egui::Rect::from_min_size(rect.min + vec2(o.x * sc, o.y * sc), vec2(o.w * sc, o.h * sc));
                if o.excluded {
                    p.rect_stroke(r, crate::theme::RAD, Stroke::new(1.0_f32, FAINT), StrokeKind::Inside);
                    p.line_segment([r.left_top(), r.right_bottom()], Stroke::new(1.0_f32, FAINT));
                    p.line_segment([r.right_top(), r.left_bottom()], Stroke::new(1.0_f32, FAINT));
                } else {
                    p.rect_filled(r, crate::theme::RAD, Color32::from_gray(70));
                    p.rect_stroke(r, crate::theme::RAD, Stroke::new(1.0_f32, Color32::from_gray(200)), StrokeKind::Inside);
                }
                p.text(r.center(), Align2::CENTER_CENTER, o.name, FontId::monospace(10.0), if o.excluded { FAINT } else { WHITE });
                if let Some(pt) = click { if r.contains(pt) { toggled = Some(o.id); } }
            }
            p.rect_stroke(rect, crate::theme::RAD, Stroke::new(1.0_f32, LINE2), StrokeKind::Inside);
            if let Some(id) = toggled {
                let o = &mut m.job.objects[id];
                o.excluded = !o.excluded;
                let msg = format!("{} {}", if o.excluded { "Excluded" } else { "Restored" }, o.name);
                m.log(Sev::Info, "exclude", &msg);
            }
            ui.label(dim("350 x 350 mm bed. Boundaries come from G-code metadata."));
        });
        panel(c.at(1), "Objects", |ui| {
            egui::Grid::new("objs").num_columns(3).striped(true).spacing([16.0, 5.0]).show(ui, |ui| {
                for h in ["Name", "Footprint", "State"] { th(ui, h); }
                ui.end_row();
                for o in m.job.objects.iter_mut() {
                    ui.label(mono(o.name));
                    ui.label(mono(format!("{:.0} x {:.0}", o.w, o.h)));
                    let lbl = if o.excluded { "Restore" } else { "Exclude" };
                    if crate::ui::sbtn(ui, lbl).clicked() { o.excluded = !o.excluded; }
                    ui.end_row();
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("POWER-LOSS RECOVERY").size(11.0).color(DIM).monospace());
            ui.add_space(4.0);
            kv(ui, "plr", &[
                ("Checkpoint", format!("layer {} / {} (flash)", m.job.layer, m.job.layers)),
                ("Z restored", format!("{:.2} mm", m.pos[2])),
                ("Thermal state", format!("hotend {:.0}, bed {:.0}, chamber {:.0}", m.heaters[0].cur, m.heaters[1].cur, m.heaters[2].cur)),
                ("Written", "every layer, wear-levelled".into()),
            ]);
            ui.horizontal(|ui| {
                ui.add_enabled_ui(m.resume_ready && m.state != MState::Printing, |ui| {
                    if primary(ui, "Resume from checkpoint") {
                        m.state = MState::Printing;
                        m.log(Sev::Info, "plr", "Resuming with Z and thermal state restored");
                    }
                });
                if button(ui, "Discard checkpoint") { m.resume_ready = false; }
            });
        });
    });
}

pub fn thermal(ui: &mut Ui, m: &mut Model) {
    title(ui, "Thermal", "Heaters, fans, control mode and cutoff limits");
    panel(ui, "Heaters", |ui| {
        egui::Grid::new("heaters").num_columns(9).striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
            for h in ["Name", "Node", "Current", "Target", "Output", "Control", "Max", "Watchdog", "State"] { th(ui, h); }
            ui.end_row();
            for h in m.heaters.iter_mut() {
                ui.label(h.name);
                ui.label(dim(h.node));
                ui.label(mono(format!("{:.1} C", h.cur)).color(WHITE));
                ui.add(DragValue::new(&mut h.target).range(0.0..=h.max).suffix(" C"));
                ui.horizontal(|ui| { bar(ui, h.power / 100.0, 50.0, None); ui.label(mono(format!("{:.0}%", h.power))); });
                egui::ComboBox::from_id_salt(h.name).width(70.0).selected_text(["PID", "MPC"][h.ctl]).show_ui(ui, |ui| {
                    ui.selectable_value(&mut h.ctl, 0, "PID");
                    ui.selectable_value(&mut h.ctl, 1, "MPC");
                });
                ui.add(DragValue::new(&mut h.max).range(50.0..=500.0).suffix(" C"));
                ui.add(DragValue::new(&mut h.wd_ms).range(100..=10000).suffix(" ms"));
                let near = (h.target - h.cur).abs() < 2.0;
                status(ui, if h.target == 0.0 { Status::Idle } else if near { Status::Ok } else { Status::Warn }, if h.target == 0.0 { "off" } else if near { "stable" } else { "heating" });
                ui.end_row();
            }
        });
        ui.add_space(4.0);
        ui.label(dim("A missing Host heartbeat cuts the heater locally on the node. Max temperature is enforced on the STM32 and the node, independent of the P4."));
        ui.horizontal(|ui| {
            for n in ["Auto-tune hotend", "Auto-tune bed", "Auto-tune chamber"] { button(ui, n); }
        });
    });
    adaptive(ui, 2, |c| {
        panel(c.at(0), "History", |ui| {
            plot(ui, 220.0, &[
                Series::from_ring("hotend", G1, &m.h_hot),
                Series::from_ring("target", G3, &m.h_t_hot).dashed(),
                Series::from_ring("bed", G2, &m.h_bed),
                Series::from_ring("chamber", G3, &m.h_cham),
            ], (0.0, HIST as f32), (0.0, 300.0), "deg C");
        });
        panel(c.at(1), "Fans", |ui| {
            ui.horizontal_wrapped(|ui| {
                for f in m.fans.iter_mut() {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        knob(ui, &mut f.pct, (0.0, 100.0), f.name.split(' ').next().unwrap_or(f.name), "%", 0);
                        ui.label(mono(format!("{:.0} rpm", f.rpm_max * f.pct / 100.0)).size(10.0).color(DIM));
                        ui.label(dim(f.node).size(9.5));
                    });
                }
            });
            ui.add_space(4.0);
            ui.label(dim("Drag a knob up or down. Shift for fine control."));
        });
    });
}
