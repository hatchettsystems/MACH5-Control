use crate::data::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align2, Color32, DragValue, FontId, Sense, Stroke, StrokeKind, Ui};

const MMU_STATES: [&str; 6] = ["IDLE", "SELECT", "FEED", "LOADED", "RETRACT", "FAULT"];

pub fn filament(ui: &mut Ui, m: &mut Model) {
    title(ui, "Filament", "MMU node state machine, spool inventory and filament profiles");
    tabs(ui, &mut m.st.tab, &["MMU", "Spool inventory", "Profiles"]);
    match m.st.tab {
        0 => {
            panel(ui, "MMU node (CAN FD B, 0x33)", |ui| {
                ui.horizontal(|ui| {
                    ui.label(dim("State"));
                    for (k, s) in MMU_STATES.iter().enumerate() {
                        let active = m.mmu_state == k;
                        let (r, _) = ui.allocate_exact_size(vec2(82.0, 24.0), Sense::hover());
                        let fill = if active { WHITE } else { Color32::TRANSPARENT };
                        ui.painter().rect_filled(r, crate::theme::RAD, fill);
                        ui.painter().rect_stroke(r, crate::theme::RAD, Stroke::new(1.0_f32, if active { WHITE } else { LINE2 }), StrokeKind::Inside);
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, *s, FontId::monospace(11.0), if active { BG } else { DIM });
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if primary(ui, "Load selected") {
                        m.mmu_state = 3;
                        m.mmu_verbs.push(format!("slot {} loaded", m.st.step + 1));
                    }
                    if button(ui, "Unload") {
                        m.mmu_state = 0;
                        m.mmu_verbs.push("unloaded".into());
                    }
                    if button(ui, "Home selector") { m.mmu_state = 0; }
                    if button(ui, "Clear fault") { m.mmu_state = 0; }
                });
            });
            panel(ui, "Slots", |ui| {
                egui::Grid::new("slots").num_columns(6).striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
                    for h in ["Slot", "Select", "Sensor", "Spool", "Material", "Remaining"] { th(ui, h); }
                    ui.end_row();
                    for (k, s) in m.slots.iter().enumerate() {
                        ui.label(mono(format!("T{}", k)));
                        if ui.selectable_label(m.st.step == k, if m.st.step == k { "selected" } else { "select" }).clicked() { m.st.step = k; }
                        status(ui, if s.sensor { Status::Ok } else { Status::Off }, if s.sensor { "filament" } else { "empty" });
                        match m.spools.iter().find(|sp| Some(sp.id) == s.spool) {
                            Some(sp) => {
                                ui.label(format!("#{} {} {}", sp.id, sp.brand, sp.color));
                                ui.label(sp.mat);
                                ui.horizontal(|ui| { bar(ui, sp.left / sp.total, 70.0, if sp.left < 100.0 { Some(Status::Warn) } else { None }); ui.label(mono(format!("{:.0} g", sp.left))); });
                            }
                            None => { ui.label(dim("-")); ui.label(dim("-")); ui.label(dim("-")); }
                        }
                        ui.end_row();
                    }
                });
                if !m.mmu_verbs.is_empty() { ui.add_space(4.0); ui.label(dim(format!("last: {}", m.mmu_verbs.last().unwrap()))); }
            });
        }
        1 => {
            ui.columns(2, |c| {
                panel(&mut c[0], "Spool database (lives in the machine, syncs to the app)", |ui| {
                    egui::Grid::new("spools").num_columns(6).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
                        for h in ["ID", "Brand", "Material", "Colour", "Remaining", "Slot"] { th(ui, h); }
                        ui.end_row();
                        for (k, s) in m.spools.iter().enumerate() {
                            if ui.selectable_label(m.st.sel_spool == k, format!("#{}", s.id)).clicked() { m.st.sel_spool = k; }
                            ui.label(s.brand);
                            ui.label(s.mat);
                            ui.label(s.color);
                            ui.horizontal(|ui| { bar(ui, s.left / s.total, 60.0, if s.left < 100.0 { Some(Status::Warn) } else { None }); ui.label(mono(format!("{:.0} g", s.left))); });
                            ui.label(mono(s.slot.map(|x| format!("T{}", x)).unwrap_or("-".into())));
                            ui.end_row();
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if button(ui, "Add spool") {
                            let id = m.spools.len() as u32 + 1;
                            m.spools.push(Spool { id, brand: "New", mat: "PLA", color: "Unset", left: 1000.0, total: 1000.0, slot: None, profile: 0 });
                        }
                        button(ui, "Scan tag");
                    });
                });
                panel(&mut c[1], "Selected spool", |ui| {
                    let k = m.st.sel_spool.min(m.spools.len() - 1);
                    let pnames: Vec<&str> = m.profiles.iter().map(|p| p.name).collect();
                    let s = &mut m.spools[k];
                    egui::Grid::new("spool_edit").num_columns(2).spacing([18.0, 6.0]).show(ui, |ui| {
                        ui.label(dim("ID")); ui.label(mono(format!("#{}", s.id))); ui.end_row();
                        ui.label(dim("Remaining")); ui.add(DragValue::new(&mut s.left).range(0.0..=s.total).suffix(" g")); ui.end_row();
                        ui.label(dim("Total")); ui.add(DragValue::new(&mut s.total).range(1.0..=10000.0).suffix(" g")); ui.end_row();
                        ui.label(dim("Profile"));
                        egui::ComboBox::from_id_salt("sp_prof").selected_text(pnames[s.profile]).show_ui(ui, |ui| {
                            for (i, n) in pnames.iter().enumerate() { ui.selectable_value(&mut s.profile, i, *n); }
                        });
                        ui.end_row();
                    });
                    ui.add_space(4.0);
                    ui.label(dim("Usage is subtracted from the extrusion count on job end."));
                });
            });
        }
        _ => {
            panel(ui, "Filament profiles", |ui| {
                egui::Grid::new("profiles").num_columns(7).striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
                    for h in ["Name", "Nozzle", "Bed", "Chamber", "Pressure adv.", "Flow", "Fan"] { th(ui, h); }
                    ui.end_row();
                    for p in m.profiles.iter_mut() {
                        ui.label(p.name);
                        ui.add(DragValue::new(&mut p.nozzle).range(150.0..=320.0).suffix(" C"));
                        ui.add(DragValue::new(&mut p.bed).range(0.0..=120.0).suffix(" C"));
                        ui.add(DragValue::new(&mut p.chamber).range(0.0..=80.0).suffix(" C"));
                        ui.add(DragValue::new(&mut p.pa).speed(0.001).range(0.0..=0.5));
                        ui.add(DragValue::new(&mut p.flow).speed(0.005).range(0.5..=1.5));
                        ui.add(DragValue::new(&mut p.fan).range(0.0..=100.0).suffix(" %"));
                        ui.end_row();
                    }
                });
            });
        }
    }
}

pub fn camera(ui: &mut Ui, m: &mut Model) {
    title(ui, "Camera", "Camera node streams to the Management Interface; failure detection runs on the PC");
    tabs(ui, &mut m.st.tab2, &["Live", "Failure detection", "Timelapse"]);
    match m.st.tab2 {
        0 => {
            ui.columns(2, |c| {
                panel(&mut c[0], "Stream", |ui| {
                    let w = ui.available_width();
                    let (r, _) = ui.allocate_exact_size(vec2(w, w * 0.5625), Sense::hover());
                    let p = ui.painter_at(r);
                    p.rect_filled(r, crate::theme::RAD, Color32::from_gray(6));
                    for k in 1..4 {
                        let x = r.left() + r.width() * k as f32 / 4.0;
                        let y = r.top() + r.height() * k as f32 / 4.0;
                        p.line_segment([egui::pos2(x, r.top()), egui::pos2(x, r.bottom())], Stroke::new(1.0_f32, Color32::from_gray(20)));
                        p.line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], Stroke::new(1.0_f32, Color32::from_gray(20)));
                    }
                    p.text(r.center(), Align2::CENTER_CENTER, "NO VIDEO - PLACEHOLDER", FontId::monospace(13.0), FAINT);
                    p.text(r.left_top() + vec2(8.0, 6.0), Align2::LEFT_TOP, format!("CAM1  1280x720  15 fps  {}", fmt_dur(m.t as f32)), FontId::monospace(10.0), DIM);
                    p.rect_stroke(r, crate::theme::RAD, Stroke::new(1.0_f32, LINE2), StrokeKind::Inside);
                });
                panel(&mut c[1], "Camera node", |ui| {
                    kv(ui, "cam_stat", &[
                        ("Node", "Camera  ESP32  WiFi".into()),
                        ("Bitrate", "2.4 Mbit/s".into()),
                        ("Latency", "180 ms".into()),
                        ("Viewers", "2 (desktop, phone)".into()),
                    ]);
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if button(ui, "Snapshot") { m.log(Sev::Info, "camera", "Snapshot saved"); }
                        button(ui, "Record clip");
                        button(ui, "Light on/off");
                    });
                    ui.add_space(6.0);
                    ui.label(dim("Remote screens and mobile control are views of the same API, not plugins."));
                });
            });
        }
        1 => {
            ui.columns(2, |c| {
                panel(&mut c[0], "Detection (runs on PC)", |ui| {
                    ui.checkbox(&mut m.det_on, "Spaghetti / failure detection");
                    egui::Grid::new("det").num_columns(2).spacing([18.0, 6.0]).show(ui, |ui| {
                        ui.label(dim("Sensitivity"));
                        ui.add(egui::Slider::new(&mut m.det_sens, 0.1..=1.0));
                        ui.end_row();
                        ui.label(dim("On detection"));
                        egui::ComboBox::from_id_salt("det_act").selected_text(["Pause via API", "Notify only", "Cancel"][m.det_action]).show_ui(ui, |ui| {
                            ui.selectable_value(&mut m.det_action, 0, "Pause via API");
                            ui.selectable_value(&mut m.det_action, 1, "Notify only");
                            ui.selectable_value(&mut m.det_action, 2, "Cancel");
                        });
                        ui.end_row();
                    });
                });
                panel(&mut c[1], "Recent detections", |ui| {
                    egui::Grid::new("det_log").num_columns(3).striped(true).spacing([16.0, 5.0]).show(ui, |ui| {
                        for h in ["Time", "Score", "Outcome"] { th(ui, h); }
                        ui.end_row();
                        for (t, s, o) in [("2026-10-27 22:41", "0.82", "paused (confirmed)"), ("2026-10-25 14:03", "0.61", "dismissed"), ("2026-10-22 03:17", "0.55", "dismissed")] {
                            ui.label(mono(t)); ui.label(mono(s)); ui.label(o);
                            ui.end_row();
                        }
                    });
                });
            });
        }
        _ => {
            panel(ui, "Timelapse", |ui| {
                ui.checkbox(&mut m.lapse_on, "Capture timelapse for each job");
                ui.horizontal(|ui| {
                    ui.label(dim("Capture"));
                    egui::ComboBox::from_id_salt("lapse").selected_text(["Every layer", "Every 30 s", "Every 5 layers"][m.lapse_every]).show_ui(ui, |ui| {
                        ui.selectable_value(&mut m.lapse_every, 0, "Every layer");
                        ui.selectable_value(&mut m.lapse_every, 1, "Every 30 s");
                        ui.selectable_value(&mut m.lapse_every, 2, "Every 5 layers");
                    });
                });
                ui.add_space(4.0);
                egui::Grid::new("lapse_list").num_columns(4).striped(true).spacing([16.0, 5.0]).show(ui, |ui| {
                    for h in ["Job", "Frames", "Length", ""] { th(ui, h); }
                    ui.end_row();
                    for (j, f, l) in [("benchy_calib", "62", "0:04"), ("enclosure_panel_left", "318", "0:21"), ("gear_train_x6", "205", "0:14")] {
                        ui.label(j); ui.label(mono(f)); ui.label(mono(l));
                        ui.horizontal(|ui| { let _ = ui.small_button("view"); let _ = ui.small_button("export"); });
                        ui.end_row();
                    }
                });
            });
        }
    }
}
