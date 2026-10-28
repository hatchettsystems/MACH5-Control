use crate::data::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align2, Color32, DragValue, FontId, RichText, Sense, Stroke, StrokeKind, Ui};

fn invalid(f: &Field) -> bool {
    match &f.val {
        Val::F(v) => *v < f.min || *v > f.max,
        Val::I(v) => (*v as f64) < f.min || (*v as f64) > f.max,
        Val::S(s) => s.trim().is_empty(),
        _ => false,
    }
}

pub fn config(ui: &mut Ui, m: &mut Model) {
    title(ui, "Config", "Typed schema, edited from the machine's own schema. Every field is validated before it is written with ack and readback");
    let dirty: usize = m.config.iter().map(|s| s.fields.iter().filter(|f| f.val != f.orig).count()).sum();
    let bad: usize = m.config.iter().map(|s| s.fields.iter().filter(|f| invalid(f)).count()).sum();
    ui.horizontal(|ui| {
        ui.label(dim("Schema 1.4.2  hash 5e1a9c  generated from shared Rust types"));
        ui.add_space(16.0);
        ui.label(mono(format!("{} modified", dirty)).color(if dirty > 0 { WHITE } else { DIM }));
        if bad > 0 { ui.label(RichText::new(format!("{} invalid", bad)).color(ERR)); }
        ui.add_enabled_ui(dirty > 0 && bad == 0, |ui| {
            if primary(ui, "Apply and write") {
                for s in m.config.iter_mut() { for f in s.fields.iter_mut() { f.orig = f.val.clone(); } }
                m.log(Sev::Info, "config", &format!("{} field(s) written, readback verified", dirty));
            }
        });
        ui.add_enabled_ui(dirty > 0, |ui| {
            if button(ui, "Revert") { for s in m.config.iter_mut() { for f in s.fields.iter_mut() { f.val = f.orig.clone(); } } }
        });
        button(ui, "Export TOML");
        button(ui, "Import...");
    });
    ui.add_space(8.0);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(vec2(200.0, 10.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            panel(ui, "Sections", |ui| {
                for (k, s) in m.config.iter().enumerate() {
                    let n = s.fields.iter().filter(|f| f.val != f.orig).count();
                    let lbl = if n > 0 { format!("{}  ({} modified)", s.name, n) } else { s.name.to_string() };
                    if ui.selectable_label(m.st.sel_sec == k, lbl).clicked() { m.st.sel_sec = k; }
                }
            });
        });
        ui.vertical(|ui| {
            let k = m.st.sel_sec.min(m.config.len() - 1);
            panel(ui, &format!("[{}]", m.config[k].name), |ui| {
                egui::Grid::new("cfg").num_columns(4).striped(true).spacing([18.0, 8.0]).show(ui, |ui| {
                    for h in ["Key", "Value", "Unit", "Description"] { th(ui, h); }
                    ui.end_row();
                    for f in m.config[k].fields.iter_mut() {
                        let changed = f.val != f.orig;
                        ui.label(mono(f.key).color(if changed { WHITE } else { TEXT }));
                        let bad = invalid(f);
                        let (min, max) = (f.min, f.max);
                        match &mut f.val {
                            Val::F(v) => { ui.add(DragValue::new(v).speed(((max - min) / 400.0).max(0.0005))); }
                            Val::I(v) => { ui.add(DragValue::new(v).speed(((max - min) / 400.0).max(1.0))); }
                            Val::B(v) => { ui.checkbox(v, ""); }
                            Val::S(v) => { ui.add(egui::TextEdit::singleline(v).desired_width(140.0)); }
                            Val::E(i, opts) => {
                                egui::ComboBox::from_id_salt(f.key).selected_text(opts[*i]).show_ui(ui, |ui| {
                                    for (n, o) in opts.iter().enumerate() { ui.selectable_value(i, n, *o); }
                                });
                            }
                        }
                        ui.label(dim(f.unit));
                        if bad { ui.label(RichText::new(format!("out of range {} .. {}", min, max)).color(ERR)); } else { wrap(ui, 360.0, dim(f.desc)); }
                        ui.end_row();
                    }
                });
            });
        });
    });
}

const TRIGGERS: [&str; 5] = ["Job start", "Sensor event", "Material change", "Spool level", "Idle timer"];

pub fn rules(ui: &mut Ui, m: &mut Model) {
    title(ui, "Rules", "Sandboxed rules engine for user logic. Triggers and macros replace shell-command macros");
    ui.columns(2, |c| {
        panel(&mut c[0], "Rules", |ui| {
            egui::Grid::new("rules").num_columns(4).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
                for h in ["On", "Name", "Trigger", "Runs"] { th(ui, h); }
                ui.end_row();
                for (k, r) in m.rules.iter_mut().enumerate() {
                    ui.checkbox(&mut r.on, "");
                    if ui.selectable_label(m.st.sel_rule == k, r.name).clicked() { m.st.sel_rule = k; }
                    ui.label(dim(TRIGGERS[r.trigger]));
                    ui.label(mono(r.runs.to_string()));
                    ui.end_row();
                }
            });
            ui.add_space(6.0);
            if button(ui, "New rule") {
                m.rules.push(Rule { name: "New rule", on: false, trigger: 0, cond: "true".into(), actions: vec!["notify(\"hello\")".into()], runs: 0 });
                m.st.sel_rule = m.rules.len() - 1;
            }
        });
        let k = m.st.sel_rule.min(m.rules.len() - 1);
        panel(&mut c[1], "Editor", |ui| {
            let r = &mut m.rules[k];
            egui::Grid::new("rule_ed").num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
                ui.label(dim("Trigger"));
                egui::ComboBox::from_id_salt("rtrig").selected_text(TRIGGERS[r.trigger]).show_ui(ui, |ui| {
                    for (i, t) in TRIGGERS.iter().enumerate() { ui.selectable_value(&mut r.trigger, i, *t); }
                });
                ui.end_row();
                ui.label(dim("Condition"));
                ui.add(egui::TextEdit::singleline(&mut r.cond).desired_width(f32::INFINITY).font(FontId::monospace(12.5)));
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.label(dim("Actions (run in order)"));
            let mut rm = None;
            for (i, a) in r.actions.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(mono(format!("{}", i + 1)).color(FAINT));
                    ui.add(egui::TextEdit::singleline(a).desired_width(ui.available_width() - 40.0).font(FontId::monospace(12.5)));
                    if ui.small_button("x").clicked() { rm = Some(i); }
                });
            }
            if let Some(i) = rm { r.actions.remove(i); }
            ui.horizontal(|ui| {
                if button(ui, "Add action") { r.actions.push("".into()); }
                if button(ui, "Dry run") { let msg = format!("dry run '{}': condition evaluated, {} action(s) would run", r.name, r.actions.len()); m.st.console.push(msg); }
            });
            if let Some(l) = m.st.console.last() { if l.starts_with("dry run") { ui.label(dim(l.clone())); } }
        });
    });
    panel(ui, "Sandbox", |ui| {
        kv(ui, "sandbox", &[
            ("Scope", "Triggers and macros only (decision pending: general scripting)".into()),
            ("CPU budget", "2 ms per evaluation, runs on P4 below planner priority".into()),
            ("Memory", "16 KB per rule".into()),
            ("Shell / filesystem", "no access".into()),
            ("Allowed actions", "pause, resume, notify, set heater/fan, wait, move (within limits)".into()),
            ("Safety limits", "cannot override heater cutoffs, drive faults or e-stop".into()),
        ]);
    });
}

pub fn api(ui: &mut Ui, m: &mut Model) {
    title(ui, "API & Security", "Versioned, schema-generated, per-client keys. The API is the only way in");
    tabs(ui, &mut m.st.tab, &["Clients", "Endpoints", "Keys", "Remote views"]);
    match m.st.tab {
        0 => {
            panel(ui, "Paired clients", |ui| {
                egui::Grid::new("clients").num_columns(7).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
                    for h in ["Name", "Kind", "Key fingerprint", "Scope", "Last seen", "Key age", ""] { th(ui, h); }
                    ui.end_row();
                    let mut act = None;
                    for (k, cl) in m.clients.iter().enumerate() {
                        ui.label(if cl.revoked { RichText::new(cl.name).color(FAINT).strikethrough() } else { RichText::new(cl.name) });
                        ui.label(dim(cl.kind));
                        ui.label(mono(cl.fp));
                        ui.label(mono(cl.scope));
                        ui.label(dim(cl.seen));
                        ui.label(mono(format!("{} d", cl.age)));
                        ui.horizontal(|ui| {
                            if !cl.revoked {
                                if ui.small_button("rotate").clicked() { act = Some((k, 0)); }
                                if ui.small_button("revoke").clicked() { act = Some((k, 1)); }
                            } else { ui.label(dim("revoked")); }
                        });
                        ui.end_row();
                    }
                    if let Some((k, a)) = act {
                        if a == 0 { m.clients[k].age = 0; let nm = m.clients[k].name; m.log(Sev::Info, "api", &format!("Key rotated for {}", nm)); }
                        else { m.clients[k].revoked = true; let nm = m.clients[k].name; m.log(Sev::Warn, "api", &format!("Client revoked: {}", nm)); }
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut m.st.new_client).hint_text("new client name").desired_width(200.0));
                    if button(ui, "Generate pairing key") && !m.st.new_client.trim().is_empty() {
                        let nm: &'static str = Box::leak(std::mem::take(&mut m.st.new_client).into_boxed_str());
                        m.clients.push(Client { name: nm, kind: "Management Interface", fp: "SHA256:new0...0000", scope: "operate", seen: "never", revoked: false, age: 0 });
                        m.log(Sev::Info, "api", "Pairing key generated (shown once)");
                    }
                });
            });
        }
        1 => {
            panel(ui, "Endpoints (generated from shared types)", |ui| {
                egui::Grid::new("endpoints").num_columns(4).striped(true).spacing([18.0, 5.0]).show(ui, |ui| {
                    for h in ["Method", "Path", "Feature", "Auth"] { th(ui, h); }
                    ui.end_row();
                    for (me, pa, ft, au) in [
                        ("WS", "/v1/telemetry", "Live telemetry stream", "view"),
                        ("GET", "/v1/schema", "Signed config + API schema", "view"),
                        ("PUT", "/v1/config/{section}/{key}", "Typed config write (ack + readback)", "admin"),
                        ("POST", "/v1/jobs", "Start job / queue", "operate"),
                        ("POST", "/v1/jobs/current/exclude", "Object exclusion", "operate"),
                        ("POST", "/v1/jobs/current/pause", "Pause (failure detection uses this)", "pause"),
                        ("POST", "/v1/calibration/auto", "Unified auto-calibration", "operate"),
                        ("POST", "/v1/calibration/resonance", "Resonance test, per-axis shapers", "operate"),
                        ("GET", "/v1/mesh", "Bed mesh data", "view"),
                        ("GET/PUT", "/v1/spools", "Spool database sync", "operate"),
                        ("GET", "/v1/camera/stream", "Video stream and snapshots", "view"),
                        ("POST", "/v1/nodes/{id}/firmware", "Stage signed image", "admin"),
                        ("GET/PUT", "/v1/rules", "Rules engine", "admin"),
                        ("POST", "/v1/faults/clear", "Host fault clear command", "admin"),
                    ] {
                        ui.label(mono(me)); ui.label(mono(pa)); ui.label(dim(ft)); ui.label(mono(au));
                        ui.end_row();
                    }
                });
            });
        }
        2 => {
            ui.columns(2, |c| {
                panel(&mut c[0], "Signing and provisioning", |ui| {
                    kv(ui, "keys", &[
                        ("Release signing key", "MACH5-REL-02 (ed25519)".into()),
                        ("Previous key", "MACH5-REL-01 - accepted until 2027-01-01".into()),
                        ("API schema signature", "valid".into()),
                        ("Client key policy", "per-client, rotate every 90 d".into()),
                    ]);
                    ui.add_space(4.0);
                    ui.label(dim("Key provisioning and rotation procedure is an open decision."));
                });
                panel(&mut c[1], "Clients due for rotation", |ui| {
                    for cl in m.clients.iter().filter(|c| !c.revoked && c.age > 80) {
                        ui.horizontal(|ui| { dot(ui, Status::Warn); ui.label(cl.name); ui.label(dim(format!("{} d", cl.age))); });
                    }
                });
            });
        }
        _ => {
            ui.columns(2, |c| {
                panel(&mut c[0], "On-machine screen (DSI) - mirrored through the API", |ui| {
                    let w = ui.available_width().min(560.0);
                    let (r, _) = ui.allocate_exact_size(vec2(w, w * 0.6), Sense::hover());
                    let p = ui.painter_at(r);
                    p.rect_filled(r, crate::theme::RAD, Color32::BLACK);
                    p.rect_stroke(r, crate::theme::RAD, Stroke::new(1.0_f32, LINE2), StrokeKind::Inside);
                    p.text(r.left_top() + vec2(12.0, 10.0), Align2::LEFT_TOP, format!("{}   {}", m.name, m.state.label()), FontId::monospace(13.0), WHITE);
                    p.text(r.left_top() + vec2(12.0, 40.0), Align2::LEFT_TOP, format!("{:.0}%  layer {}/{}", m.job.progress * 100.0, m.job.layer, m.job.layers), FontId::monospace(12.0), TEXT);
                    let b = egui::Rect::from_min_size(r.left_top() + vec2(12.0, 66.0), vec2(r.width() - 24.0, 8.0));
                    p.rect_stroke(b, crate::theme::RAD, Stroke::new(1.0_f32, LINE2), StrokeKind::Inside);
                    p.rect_filled(egui::Rect::from_min_size(b.min, vec2(b.width() * m.job.progress, 8.0)).shrink(1.0), crate::theme::RAD, Color32::from_gray(200));
                    for (i, h) in m.heaters.iter().enumerate() {
                        p.text(r.left_top() + vec2(12.0 + i as f32 * 150.0, 96.0), Align2::LEFT_TOP, format!("{} {:.0}/{:.0}", h.name, h.cur, h.target), FontId::monospace(11.0), DIM);
                    }
                });
                panel(&mut c[1], "Other views", |ui| {
                    for (n, d) in [("Phone / tablet", "Ports of the Management Interface, paired with per-client keys"), ("Timelapse", "View of camera frames captured per job"), ("Remote screen", "Mirror of the on-machine DSI UI")] {
                        ui.label(RichText::new(n).color(WHITE));
                        ui.label(dim(d));
                        ui.add_space(4.0);
                    }
                });
            });
        }
    }
}

pub fn simulator(ui: &mut Ui, m: &mut Model) {
    title(ui, "Simulator", "Host tooling: planner and a virtual CAN bus on the PC, so firmware is tested before hardware exists");
    ui.columns(2, |c| {
        panel(&mut c[0], "Planner run", |ui| {
            egui::Grid::new("sim_cfg").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                ui.label(dim("G-code"));
                ui.label(mono(m.files[m.st.sel_file].name));
                ui.end_row();
                ui.label(dim("Reference"));
                ui.label(mono("klipper_run_2026-10-21.json"));
                ui.end_row();
                ui.label(dim("Virtual CAN"));
                ui.label(mono("2 segments, 13 virtual nodes"));
                ui.end_row();
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if primary(ui, "Run simulation") { m.sim.running = true; m.sim.prog = 0.0; m.sim.results.clear(); m.sim.log.push("planner run started".into()); }
                if m.sim.running { bar(ui, m.sim.prog, 120.0, None); }
            });
        });
        panel(&mut c[1], "Fault injection", |ui| {
            for (k, (n, e)) in [("Heater loss", "node cuts heater; Host reports fault"), ("CAN drop (segment A)", "controlled stop on missing time sync"), ("Power loss", "checkpoint written; resume available"), ("Drive over-current", "drive disables; fault latched")].iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut m.sim.faults[k], *n);
                    ui.label(dim(*e));
                });
            }
            if button(ui, "Inject selected") {
                for (k, n) in ["heater loss", "CAN drop", "power loss", "drive over-current"].iter().enumerate() {
                    if m.sim.faults[k] { m.sim.log.push(format!("injected {} -> safe state reached", n)); }
                }
            }
        });
    });
    panel(ui, "Planner vs reference (phase 1 gate: within defined tolerance)", |ui| {
        if m.sim.results.is_empty() { ui.label(dim("No results. Run the simulation.")); }
        else {
            egui::Grid::new("sim_res").num_columns(6).striped(true).spacing([20.0, 5.0]).show(ui, |ui| {
                for h in ["Metric", "Reference", "Planner", "Delta", "Tolerance", "Result"] { th(ui, h); }
                ui.end_row();
                for (n, r, s, t) in &m.sim.results {
                    let d = (s - r).abs() / r * 100.0;
                    ui.label(*n); ui.label(mono(format!("{:.1}", r))); ui.label(mono(format!("{:.1}", s)));
                    ui.label(mono(format!("{:.2} %", d))); ui.label(mono(format!("{:.1} %", t)));
                    status(ui, if d <= *t { Status::Ok } else { Status::Err }, if d <= *t { "pass" } else { "fail" });
                    ui.end_row();
                }
            });
        }
    });
    panel(ui, "Log", |ui| {
        for l in m.sim.log.iter().rev().take(8) { ui.label(mono(l.clone()).color(DIM)); }
        if m.sim.log.is_empty() { ui.label(dim("-")); }
    });
}

pub fn logs(ui: &mut Ui, m: &mut Model) {
    title(ui, "Event Log", "Safety, job, bus and API events");
    ui.horizontal(|ui| {
        for (k, n) in ["All", "Info", "Warn", "Error"].iter().enumerate() {
            if ui.selectable_label(m.st.log_filter == k, *n).clicked() { m.st.log_filter = k; }
        }
        ui.add(egui::TextEdit::singleline(&mut m.st.log_q).hint_text("search").desired_width(220.0));
        button(ui, "Export");
    });
    ui.add_space(6.0);
    panel(ui, "", |ui| {
        egui::Grid::new("logs").num_columns(4).striped(true).spacing([18.0, 4.0]).show(ui, |ui| {
            for h in ["Time", "Level", "Source", "Message"] { th(ui, h); }
            ui.end_row();
            let q = m.st.log_q.to_lowercase();
            for l in m.logs.iter().rev() {
                let ok = match m.st.log_filter { 1 => l.sev == Sev::Info, 2 => l.sev == Sev::Warn, 3 => l.sev == Sev::Err, _ => true };
                if !ok || (!q.is_empty() && !l.msg.to_lowercase().contains(&q) && !l.src.to_lowercase().contains(&q)) { continue; }
                ui.label(mono(l.ts.clone()).color(FAINT));
                match l.sev { Sev::Info => status(ui, Status::Idle, "info"), Sev::Warn => status(ui, Status::Warn, "warn"), Sev::Err => status(ui, Status::Err, "error") }
                ui.label(dim(l.src.clone()));
                ui.label(l.msg.clone());
                ui.end_row();
            }
        });
    });
}

const FEATURES: [(&str, &str, &str, Page); 12] = [
    ("Resonance testing and auto input-shaper", "Shake&Tune, Klippain", "P4 with ADXL data from CAN nodes; per-axis results applied automatically", Page::Calibration),
    ("Adaptive bed mesh and purge", "KAMP", "Mesh and purge limited to print footprint from parsed G-code", Page::Calibration),
    ("Eddy probing with thermal compensation", "Beacon, Cartographer", "Own probe; homing, mesh and X-twist as one auto-calibration", Page::Probe),
    ("Driver tuning", "TMC autotune", "FOC drivers self-characterise the motor at setup", Page::Drives),
    ("Multi-material and filament handling", "Happy Hare", "MMU is a CAN node class with a native state machine", Page::Materials),
    ("Spool inventory", "Spoolman", "Spool database lives in the machine", Page::Materials),
    ("Object exclusion", "Exclude Object", "Planner-level boundaries, cancellable mid-print", Page::Print),
    ("Failure detection", "Obico, spaghetti detection", "Camera streams to the app; detection on the PC pauses via API", Page::Camera),
    ("Power-loss recovery", "Community macros", "Checkpoint to flash; resume with Z and thermal state", Page::Print),
    ("Gantry leveling", "Z tilt, QGL", "Probe-driven leveling for any Z-motor count", Page::Calibration),
    ("Pressure advance calibration", "Mainline, calculators", "Print generated and analysed in the app; written to profile", Page::Calibration),
    ("Config and macros", "Klipper config, Kalico", "Typed schema editor; sandboxed rules engine", Page::Config),
];

pub fn spec(ui: &mut Ui, m: &mut Model) {
    title(ui, "Spec & Roadmap", "MACH5 Ecosystem: Spec & Design Plan - 2026-10-28");
    tabs(ui, &mut m.st.tab, &["Roadmap", "Feature coverage", "Principles", "Architecture", "Open decisions"]);
    match m.st.tab {
        0 => panel(ui, "Roadmap - each phase ends in a gate that must pass before the next starts", |ui| {
            egui::Grid::new("road").num_columns(4).striped(true).spacing([18.0, 8.0]).show(ui, |ui| {
                for h in ["Phase", "Work", "Exit gate", "Status"] { th(ui, h); }
                ui.end_row();
                let rows = [
                    ("0. Spec freeze", "Requirements doc, CAN FD protocol spec, config schema, API schema, node classes, safety case", "Review sign-off; every feature has an owner and an interface"),
                    ("1. Simulator and core", "Shared crates, planner, G-code, virtual CAN bus, desktop app skeleton", "Planner output matches a reference Klipper run within tolerance"),
                    ("2. Bus bring-up", "CAN FD dev boards, bootloader, discovery, time sync, update and rollback", "Two segments at target bitrate, error-free multi-hour soak while streaming and flashing"),
                    ("3. Driver", "FOC driver v1 (39x39 mm, 4 A RMS), hardwired STEP/DIR, CAN config and faults", "1 m/s carriage on 20T 2GT pulley, 14-bit-class encoder, 60 C chamber"),
                    ("4. Mainboard and probe", "P4 + S3 + STM32 board, eddy probe, ADXL", "Auto-calibration (Z home, mesh, X twist) completes unattended"),
                    ("5. Full machine", "Tomahawk first platform: toolhead, power, chamber, camera nodes", "Full-length print with clean fault-injection run"),
                    ("6. Fleet", "Management Interface farm features, signed OTA, desktop release", "10+ machines managed from one app with staged roll-out"),
                    ("7. Step-servo CAN mode (v2)", "Drop STEP/DIR from the harness", "Jitter and latency below the hardwired baseline"),
                ];
                for (i, (p, w, g)) in rows.iter().enumerate() {
                    ui.label(mono(*p).color(WHITE)); wrap(ui, 300.0, dim(*w)); wrap(ui, 320.0, *g);
                    let (st, tx) = if i < m.phases_done { (Status::Ok, "gate passed") } else if i == m.phases_done { (Status::Warn, "in progress") } else { (Status::Off, "not started") };
                    status(ui, st, tx);
                    ui.end_row();
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if button(ui, "Mark current gate passed") && m.phases_done < 8 { m.phases_done += 1; }
                if button(ui, "Reopen last gate") && m.phases_done > 0 { m.phases_done -= 1; }
            });
            ui.label(dim("Build a Klipper compatibility shim only if migrating the existing fleet requires it."));
        }),
        1 => panel(ui, "Native feature set - each is a first-class function with config schema, API endpoints and UI panel", |ui| {
            egui::Grid::new("feat").num_columns(4).striped(true).spacing([18.0, 7.0]).show(ui, |ui| {
                for h in ["Feature", "Community origin", "MACH5 native design", "Panel"] { th(ui, h); }
                ui.end_row();
                for (f, o, d, p) in FEATURES.iter() {
                    wrap(ui, 240.0, mono(*f)); wrap(ui, 150.0, dim(*o)); wrap(ui, 420.0, *d);
                    if ui.small_button(p.title()).clicked() { m.goto = Some(*p); }
                    ui.end_row();
                }
            });
            ui.add_space(4.0);
            ui.label(dim("Timelapse, remote screens and mobile control are views of the same API, not plugins."));
        }),
        2 => panel(ui, "Design principles", |ui| {
            egui::Grid::new("prin").num_columns(3).striped(true).spacing([18.0, 7.0]).show(ui, |ui| {
                for h in ["Area", "Carried over", "Proposed change"] { th(ui, h); }
                ui.end_row();
                for (a, c, p) in [
                    ("Compute", "Host ESP32-P4, Motion Bridge STM32, Network Bridge ESP32-S3; no SBC", "None. Keep plain, definitive names"),
                    ("Language", "Rust wherever possible", "Shared no_std crates for protocol, units and config schema across all chips and the desktop app"),
                    ("Motion", "Third-order jerk-limited planner, no junction deviation, closed-loop/FOC default", "Input shaping and pressure advance inside the planner"),
                    ("Probe", "Own eddy probe with own STM32, read by the P4", "Beacon/Cartographer as compatibility driver only"),
                    ("Wired modules", "CAN FD, no dedicated probe trigger line", "Two physical CAN FD segments: drive and peripherals"),
                    ("Drivers", "STEP/DIR/EN hardwired, CAN for config and faults", "v2: CAN-synchronised step-servo mode"),
                    ("Extensibility", "No plugin system; features native", "Sandboxed rules engine for user logic"),
                    ("Management", "Native desktop app; machine serves API only", "Signed API schema from the same Rust types as firmware"),
                    ("Updates", "Network flash via SD, per-chip, by config", "Dual-bank images with rollback on every MCU"),
                ] {
                    ui.label(mono(a)); wrap(ui, 380.0, dim(c)); wrap(ui, 380.0, p);
                    ui.end_row();
                }
            });
        }),
        3 => {
            ui.columns(2, |c| {
                panel(&mut c[0], "System architecture", |ui| {
                    ui.label(mono("Management Interface (desktop; phone/tablet ports)\n   | HTTPS/WebSocket API (signed schema)\n+--+----------------- MAINBOARD -----------------+\n| Network Bridge ESP32-S3: WiFi/Ethernet, API,   |\n|   OTA, SD files                                |\n|   | SPI                                        |\n| Host ESP32-P4: G-code, planner, shaper,        |\n|   thermal, leveling, state machine, DSI screen |\n|   | SPI/shared                                 |\n| Motion Bridge STM32: step generation, hard     |\n|   safety, heater watchdog, ADC                 |\n+--------+---------------------+-----------------+\n         | CAN FD A (drive)    | CAN FD B (periph.)\n   FOC drivers X/Y/Z/E     Eddy probe, toolhead,\n   (STEP/DIR/EN + CAN)     chamber, AMS/MMU, filament\n                           sensors, ADXL, camera, light"));
                    ui.add_space(4.0);
                    ui.label(dim("The P4 owns all decisions and the STM32 owns all deadlines."));
                });
                panel(&mut c[1], "Software stack (one Rust workspace)", |ui| {
                    for (i, t) in [
                        "Shared crates (no_std): CAN FD protocol, node identity, typed config schema, units, motion math",
                        "Motion Bridge firmware: step generation, hard safety, ADC, heater watchdog",
                        "Host firmware (P4): G-code, third-order planner, shaper, PA, thermal, auto-cal, state machine, UI",
                        "Network Bridge firmware (S3): WiFi/Ethernet, API server, SD upload, OTA staging, discovery",
                        "Node firmware: one crate per node class, common bootloader, dual-bank signed images",
                        "API: versioned, schema-generated, per-client keys, live telemetry streams",
                        "Management Interface: native desktop app (Windows and Linux first, macOS later)",
                        "Host tooling: simulator with virtual CAN bus",
                    ].iter().enumerate() {
                        ui.label(format!("{}.  {}", i + 1, t));
                    }
                });
            });
        }
        _ => panel(ui, "Open decisions and risks", |ui| {
            let items = [
                "CAN FD on the P4: confirm controller and Rust driver support before committing the mainboard design",
                "Two segments or one: approve the drive/peripheral split, or one bus with strict priority and bandwidth budget",
                "P4 plus STM32 split: which timing-critical work stays on the STM32; shared memory versus SPI queue",
                "Sandboxed rules engine scope: triggers and macros only, or general scripting",
                "Eddy probe versus compatibility: do Beacon and Cartographer drivers ship at launch or later",
                "Motion planner validation: print-quality benchmark for third-order planning without junction deviation",
                "Security: key provisioning and rotation for API clients and signed firmware",
                "Supply: second sources for the P4, S3 and driver parts",
            ];
            for (i, t) in items.iter().enumerate() { ui.checkbox(&mut m.decisions[i], *t); }
            ui.add_space(4.0);
            ui.label(dim(format!("{} of {} resolved. Largest schedule risk: P4 firmware effort. The phase 1 simulator is the main mitigation.", m.decisions.iter().filter(|d| **d).count(), items.len())));
        }),
    }
}
