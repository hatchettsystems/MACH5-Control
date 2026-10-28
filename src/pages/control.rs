use crate::data::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align2, Color32, DragValue, FontId, RichText, Sense, Stroke, StrokeKind, Ui};

fn state_actions(ui: &mut Ui, m: &mut Model) {
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

pub fn dashboard(ui: &mut Ui, m: &mut Model) {
    title(ui, "Dashboard", &format!("{} - live telemetry, mock data", m.name));
    ui.columns(3, |c| {
        panel(&mut c[0], "Job", |ui| {
            ui.label(mono(m.job.file).color(WHITE));
            ui.add_space(2.0);
            bar_full(ui, m.job.progress, None);
            let rem = m.job.total - m.job.elapsed;
            kv(ui, "dash_job", &[
                ("Progress", format!("{:.1} %", m.job.progress * 100.0)),
                ("Layer", format!("{} / {}", m.job.layer, m.job.layers)),
                ("Elapsed", fmt_dur(m.job.elapsed)),
                ("Remaining", fmt_dur(rem)),
                ("Objects", format!("{} active / {} total", m.job.objects.iter().filter(|o| !o.excluded).count(), m.job.objects.len())),
            ]);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(dim("Speed"));
                ui.add(DragValue::new(&mut m.job.speed).range(20.0..=300.0).suffix(" %"));
                ui.label(dim("Flow"));
                ui.add(DragValue::new(&mut m.job.flow).range(80.0..=120.0).suffix(" %"));
            });
            ui.add_space(4.0);
            state_actions(ui, m);
        });
        panel(&mut c[1], "Temperatures", |ui| {
            egui::Grid::new("dash_temps").num_columns(4).spacing([14.0, 6.0]).show(ui, |ui| {
                for h in &m.heaters {
                    ui.label(dim(h.name));
                    ui.label(mono(format!("{:6.1}", h.cur)).size(15.0).color(WHITE));
                    ui.label(mono(format!("/ {:.0} C", h.target)).color(DIM));
                    bar(ui, h.power / 100.0, 60.0, None);
                    ui.end_row();
                }
            });
            ui.add_space(6.0);
            ui.label(dim("Bar = heater output. Cutoff limits are enforced on the STM32 and the node itself."));
        });
        panel(&mut c[2], "Position", |ui| {
            egui::Grid::new("dash_pos").num_columns(3).spacing([14.0, 4.0]).show(ui, |ui| {
                for (k, (ax, v)) in [("X", m.pos[0]), ("Y", m.pos[1]), ("Z", m.pos[2]), ("E", m.pos[3])].iter().enumerate() {
                    ui.label(dim(*ax));
                    ui.label(mono(format!("{:8.2}", v)).size(15.0).color(WHITE));
                    if k < 3 {
                        status(ui, if m.home[k] { Status::Ok } else { Status::Warn }, if m.home[k] { "homed" } else { "not homed" });
                    }
                    ui.end_row();
                }
            });
            ui.add_space(4.0);
            kv(ui, "dash_mot", &[
                ("Velocity", format!("{:.0} mm/s", 180.0 * m.job.speed / 100.0)),
                ("Accel limit", format!("{:.0} mm/s2", m.accel[0])),
                ("Shaper X/Y", format!("{} {:.1} / {} {:.1} Hz", SHAPERS[m.shapers[0].kind], m.shapers[0].freq, SHAPERS[m.shapers[1].kind], m.shapers[1].freq)),
                ("Pressure adv.", format!("{:.3} s", m.pa_value)),
            ]);
        });
    });

    ui.columns(2, |c| {
        panel(&mut c[0], "Thermal history (60 s)", |ui| {
            plot(ui, 170.0, &[
                Series::from_ring("hotend", G1, &m.h_hot),
                Series::from_ring("target", G3, &m.h_t_hot).dashed(),
                Series::from_ring("bed", G2, &m.h_bed),
                Series::from_ring("chamber", G3, &m.h_cham),
            ], (0.0, HIST as f32), (0.0, 300.0), "deg C");
        });
        panel(&mut c[1], "Safety", |ui| {
            let sync = *m.h_sync.back().unwrap_or(&0.0);
            let nodes_ok = m.nodes.iter().filter(|n| matches!(n.status, Status::Ok | Status::Idle)).count();
            egui::Grid::new("dash_safe").num_columns(2).spacing([20.0, 5.0]).show(ui, |ui| {
                ui.label(dim("Heater watchdog (STM32)"));
                status(ui, Status::Ok, "armed, 0 trips");
                ui.end_row();
                ui.label(dim("Node heartbeats"));
                status(ui, if nodes_ok == m.nodes.len() { Status::Ok } else { Status::Warn }, &format!("{}/{} responding", nodes_ok, m.nodes.len()));
                ui.end_row();
                ui.label(dim("Master time sync"));
                status(ui, if sync < 80.0 { Status::Ok } else { Status::Warn }, &format!("skew {:.0} us (limit 100)", sync));
                ui.end_row();
                ui.label(dim("Drive faults"));
                status(ui, if m.state == MState::Fault { Status::Err } else { Status::Ok }, if m.state == MState::Fault { "latched" } else { "none" });
                ui.end_row();
                ui.label(dim("Power-loss checkpoint"));
                status(ui, Status::Ok, &format!("layer {}", m.job.layer));
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_enabled_ui(m.clear_needed || m.state == MState::Fault, |ui| {
                    if button(ui, "Clear faults") {
                        m.clear_faults();
                    }
                });
                ui.label(dim("Nodes never clear a fault without a Host command."));
            });
        });
    });

    ui.columns(2, |c| {
        panel(&mut c[0], "Node health", |ui| {
            egui::Grid::new("dash_nodes").num_columns(4).spacing([18.0, 4.0]).show(ui, |ui| {
                for (k, n) in m.nodes.iter().enumerate() {
                    status(ui, n.status, n.name);
                    ui.label(dim(n.seg.label()));
                    ui.label(mono(format!("{:.1} Hz", n.hb)));
                    if ui.small_button("open").clicked() {
                        m.st.sel_node = k;
                        m.goto = Some(Page::Nodes);
                    }
                    ui.end_row();
                }
            });
        });
        panel(&mut c[1], "Recent events", |ui| {
            for l in m.logs.iter().rev().take(9) {
                ui.horizontal(|ui| {
                    ui.label(mono(l.ts.clone()).color(FAINT));
                    let st = match l.sev { Sev::Info => Status::Idle, Sev::Warn => Status::Warn, Sev::Err => Status::Err };
                    dot(ui, st);
                    ui.label(dim(l.src.clone()));
                    ui.label(l.msg.clone());
                });
            }
            if button(ui, "Open event log") {
                m.goto = Some(Page::Logs);
            }
        });
    });
}

pub const SHAPERS: [&str; 5] = ["ZV", "MZV", "ZVD", "EI", "2HUMP_EI"];

pub fn control(ui: &mut Ui, m: &mut Model) {
    title(ui, "Control", "Manual motion, temperatures, extrusion and console");
    ui.columns(2, |c| {
        panel(&mut c[0], "Jog", |ui| {
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
            let bs = vec2(54.0, 30.0);
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
        panel(&mut c[1], "Temperatures and extrusion", |ui| {
            egui::Grid::new("ctl_temps").num_columns(4).spacing([14.0, 6.0]).show(ui, |ui| {
                for h in m.heaters.iter_mut() {
                    ui.label(dim(h.name));
                    ui.label(mono(format!("{:.1}", h.cur)).color(WHITE));
                    ui.add(DragValue::new(&mut h.target).range(0.0..=h.max).suffix(" C"));
                    if ui.small_button("off").clicked() { h.target = 0.0; }
                    ui.end_row();
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(dim("Preset"));
                for p in m.profiles.iter() {
                    if ui.small_button(p.name).clicked() {
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

fn run_cmd(m: &mut Model, cmd: &str) {
    m.st.console.push(format!("> {}", cmd));
    let up = cmd.to_uppercase();
    let num = |p: &str| up.split_whitespace().find_map(|w| w.strip_prefix(p)).and_then(|v| v.parse::<f32>().ok());
    if up.starts_with("G28") {
        m.home = [true; 3];
        m.pos[..3].fill(0.0);
        m.st.console.push("ok - homed X Y Z".into());
    } else if up.starts_with("M104") {
        let t = num("S").unwrap_or(0.0).clamp(0.0, m.heaters[0].max);
        m.heaters[0].target = t;
        m.st.console.push(format!("ok - hotend target {:.0}", t));
    } else if up.starts_with("M140") {
        let t = num("S").unwrap_or(0.0).clamp(0.0, m.heaters[1].max);
        m.heaters[1].target = t;
        m.st.console.push(format!("ok - bed target {:.0}", t));
    } else if up.starts_with("M112") {
        m.estop();
        m.st.console.push("!! emergency stop".into());
    } else if up.starts_with("HELP") {
        m.st.console.push("typed commands: G28, G0/G1, M104, M140, M112, STATUS".into());
    } else if up.starts_with("STATUS") {
        m.st.console.push(format!("state={} nodes={} schema=1.4.2", m.state.label(), m.nodes.len()));
    } else {
        m.st.console.push("error: command not in API schema".into());
    }
}

pub fn print(ui: &mut Ui, m: &mut Model) {
    title(ui, "Print", "Files on SD, queue, object exclusion and power-loss recovery");
    ui.columns(2, |c| {
        panel(&mut c[0], "Files (SD)", |ui| {
            egui::Grid::new("files").num_columns(5).striped(true).spacing([14.0, 5.0]).show(ui, |ui| {
                for h in ["Name", "Material", "Time", "Filament", "Objects"] { th(ui, h); }
                ui.end_row();
                for (k, f) in m.files.iter().enumerate() {
                    if ui.selectable_label(m.st.sel_file == k, f.name).clicked() { m.st.sel_file = k; }
                    ui.label(f.mat);
                    ui.label(mono(format!("{}h{:02}", f.mins / 60, f.mins % 60)));
                    ui.label(mono(format!("{:.0} g", f.grams)));
                    ui.label(mono(f.objects.to_string()));
                    ui.end_row();
                }
            });
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
                    m.job.file = n;
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
        panel(&mut c[1], "Queue", |ui| {
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

    ui.columns(2, |c| {
        panel(&mut c[0], "Object exclusion - click an object to cancel it mid-print", |ui| {
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
        panel(&mut c[1], "Objects", |ui| {
            egui::Grid::new("objs").num_columns(3).striped(true).spacing([16.0, 5.0]).show(ui, |ui| {
                for h in ["Name", "Footprint", "State"] { th(ui, h); }
                ui.end_row();
                for o in m.job.objects.iter_mut() {
                    ui.label(mono(o.name));
                    ui.label(mono(format!("{:.0} x {:.0}", o.w, o.h)));
                    let lbl = if o.excluded { "Restore" } else { "Exclude" };
                    if ui.small_button(lbl).clicked() { o.excluded = !o.excluded; }
                    ui.end_row();
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("POWER-LOSS RECOVERY").size(11.0).color(DIM).monospace());
            ui.add_space(4.0);
            kv(ui, "plr", &[
                ("Checkpoint", format!("layer {} / {} (flash)", m.job.layer, m.job.layers)),
                ("Z restored", format!("{:.2} mm", m.pos[2])),
                ("Thermal state", "hotend 250, bed 90, chamber 52".into()),
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
    ui.columns(2, |c| {
        panel(&mut c[0], "History", |ui| {
            plot(ui, 220.0, &[
                Series::from_ring("hotend", G1, &m.h_hot),
                Series::from_ring("target", G3, &m.h_t_hot).dashed(),
                Series::from_ring("bed", G2, &m.h_bed),
                Series::from_ring("chamber", G3, &m.h_cham),
            ], (0.0, HIST as f32), (0.0, 300.0), "deg C");
        });
        panel(&mut c[1], "Fans", |ui| {
            egui::Grid::new("fans").num_columns(4).spacing([14.0, 8.0]).show(ui, |ui| {
                for h in ["Fan", "Node", "Speed", "RPM"] { th(ui, h); }
                ui.end_row();
                for f in m.fans.iter_mut() {
                    ui.label(f.name);
                    ui.label(dim(f.node));
                    ui.add(egui::Slider::new(&mut f.pct, 0.0..=100.0).suffix(" %"));
                    ui.label(mono(format!("{:.0}", f.rpm_max * f.pct / 100.0)));
                    ui.end_row();
                }
            });
        });
    });
}
