use crate::data::*;
use crate::fleetsim::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align2, Color32, DragValue, FontId, RichText, Sense, Stroke, StrokeKind, Ui};

fn short(s: &str) -> String {
    let s = s.trim_end_matches(".gcode");
    if s.len() > 22 { format!("{}..", &s[..20]) } else { s.to_string() }
}

fn stat(ui: &mut Ui, big: String, cap: &str, st: Option<Status>) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let col = match st { Some(s) if s != Status::Ok => s.color(), _ => WHITE };
        ui.label(mono(big).size(20.0).color(col));
        ui.label(dim(cap));
    });
    ui.add_space(22.0);
}

fn in_group(m: &Model, i: usize) -> bool {
    m.st.grp_filter == 0 || m.fleet[i].group == m.st.grp_filter - 1
}

pub fn fleet(ui: &mut Ui, m: &mut Model) {
    title(ui, "Fleet", "Farm of machines: all running at once, live. Pick any machine in the top bar to drive it in the other pages");
    summary(ui, m);
    tabs(ui, &mut m.st.farm_tab, &["Overview", "Queue", "Cameras", "Alerts", "Spools", "Analytics", "Roll-out & discovery"]);
    match m.st.farm_tab {
        0 => overview(ui, m),
        1 => queue(ui, m),
        2 => cameras(ui, m),
        3 => alerts(ui, m),
        4 => spools(ui, m),
        5 => analytics(ui, m),
        _ => rollout(ui, m),
    }
}

fn summary(ui: &mut Ui, m: &mut Model) {
    let c = |s: MState| m.fleet.iter().filter(|f| f.state == s).count();
    let open = m.alerts.iter().filter(|a| !a.ack && a.sev != Sev::Info).count();
    let open_err = m.alerts.iter().any(|a| !a.ack && a.sev == Sev::Err);
    let rate = *m.h_farm_g.back().unwrap_or(&0.0);
    let util = m.fleet.iter().map(|f| f.util[6]).sum::<f32>() / m.fleet.len().max(1) as f32;
    let queued = m.queue.iter().map(|q| q.left).sum::<u32>();
    panel(ui, "", |ui| {
        ui.horizontal(|ui| {
            stat(ui, format!("{}/{}", c(MState::Printing), m.fleet.len()), "printing", None);
            stat(ui, c(MState::Idle).to_string(), "idle", None);
            stat(ui, c(MState::Paused).to_string(), "paused", if c(MState::Paused) > 0 { Some(Status::Warn) } else { None });
            stat(ui, c(MState::Fault).to_string(), "faulted", if c(MState::Fault) > 0 { Some(Status::Err) } else { None });
            stat(ui, c(MState::Calibrating).to_string(), "calibrating", None);
            stat(ui, open.to_string(), "open alerts", if open_err { Some(Status::Err) } else if open > 0 { Some(Status::Warn) } else { None });
            stat(ui, format!("{:.0} g/h", rate), "filament rate", None);
            stat(ui, format!("{:.0} %", util * 100.0), "utilisation today", None);
            stat(ui, queued.to_string(), "jobs queued", None);
        });
    });
}

fn overview(ui: &mut Ui, m: &mut Model) {
    panel(ui, "Bulk actions", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(dim("Group"));
            egui::ComboBox::from_id_salt("grp_filter").width(110.0).selected_text(if m.st.grp_filter == 0 { "All" } else { GROUPS[m.st.grp_filter - 1] }).show_ui(ui, |ui| {
                ui.selectable_value(&mut m.st.grp_filter, 0, "All");
                for (k, g) in GROUPS.iter().enumerate() { ui.selectable_value(&mut m.st.grp_filter, k + 1, *g); }
            });
            ui.label(dim("Select"));
            if ui.small_button("all").clicked() { for i in 0..m.fleet.len() { m.fleet[i].sel = in_group(m, i); } }
            if ui.small_button("none").clicked() { m.fleet.iter_mut().for_each(|f| f.sel = false); }
            if ui.small_button("printing").clicked() { for i in 0..m.fleet.len() { m.fleet[i].sel = in_group(m, i) && m.fleet[i].state == MState::Printing; } }
            if ui.small_button("idle").clicked() { for i in 0..m.fleet.len() { m.fleet[i].sel = in_group(m, i) && m.fleet[i].state == MState::Idle; } }
        });
        let sel: Vec<usize> = (0..m.fleet.len()).filter(|i| m.fleet[*i].sel).collect();
        ui.horizontal_wrapped(|ui| {
            ui.label(dim(format!("{} selected", sel.len())));
            ui.add_enabled_ui(!sel.is_empty(), |ui| {
                if button(ui, "Pause") { for i in &sel { m.m_pause(*i); } m.log(Sev::Info, "farm", &format!("Pause sent to {} machine(s)", sel.len())); }
                if button(ui, "Resume") { for i in &sel { m.m_resume(*i); } m.log(Sev::Info, "farm", &format!("Resume sent to {} machine(s)", sel.len())); }
                if button(ui, "Cancel job") { for i in &sel { m.m_cancel(*i); } m.log(Sev::Warn, "farm", &format!("Cancel sent to {} machine(s)", sel.len())); }
                if button(ui, "Cool down") { for i in &sel { m.m_cool(*i); } }
                if button(ui, "Clear faults") { for i in &sel { m.m_clear(*i); } }
                ui.separator();
                egui::ComboBox::from_id_salt("send_file").width(190.0).selected_text(JOBS[m.st.send_file].0).show_ui(ui, |ui| {
                    for (k, j) in JOBS.iter().enumerate() { ui.selectable_value(&mut m.st.send_file, k, j.0); }
                });
                if primary(ui, "Send job to idle") {
                    let mut n = 0;
                    for i in &sel { if m.fleet[*i].state == MState::Idle { m.m_start(*i, m.st.send_file); n += 1; } }
                    m.log(Sev::Info, "farm", &format!("Job {} started on {} idle machine(s)", JOBS[m.st.send_file].0, n));
                }
                ui.separator();
                egui::ComboBox::from_id_salt("set_group").width(100.0).selected_text(GROUPS[m.st.set_group]).show_ui(ui, |ui| {
                    for (k, g) in GROUPS.iter().enumerate() { ui.selectable_value(&mut m.st.set_group, k, *g); }
                });
                if button(ui, "Set group") { for i in &sel { m.fleet[*i].group = m.st.set_group; } }
            });
        });
    });

    panel(ui, "Machines (live)", |ui| {
        let mut acts: Vec<(usize, u8)> = Vec::new();
        let act = m.active;
        egui::Grid::new("fleet_tbl").num_columns(13).striped(true).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("");
            for h in ["Name", "Group", "State", "Job", "Progress", "ETA", "Hotend / bed", "Mat.", "FW", "Nodes", "Health", ""] { th(ui, h); }
            ui.end_row();
            for i in 0..m.fleet.len() {
                if !in_group(m, i) { continue; }
                let f = &mut m.fleet[i];
                ui.checkbox(&mut f.sel, "");
                if ui.selectable_label(m.st.sel_fleet == i, if i == act { format!("{} *", f.name) } else { f.name.clone() }).clicked() { acts.push((i, 0)); }
                ui.label(dim(GROUPS[f.group]));
                status(ui, f.state.status(), f.state.label());
                wrap(ui, 130.0, mono(short(&f.file)));
                ui.horizontal(|ui| { bar(ui, f.prog, 56.0, None); ui.label(mono(format!("{:3.0}%", f.prog * 100.0))); });
                ui.label(mono(if f.state == MState::Printing { fmt_dur(eta(f)) } else { "-".into() }));
                ui.label(mono(format!("{:.0}/{:.0}  {:.0}/{:.0}", f.temps[0], f.targets[0], f.temps[1], f.targets[1])));
                ui.label(mono(if f.file.is_empty() { "-" } else { f.material }));
                ui.label(mono(f.fw.clone()).color(if f.fw == TARGET_FW { WHITE } else { TEXT }));
                ui.label(mono(format!("{}/{}", f.nodes.0, f.nodes.1)).color(if f.nodes.0 < f.nodes.1 { WARN } else { TEXT }));
                status(ui, f.health, f.health.label());
                ui.horizontal(|ui| {
                    if ui.small_button("open").clicked() { acts.push((i, 1)); }
                    match f.state {
                        MState::Printing => { if ui.small_button("pause").clicked() { acts.push((i, 2)); } }
                        MState::Paused => { if ui.small_button("resume").clicked() { acts.push((i, 3)); } }
                        MState::Fault => { if ui.small_button("clear").clicked() { acts.push((i, 4)); } }
                        _ => {}
                    }
                });
                ui.end_row();
            }
        });
        for (i, a) in acts {
            match a {
                0 => m.st.sel_fleet = i,
                1 => { m.switch_machine(i); m.goto = Some(Page::Dashboard); }
                2 => m.m_pause(i),
                3 => m.m_resume(i),
                _ => m.m_clear(i),
            }
        }
        ui.label(dim("* = machine currently driven by the other pages. Simulation clock runs 30x real time."));
    });

    let k = m.st.sel_fleet.min(m.fleet.len() - 1);
    ui.columns(2, |c| {
        panel(&mut c[0], &format!("{} - identity and keys", m.fleet[k].name), |ui| {
            let f = &m.fleet[k];
            kv(ui, "fm_id", &[
                ("Address", f.ip.clone()),
                ("Model", f.model.to_string()),
                ("Firmware", f.fw.clone()),
                ("Client key", format!("{}  ({} d old)", f.fp, f.key_age)),
            ]);
            ui.horizontal(|ui| {
                ui.label(dim("Group"));
                egui::ComboBox::from_id_salt("fm_group").width(100.0).selected_text(GROUPS[m.fleet[k].group]).show_ui(ui, |ui| {
                    for (g, n) in GROUPS.iter().enumerate() { ui.selectable_value(&mut m.fleet[k].group, g, *n); }
                });
            });
            ui.horizontal(|ui| {
                if button(ui, "Rotate key") { m.fleet[k].key_age = 0; let n = m.fleet[k].name.clone(); m.log(Sev::Info, "farm", &format!("Key rotated for {}", n)); }
                if button(ui, "Open dashboard") { m.switch_machine(k); m.goto = Some(Page::Dashboard); }
                ui.add_enabled_ui(k != m.active && m.fleet[k].state != MState::Printing, |ui| {
                    if button(ui, "Unpair") { m.unpair(k); }
                });
            });
        });
        panel(&mut c[1], &format!("{} - history", m.fleet[k].name), |ui| {
            let f = &m.fleet[k];
            kv(ui, "fm_hist", &[
                ("Jobs completed", f.jobs_done.to_string()),
                ("Print hours", format!("{:.0} h", f.hours)),
                ("Filament used", format!("{:.1} kg", f.grams / 1000.0)),
                ("Failed jobs", format!("{} ({:.1} %)", f.fails, f.fails as f32 / f.jobs_done.max(1) as f32 * 100.0)),
            ]);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(dim("7 days"));
                for u in f.util.iter() { bar(ui, *u, 36.0, None); }
            });
        });
    });
}

fn queue(ui: &mut Ui, m: &mut Model) {
    ui.columns(2, |c| {
        panel(&mut c[0], "Add job to farm queue", |ui| {
            egui::Grid::new("q_add").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                ui.label(dim("File"));
                egui::ComboBox::from_id_salt("q_file").width(220.0).selected_text(JOBS[m.st.q_file].0).show_ui(ui, |ui| {
                    for (k, j) in JOBS.iter().enumerate() { ui.selectable_value(&mut m.st.q_file, k, format!("{}  ({}, {}h{:02})", j.0, j.1, j.2 / 60, j.2 % 60)); }
                });
                ui.end_row();
                ui.label(dim("Copies"));
                ui.add(DragValue::new(&mut m.st.q_copies).range(1..=50));
                ui.end_row();
                ui.label(dim("Priority"));
                egui::ComboBox::from_id_salt("q_prio").width(110.0).selected_text(["Low", "Normal", "High"][m.st.q_prio]).show_ui(ui, |ui| {
                    for (k, n) in ["Low", "Normal", "High"].iter().enumerate() { ui.selectable_value(&mut m.st.q_prio, k, *n); }
                });
                ui.end_row();
                ui.label(dim("Machine"));
                let tn = if m.st.q_target == 0 { "Any idle".to_string() } else { m.fleet.get(m.st.q_target - 1).map(|f| f.name.clone()).unwrap_or_default() };
                egui::ComboBox::from_id_salt("q_target").width(150.0).selected_text(tn).show_ui(ui, |ui| {
                    ui.selectable_value(&mut m.st.q_target, 0, "Any idle");
                    for (k, f) in m.fleet.iter().enumerate() { ui.selectable_value(&mut m.st.q_target, k + 1, f.name.clone()); }
                });
                ui.end_row();
            });
            ui.add_space(4.0);
            if primary(ui, "Add to queue") {
                let (file, material, mins) = JOBS[m.st.q_file];
                let target = if m.st.q_target == 0 { None } else { Some(m.st.q_target - 1) };
                let id = m.next_qid;
                m.next_qid += 1;
                m.queue.push(QJob { id, file, material, mins, left: m.st.q_copies, prio: m.st.q_prio, target, running: None });
                m.log(Sev::Info, "farm", &format!("Queued #{} {} x{}", id, file, m.st.q_copies));
            }
        });
        panel(&mut c[1], "Dispatcher", |ui| {
            ui.checkbox(&mut m.auto_dispatch, "Auto-dispatch queued jobs to idle machines (off by default; not dev benches)");
            ui.label(dim("Highest priority first, then oldest. A machine with a fault is skipped."));
            ui.add_space(4.0);
            ui.label(RichText::new("RECENTLY COMPLETED").size(11.0).color(DIM).monospace());
            for l in m.done_log.iter().rev().take(5) { ui.label(mono(l.clone()).color(DIM)); }
        });
    });
    panel(ui, "Queue", |ui| {
        if m.queue.is_empty() { ui.label(dim("Queue empty.")); }
        let mut act: Option<(usize, u8)> = None;
        egui::Grid::new("q_tbl").num_columns(9).striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
            for h in ["#", "File", "Material", "Est.", "Left", "Priority", "Machine", "Status", ""] { th(ui, h); }
            ui.end_row();
            for (k, q) in m.queue.iter().enumerate() {
                ui.label(mono(q.id.to_string()));
                ui.label(mono(q.file));
                ui.label(q.material);
                ui.label(mono(format!("{}h{:02}", q.mins / 60, q.mins % 60)));
                ui.label(mono(q.left.to_string()));
                ui.label(["low", "normal", "high"][q.prio]);
                ui.label(dim(q.target.and_then(|t| m.fleet.get(t)).map(|f| f.name.clone()).unwrap_or("any idle".into())));
                match q.running.and_then(|r| m.fleet.get(r)) {
                    Some(f) => ui.horizontal(|ui| { status(ui, Status::Ok, &format!("running on {}", f.name)); bar(ui, f.prog, 50.0, None); }).response,
                    None => ui.label(dim("waiting")),
                };
                ui.horizontal(|ui| {
                    if ui.small_button("up").clicked() { act = Some((k, 0)); }
                    if ui.small_button("down").clicked() { act = Some((k, 1)); }
                    if ui.small_button("prio+").clicked() { act = Some((k, 2)); }
                    if ui.small_button("remove").clicked() { act = Some((k, 3)); }
                });
                ui.end_row();
            }
        });
        if let Some((k, a)) = act {
            match a {
                0 if k > 0 => m.queue.swap(k, k - 1),
                1 if k + 1 < m.queue.len() => m.queue.swap(k, k + 1),
                2 => m.queue[k].prio = (m.queue[k].prio + 1) % 3,
                3 => { let q = m.queue.remove(k); if let Some(r) = q.running { m.m_cancel(r); } }
                _ => {}
            }
        }
    });
}

/// Animated placeholder camera view driven by the machine's live state.
fn cam_tile(ui: &mut Ui, f: &FleetMachine, t: f32, w: f32, seed: f32, current: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(w, w * 0.62), Sense::click());
    let p = ui.painter_at(r);
    p.rect_filled(r, RAD, Color32::from_gray(7));
    for k in 0..(r.height() / 5.0) as i32 {
        p.line_segment([egui::pos2(r.left(), r.top() + k as f32 * 5.0), egui::pos2(r.right(), r.top() + k as f32 * 5.0)], Stroke::new(1.0_f32, Color32::from_white_alpha(4)));
    }
    let floor = r.bottom() - r.height() * 0.1;
    p.rect_filled(egui::Rect::from_min_max(egui::pos2(r.left() + r.width() * 0.1, floor), egui::pos2(r.right() - r.width() * 0.1, floor + 5.0)), 1.0, Color32::from_gray(60));
    let live = matches!(f.state, MState::Printing | MState::Paused | MState::Fault);
    let stack = if live { f.prog.max(0.03) * r.height() * 0.55 } else { 0.0 };
    if live {
        for (k, cx) in [0.28_f32, 0.5, 0.72].iter().enumerate() {
            let bw = r.width() * (0.12 + 0.03 * k as f32);
            let blk = egui::Rect::from_min_max(egui::pos2(r.left() + r.width() * cx - bw / 2.0, floor - stack), egui::pos2(r.left() + r.width() * cx + bw / 2.0, floor));
            p.rect_filled(blk, 0.0, Color32::from_gray(78));
            let mut y = blk.bottom();
            while y > blk.top() { p.line_segment([egui::pos2(blk.left(), y), egui::pos2(blk.right(), y)], Stroke::new(1.0_f32, Color32::from_gray(50))); y -= 3.0; }
        }
        let nx = r.left() + r.width() * (0.2 + 0.6 * (0.5 + 0.5 * (t * 2.4 + seed).sin()));
        let ny = floor - stack - 6.0;
        p.line_segment([egui::pos2(r.left() + 6.0, ny - 4.0), egui::pos2(r.right() - 6.0, ny - 4.0)], Stroke::new(1.0_f32, Color32::from_gray(70)));
        p.rect_filled(egui::Rect::from_center_size(egui::pos2(nx, ny), vec2(10.0, 8.0)), 1.0, Color32::from_gray(if f.state == MState::Printing { 230 } else { 120 }));
    }
    let (msg, col) = match f.state {
        MState::Idle => ("READY", FAINT),
        MState::Paused => ("PAUSED", WARN),
        MState::Fault => ("FAULT", ERR),
        MState::Calibrating => ("CALIBRATING", DIM),
        MState::Printing => ("", DIM),
    };
    if !msg.is_empty() { p.text(r.center() - vec2(0.0, r.height() * 0.15), Align2::CENTER_CENTER, msg, FontId::monospace(13.0), col); }
    p.text(r.left_top() + vec2(7.0, 5.0), Align2::LEFT_TOP, &f.name, FontId::monospace(10.5), WHITE);
    p.text(r.left_bottom() + vec2(7.0, -5.0), Align2::LEFT_BOTTOM, format!("{:.0}%  {:.0}C  {}", f.prog * 100.0, f.temps[0], if f.state == MState::Printing { fmt_dur(eta(f)) } else { String::new() }), FontId::monospace(10.0), DIM);
    p.rect_filled(egui::Rect::from_center_size(r.right_top() + vec2(-10.0, 10.0), vec2(7.0, 7.0)), 0.0, f.state.status().color());
    p.rect_stroke(r, RAD, Stroke::new(1.0_f32, if current { WHITE } else if resp.hovered() { LINE2 } else { LINE }), StrokeKind::Inside);
    resp
}

fn cameras(ui: &mut Ui, m: &mut Model) {
    ui.horizontal(|ui| {
        ui.label(dim("Group"));
        egui::ComboBox::from_id_salt("cam_grp").width(110.0).selected_text(if m.st.grp_filter == 0 { "All" } else { GROUPS[m.st.grp_filter - 1] }).show_ui(ui, |ui| {
            ui.selectable_value(&mut m.st.grp_filter, 0, "All");
            for (k, g) in GROUPS.iter().enumerate() { ui.selectable_value(&mut m.st.grp_filter, k + 1, *g); }
        });
        ui.checkbox(&mut m.st.only_printing, "Only printing");
        ui.checkbox(&mut m.auto_pause, "Auto-pause on detected failure");
        ui.label(dim("Click a tile to open that machine."));
    });
    ui.add_space(6.0);
    let t = m.t as f32;
    let cols = 4;
    let w = (ui.available_width() - 8.0 * (cols as f32 - 1.0) - 4.0) / cols as f32;
    let idx: Vec<usize> = (0..m.fleet.len()).filter(|i| in_group(m, *i) && (!m.st.only_printing || m.fleet[*i].state == MState::Printing)).collect();
    let mut open = None;
    egui::Grid::new("cam_wall").spacing([8.0, 8.0]).show(ui, |ui| {
        for (n, i) in idx.iter().enumerate() {
            if cam_tile(ui, &m.fleet[*i], t, w, *i as f32 * 1.7, *i == m.active).clicked() { open = Some(*i); }
            if (n + 1) % cols == 0 { ui.end_row(); }
        }
    });
    if let Some(i) = open { m.switch_machine(i); m.goto = Some(Page::Camera); }
}

fn alerts(ui: &mut Ui, m: &mut Model) {
    ui.horizontal(|ui| {
        for (k, n) in ["All", "Open", "Errors", "Warnings"].iter().enumerate() {
            if ui.selectable_label(m.st.al_filter == k, *n).clicked() { m.st.al_filter = k; }
        }
        if button(ui, "Acknowledge all") { m.alerts.iter_mut().for_each(|a| a.ack = true); }
        if button(ui, "Clear acknowledged") { m.alerts.retain(|a| !a.ack); }
    });
    ui.add_space(6.0);
    panel(ui, "Farm alerts and failure-detection feed", |ui| {
        let mut act: Option<(usize, u8)> = None;
        egui::Grid::new("alerts").num_columns(6).striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
            for h in ["Time", "Machine", "Level", "Message", "", ""] { th(ui, h); }
            ui.end_row();
            for (k, a) in m.alerts.iter().enumerate().rev() {
                let show = match m.st.al_filter { 1 => !a.ack, 2 => a.sev == Sev::Err, 3 => a.sev == Sev::Warn, _ => true };
                if !show { continue; }
                ui.label(mono(a.ts.clone()).color(FAINT));
                ui.label(m.fleet.get(a.machine).map(|f| f.name.clone()).unwrap_or_default());
                match a.sev { Sev::Info => status(ui, Status::Idle, "info"), Sev::Warn => status(ui, Status::Warn, "warn"), Sev::Err => status(ui, Status::Err, "error") }
                wrap(ui, 520.0, if a.ack { dim(a.msg.clone()) } else { RichText::new(a.msg.clone()) });
                if a.ack { ui.label(dim("acked")); } else if ui.small_button("ack").clicked() { act = Some((k, 0)); }
                if ui.small_button("open").clicked() { act = Some((k, 1)); }
                ui.end_row();
            }
        });
        match act {
            Some((k, 0)) => m.alerts[k].ack = true,
            Some((k, _)) => { let i = m.alerts[k].machine; m.switch_machine(i); m.goto = Some(Page::Dashboard); }
            None => {}
        }
    });
}

fn spools(ui: &mut Ui, m: &mut Model) {
    panel(ui, "Stock by material", |ui| {
        ui.horizontal(|ui| {
            ui.label(dim("Low-stock threshold per spool"));
            ui.add(DragValue::new(&mut m.low_thr).range(20.0..=500.0).suffix(" g"));
        });
        egui::Grid::new("stock").num_columns(6).striped(true).spacing([20.0, 6.0]).show(ui, |ui| {
            for h in ["Material", "Spools", "Loaded", "In stock", "Total", "Status"] { th(ui, h); }
            ui.end_row();
            for mat in ["PLA", "PETG", "ASA", "PA-CF"] {
                let all: Vec<&FSpool> = m.fspools.iter().filter(|s| s.mat == mat).collect();
                let loaded: f32 = all.iter().filter(|s| s.machine.is_some()).map(|s| s.left).sum();
                let stock: f32 = all.iter().filter(|s| s.machine.is_none()).map(|s| s.left).sum();
                let lowc = all.iter().filter(|s| s.left < m.low_thr).count();
                ui.label(mono(mat));
                ui.label(mono(all.len().to_string()));
                ui.label(mono(format!("{:.1} kg", loaded / 1000.0)));
                ui.label(mono(format!("{:.1} kg", stock / 1000.0)));
                ui.label(mono(format!("{:.1} kg", (loaded + stock) / 1000.0)));
                if stock < 1500.0 { status(ui, Status::Warn, "reorder soon"); } else if lowc > 0 { status(ui, Status::Warn, &format!("{} spool(s) low", lowc)); } else { status(ui, Status::Ok, "ok"); }
                ui.end_row();
            }
        });
    });
    panel(ui, "Spool database (synced to every machine)", |ui| {
        let mut act: Option<(usize, u8, usize)> = None;
        egui::ScrollArea::vertical().max_height(380.0).auto_shrink([false, true]).show(ui, |ui| {
            egui::Grid::new("fspools").num_columns(7).striped(true).spacing([16.0, 5.0]).show(ui, |ui| {
                for h in ["ID", "Brand", "Material", "Colour", "Remaining", "Location", ""] { th(ui, h); }
                ui.end_row();
                for (k, s) in m.fspools.iter().enumerate() {
                    ui.label(mono(format!("#{}", s.id)));
                    ui.label(s.brand);
                    ui.label(s.mat);
                    ui.label(s.color);
                    ui.horizontal(|ui| { bar(ui, s.left / s.total, 70.0, if s.left < m.low_thr { Some(Status::Warn) } else { None }); ui.label(mono(format!("{:.0} g", s.left))); });
                    ui.label(match s.machine { Some(i) => m.fleet.get(i).map(|f| f.name.clone()).unwrap_or_default(), None => "stock".into() });
                    ui.horizontal(|ui| {
                        if s.left < m.low_thr && ui.small_button("replace").clicked() { act = Some((k, 0, 0)); }
                        if s.machine.is_some() {
                            if ui.small_button("to stock").clicked() { act = Some((k, 1, 0)); }
                        } else {
                            ui.menu_button("assign", |ui| {
                                for (i, f) in m.fleet.iter().enumerate().filter(|(_, f)| f.group != 2) {
                                    if ui.button(&f.name).clicked() { act = Some((k, 2, i)); ui.close_menu(); }
                                }
                            });
                        }
                    });
                    ui.end_row();
                }
            });
        });
        match act {
            Some((k, 0, _)) => m.fspools[k].left = m.fspools[k].total,
            Some((k, 1, _)) => m.fspools[k].machine = None,
            Some((k, _, i)) => m.fspools[k].machine = Some(i),
            None => {}
        }
        if button(ui, "Add spool to stock") {
            let id = m.fspools.iter().map(|s| s.id).max().unwrap_or(0) + 1;
            m.fspools.push(FSpool { id, brand: "New", mat: "PLA", color: "Black", left: 1000.0, total: 1000.0, machine: None });
        }
    });
}

fn analytics(ui: &mut Ui, m: &mut Model) {
    ui.columns(2, |c| {
        panel(&mut c[0], "Printers running (last 4 min)", |ui| {
            plot(ui, 150.0, &[Series::from_ring("printing", G1, &m.h_farm_n)], (0.0, HIST as f32), (0.0, m.fleet.len() as f32 + 1.0), "machines");
        });
        panel(&mut c[1], "Filament rate", |ui| {
            plot(ui, 150.0, &[Series::from_ring("g/h", G2, &m.h_farm_g)], (0.0, HIST as f32), (0.0, 2400.0), "g per hour");
        });
    });
    panel(ui, "Utilisation, last 7 days (rightmost = today)", |ui| {
        let w = ui.available_width();
        let rows = m.fleet.len();
        let (r, _) = ui.allocate_exact_size(vec2(w, rows as f32 * 22.0 + 4.0), Sense::hover());
        let p = ui.painter_at(r);
        let cw = ((w - 130.0) / 7.0).min(90.0);
        for (i, f) in m.fleet.iter().enumerate() {
            let y = r.top() + i as f32 * 22.0;
            p.text(egui::pos2(r.left() + 4.0, y + 11.0), Align2::LEFT_CENTER, &f.name, FontId::monospace(11.0), TEXT);
            for d in 0..7 {
                let u = f.util[d];
                let cell = egui::Rect::from_min_size(egui::pos2(r.left() + 130.0 + d as f32 * cw, y + 1.0), vec2(cw - 3.0, 19.0));
                let g = (24.0 + u * 190.0) as u8;
                p.rect_filled(cell, 2.0, Color32::from_gray(g));
                p.text(cell.center(), Align2::CENTER_CENTER, format!("{:.0}%", u * 100.0), FontId::monospace(10.0), if g > 120 { Color32::BLACK } else { Color32::from_gray(210) });
            }
        }
    });
    panel(ui, "Per-machine totals", |ui| {
        egui::Grid::new("totals").num_columns(6).striped(true).spacing([20.0, 5.0]).show(ui, |ui| {
            for h in ["Machine", "Jobs", "Print hours", "Filament", "Failures", "Failure rate"] { th(ui, h); }
            ui.end_row();
            for f in m.fleet.iter() {
                ui.label(&f.name);
                ui.label(mono(f.jobs_done.to_string()));
                ui.label(mono(format!("{:.0} h", f.hours)));
                ui.label(mono(format!("{:.1} kg", f.grams / 1000.0)));
                ui.label(mono(f.fails.to_string()));
                ui.label(mono(format!("{:.1} %", f.fails as f32 / f.jobs_done.max(1) as f32 * 100.0)));
                ui.end_row();
            }
        });
    });
}

fn rollout(ui: &mut Ui, m: &mut Model) {
    ui.columns(2, |c| {
        panel(&mut c[0], &format!("Staged firmware roll-out to {}", TARGET_FW), |ui| {
            for w in 1..=3u8 {
                let names: Vec<String> = m.fleet.iter().filter(|f| f.wave == w).map(|f| f.name.clone()).collect();
                ui.horizontal(|ui| {
                    let wi = (w - 1) as usize;
                    let st = if m.rollout.done || m.rollout.wave > wi { Status::Ok } else if m.rollout.wave == wi && m.rollout.running { Status::Warn } else if m.rollout.wave == wi && m.rollout.halted.is_some() { Status::Err } else { Status::Idle };
                    dot(ui, st);
                    ui.label(["Wave 1 - canary", "Wave 2", "Wave 3 - remainder"][wi]);
                    if m.rollout.wave == wi && m.rollout.running { bar(ui, m.rollout.prog, 80.0, None); }
                });
                ui.label(dim(names.join(", ")));
                ui.add_space(4.0);
            }
            if let Some(n) = m.rollout.halted.clone() {
                ui.horizontal(|ui| {
                    status(ui, Status::Err, &format!("Halted: {} is faulted", n));
                    if let Some(i) = m.fleet.iter().position(|f| f.name == n) {
                        if button(ui, "Clear fault") { m.m_clear(i); }
                    }
                });
            }
            ui.horizontal(|ui| {
                if !m.rollout.running {
                    if m.rollout.halted.is_some() || (m.rollout.wave > 0 && !m.rollout.done) {
                        if primary(ui, "Resume roll-out") { m.rollout_gate(); if m.rollout.halted.is_none() { m.rollout.running = true; } }
                    }
                    if primary(ui, if m.rollout.done { "Run again" } else { "Start roll-out" }) {
                        for f in m.fleet.iter_mut().filter(|f| f.wave > 0) { f.fw = "1.4.1".into(); }
                        m.rollout = Rollout { wave: 0, prog: 0.0, running: false, halted: None, done: false };
                        m.rollout_gate();
                        if m.rollout.halted.is_none() { m.rollout.running = true; }
                        m.log(Sev::Info, "fleet", &format!("Staged roll-out of {} started", TARGET_FW));
                    }
                } else if button(ui, "Pause") { m.rollout.running = false; }
            });
            if m.rollout.done { ui.label(dim("Complete: every wave passed its health gate.")); }
        });
        panel(&mut c[1], "Health gates and firmware spread", |ui| {
            for t in ["No machine in the next wave has a latched fault", "No node bootloader rollbacks in the previous wave", "Heartbeats and time sync nominal", "Applied between jobs; signed images only"] {
                ui.horizontal(|ui| { dot(ui, Status::Ok); ui.label(t); });
            }
            ui.add_space(6.0);
            let mut vers: Vec<String> = m.fleet.iter().map(|f| f.fw.clone()).collect();
            vers.sort();
            vers.dedup();
            for v in vers {
                let n = m.fleet.iter().filter(|f| f.fw == v).count();
                ui.horizontal(|ui| { ui.label(mono(v.clone())); bar(ui, n as f32 / m.fleet.len() as f32, 120.0, None); ui.label(dim(format!("{} machine(s)", n))); });
            }
        });
    });
    panel(ui, "Discovery (machines announce themselves on the network)", |ui| {
        let mut pair: Option<usize> = None;
        for (k, d) in m.disc.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(mono(format!("{}  {}", d.name, d.ip)));
                ui.label(dim("unpaired"));
                if ui.small_button("Pair").clicked() { pair = Some(k); }
            });
        }
        if let Some(k) = pair { let d = m.disc.remove(k); m.pair(d); }
        if m.disc.is_empty() { ui.label(dim("No unpaired machines announced.")); }
        if button(ui, "Scan network") {
            m.scan_n += 1;
            let name = format!("Tomahawk-{}", 15 + m.scan_n);
            if !m.fleet.iter().any(|f| f.name == name) && !m.disc.iter().any(|d| d.name == name) {
                m.disc.push(Disc { name, ip: format!("10.0.4.{}", 35 + m.scan_n) });
            }
        }
    });
}
