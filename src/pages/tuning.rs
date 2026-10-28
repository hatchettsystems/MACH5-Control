use super::control::SHAPERS;
use crate::data::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, DragValue, RichText, Ui};

pub fn motion(ui: &mut Ui, m: &mut Model) {
    title(ui, "Motion", "Third-order jerk-limited planner. Input shaping and pressure advance live inside the planner");
    ui.columns(2, |c| {
        panel(&mut c[0], "Planner limits", |ui| {
            egui::Grid::new("planner").num_columns(2).spacing([20.0, 6.0]).show(ui, |ui| {
                ui.label(dim("Max velocity"));
                ui.add(DragValue::new(&mut m.vmax).range(10.0..=1500.0).suffix(" mm/s"));
                ui.end_row();
                for (k, ax) in ["X", "Y", "Z"].iter().enumerate() {
                    ui.label(dim(format!("Max accel {}", ax)));
                    ui.add(DragValue::new(&mut m.accel[k]).range(100.0..=40000.0).speed(50.0).suffix(" mm/s2"));
                    ui.end_row();
                }
                ui.label(dim("Max jerk"));
                ui.add(DragValue::new(&mut m.jerk).range(1000.0..=1.0e6).speed(500.0).suffix(" mm/s3"));
                ui.end_row();
                ui.label(dim("Max snap"));
                ui.add(DragValue::new(&mut m.snap).range(1.0e5..=1.0e8).speed(1.0e4).suffix(" mm/s4"));
                ui.end_row();
                ui.label(dim("Corner blend tolerance"));
                ui.add(DragValue::new(&mut m.blend).range(0.0..=1.0).speed(0.005).suffix(" mm"));
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.label(dim("No junction deviation: corners are blended inside the third-order profile. The benchmark that decides this is an open decision (see Spec)."));
        });
        panel(&mut c[1], "Profile preview (illustrative)", |ui| {
            let a = m.accel[0].max(100.0);
            let v = m.vmax.min(300.0);
            let ta = v / a;
            let tj = (a / m.jerk).max(0.0005) * 4.0 + ta * 0.6;
            let total = 2.0 * (ta + tj) + 0.04;
            let n = 120;
            let ramp = |t: f32, len: f32| { let x = (t / len).clamp(0.0, 1.0); x * x * (3.0 - 2.0 * x) };
            let mut trap = Vec::new();
            let mut s3 = Vec::new();
            let mut ac = Vec::new();
            for k in 0..=n {
                let t = total * k as f32 / n as f32;
                let (tv, sv);
                if t < total / 2.0 {
                    tv = (t / ta).min(1.0) * v;
                    sv = ramp(t, ta + tj) * v;
                } else {
                    let r = total - t;
                    tv = (r / ta).min(1.0) * v;
                    sv = ramp(r, ta + tj) * v;
                }
                trap.push([t * 1000.0, tv]);
                s3.push([t * 1000.0, sv]);
                let prev = if k > 0 { s3[k - 1][1] } else { 0.0 };
                ac.push([t * 1000.0, ((sv - prev) / (total / n as f32)) / a * v * 0.5]);
            }
            plot(ui, 180.0, &[Series::new("trapezoid v", G3, trap).dashed(), Series::new("3rd-order v", G1, s3), Series::new("accel (scaled)", G2, ac)], (0.0, total * 1000.0), (-v * 0.4, v * 1.05), "mm/s  vs  ms");
        });
    });
    ui.columns(2, |c| {
        panel(&mut c[0], "Input shaper (applied in planner)", |ui| {
            egui::Grid::new("shapers").num_columns(6).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
                for h in ["Axis", "Type", "Freq", "Damping", "Max accel", "Measured"] { th(ui, h); }
                ui.end_row();
                for s in m.shapers.iter_mut() {
                    ui.label(mono(s.axis));
                    egui::ComboBox::from_id_salt(("shp", s.axis)).width(90.0).selected_text(SHAPERS[s.kind]).show_ui(ui, |ui| {
                        for (k, n) in SHAPERS.iter().enumerate() { ui.selectable_value(&mut s.kind, k, *n); }
                    });
                    ui.add(DragValue::new(&mut s.freq).range(5.0..=200.0).speed(0.1).suffix(" Hz"));
                    ui.add(DragValue::new(&mut s.damp).range(0.0..=0.5).speed(0.005));
                    ui.label(mono(format!("{:.0}", s.max_accel)));
                    ui.label(dim(s.when));
                    ui.end_row();
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if button(ui, "Run resonance test") { m.st.tab = 3; m.goto = Some(Page::Calibration); }
                ui.label(dim("Results are saved per axis and applied automatically."));
            });
        });
        panel(&mut c[1], "Pressure advance (applied in planner)", |ui| {
            ui.checkbox(&mut m.pa_on, "Enable pressure advance");
            egui::Grid::new("pa").num_columns(2).spacing([20.0, 6.0]).show(ui, |ui| {
                ui.label(dim("Smooth time"));
                ui.add(DragValue::new(&mut m.pa_smooth).range(0.0..=0.2).speed(0.001).suffix(" s"));
                ui.end_row();
                ui.label(dim("Active value"));
                ui.label(mono(format!("{:.3} s", m.pa_value)).color(WHITE));
                ui.end_row();
            });
            ui.add_space(4.0);
            egui::Grid::new("pa_profiles").num_columns(3).striped(true).spacing([16.0, 5.0]).show(ui, |ui| {
                for h in ["Filament profile", "PA value", ""] { th(ui, h); }
                ui.end_row();
                for p in m.profiles.iter_mut() {
                    ui.label(p.name);
                    ui.add(DragValue::new(&mut p.pa).range(0.0..=0.5).speed(0.001));
                    if ui.small_button("use").clicked() { m.pa_value = p.pa; }
                    ui.end_row();
                }
            });
            if button(ui, "Calibrate...") { m.st.tab = 4; m.goto = Some(Page::Calibration); }
        });
    });
    panel(ui, "Drive mode", |ui| {
        ui.horizontal(|ui| {
            ui.label("Closed-loop / FOC");
            ui.label(dim("default on all axes"));
            ui.add_space(20.0);
            ui.add_enabled(false, egui::Checkbox::new(&mut false, "CAN-synchronised step-servo (v2)"));
            ui.label(dim("phase 7"));
        });
    });
}

fn psd(f: f32, peaks: &[(f32, f32, f32)]) -> f32 {
    let mut s = 1.5e-3;
    for (f0, w, a) in peaks {
        s += a / (1.0 + ((f - f0) / w).powi(2));
    }
    s
}

pub fn calibration(ui: &mut Ui, m: &mut Model) {
    title(ui, "Calibration", "One auto-calibration routine, plus resonance and pressure advance");
    tabs(ui, &mut m.st.tab, &["Auto-calibration", "Bed mesh", "Gantry leveling", "Resonance", "Pressure advance"]);
    match m.st.tab {
        0 => {
            ui.columns(2, |c| {
                panel(&mut c[0], "Auto-calibration routine", |ui| {
                    let steps = ["Home X / Y / Z (eddy)", "Probe thermal soak and compensation", "Gantry level (3 Z motors)", "X-twist calibration", "Adaptive bed mesh", "Verify and save"];
                    egui::Grid::new("autocal").num_columns(3).spacing([16.0, 7.0]).show(ui, |ui| {
                        for (k, s) in steps.iter().enumerate() {
                            let (st, txt) = if k < m.cal.step { (Status::Ok, "done") } else if k == m.cal.step && m.cal.running { (Status::Warn, "running") } else { (Status::Idle, "pending") };
                            dot(ui, st);
                            ui.label(*s);
                            if k == m.cal.step && m.cal.running { bar(ui, m.cal.prog, 90.0, None); } else { ui.label(dim(txt)); }
                            ui.end_row();
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let printing = m.state == MState::Printing;
                        ui.add_enabled_ui(!m.cal.running && !printing, |ui| {
                            if primary(ui, "Run auto-calibration") {
                                m.cal = Cal { running: true, step: 0, prog: 0.0 };
                                m.state = MState::Calibrating;
                                m.log(Sev::Info, "autocal", "Started: unattended Z home, mesh, X twist");
                            }
                        });
                        if m.cal.running && button(ui, "Abort") {
                            m.cal.running = false;
                            m.state = MState::Idle;
                        }
                        if printing { ui.label(dim("stop the print first")); }
                    });
                });
                panel(&mut c[1], "Last results", |ui| {
                    let (mn, mx) = m.mesh.iter().flatten().fold((9.0f32, -9.0f32), |a, v| (a.0.min(*v), a.1.max(*v)));
                    kv(ui, "autocal_res", &[
                        ("Date", "2026-10-28 08:05".into()),
                        ("Z offset", format!("{:+.3} mm", m.z_offset)),
                        ("Mesh range", format!("{:+.3} .. {:+.3} mm", mn, mx)),
                        ("Gantry tilt", format!("{:+.3} / {:+.3} / {:+.3} mm", m.tilt[0], m.tilt[1], m.tilt[2])),
                        ("X twist", format!("{:+.3} mm/m", m.xtwist)),
                        ("Probe temp", "34.2 C (compensated)".into()),
                    ]);
                    ui.add_space(6.0);
                    ui.checkbox(&mut m.mesh_adaptive, "Adaptive mesh (print footprint only)");
                    ui.checkbox(&mut m.purge_adaptive, "Adaptive purge (near print start)");
                });
            });
        }
        1 => {
            ui.columns(2, |c| {
                panel(&mut c[0], "Bed mesh (um deviation)", |ui| {
                    let b = if m.mesh_adaptive { objects_bounds(m) } else { None };
                    heatmap(ui, &m.mesh, vec2(ui.available_width().min(460.0), 400.0), (-0.12, 0.12), b);
                    ui.label(dim("White outline = adaptive region derived from parsed G-code."));
                });
                panel(&mut c[1], "Mesh", |ui| {
                    let all: Vec<f32> = m.mesh.iter().flatten().copied().collect();
                    let mean = all.iter().sum::<f32>() / all.len() as f32;
                    let sd = (all.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / all.len() as f32).sqrt();
                    let pts = if m.mesh_adaptive { 36 } else { 81 };
                    kv(ui, "mesh_stats", &[
                        ("Points probed", format!("{} of 81", pts)),
                        ("Mean", format!("{:+.3} mm", mean)),
                        ("Std dev", format!("{:.3} mm", sd)),
                        ("Probe time", format!("{} s", pts * 2)),
                    ]);
                    ui.add_space(6.0);
                    ui.checkbox(&mut m.mesh_adaptive, "Adaptive mesh");
                    ui.checkbox(&mut m.purge_adaptive, "Adaptive purge");
                    ui.add_space(4.0);
                    if primary(ui, "Re-mesh now") {
                        m.remesh();
                        m.log(Sev::Info, "mesh", "Bed mesh updated");
                    }
                });
            });
        }
        2 => {
            ui.columns(2, |c| {
                panel(&mut c[0], "Gantry leveling (probe-driven, any Z-motor count)", |ui| {
                    egui::Grid::new("lvl").num_columns(3).spacing([16.0, 7.0]).show(ui, |ui| {
                        for h in ["Motor", "Error", ""] { th(ui, h); }
                        ui.end_row();
                        for (k, n) in ["Z1 front-left", "Z2 front-right", "Z3 rear"].iter().enumerate() {
                            ui.label(*n);
                            ui.label(mono(format!("{:+.3} mm", m.tilt[k])));
                            bar(ui, m.tilt[k].abs() / 0.05, 100.0, if m.tilt[k].abs() > 0.02 { Some(Status::Warn) } else { None });
                            ui.end_row();
                        }
                    });
                    ui.add_space(6.0);
                    if primary(ui, "Level gantry") {
                        m.tilt = [0.001, -0.001, 0.000];
                        m.log(Sev::Info, "level", "Gantry levelled, residual < 2 um");
                    }
                });
                panel(&mut c[1], "X twist", |ui| {
                    let pts: Vec<[f32; 2]> = (0..=20).map(|k| { let x = k as f32 * 17.5; [x, m.xtwist * (x / 350.0 - 0.5) * 2.0 * 1000.0 + (x * 0.05).sin() * 2.0] }).collect();
                    plot(ui, 150.0, &[Series::new("correction", G1, pts)], (0.0, 350.0), (-30.0, 30.0), "um vs X mm");
                    ui.label(dim("Measured by the eddy probe along X; applied by the planner."));
                });
            });
        }
        3 => {
            panel(ui, "Resonance test", |ui| {
                ui.horizontal(|ui| {
                    ui.label(dim("Source"));
                    egui::ComboBox::from_id_salt("adxl_src").selected_text("Eddy probe ADXL345 (CAN B)").show_ui(ui, |ui| {
                        let _ = ui.selectable_label(true, "Eddy probe ADXL345 (CAN B)");
                        let _ = ui.selectable_label(false, "Wireless ADXL module (WiFi)");
                    });
                    if primary(ui, "Run test (X and Y)") {
                        m.st.res_done = true;
                        m.log(Sev::Info, "resonance", "Sweep complete on P4; shapers saved and applied");
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(dim("Report"));
                    for (k, n) in ["Spectrum", "Belt comparison", "Vibration vs speed"].iter().enumerate() {
                        if ui.selectable_label(m.st.tab2 == k, *n).clicked() { m.st.tab2 = k; }
                    }
                });
            });
            let fr: Vec<f32> = (0..=72).map(|k| 5.0 + k as f32 * 2.0).collect();
            match m.st.tab2 {
                0 => {
                    ui.columns(2, |c| {
                        panel(&mut c[0], "Power spectrum", |ui| {
                            let x: Vec<[f32; 2]> = fr.iter().map(|f| [*f, psd(*f, &[(48.2, 3.0, 0.9), (95.0, 6.0, 0.22)])]).collect();
                            let y: Vec<[f32; 2]> = fr.iter().map(|f| [*f, psd(*f, &[(41.6, 3.5, 1.0), (78.0, 5.0, 0.25)])]).collect();
                            plot(ui, 230.0, &[Series::new("X", G1, x), Series::new("Y", G2, y)], (5.0, 149.0), (0.0, 1.1), "PSD vs Hz");
                        });
                        panel(&mut c[1], "Recommended shapers", |ui| {
                            for s in m.shapers.iter() {
                                kv(ui, ("rec_".to_string() + s.axis).as_str(), &[
                                    ("Axis", s.axis.into()),
                                    ("Shaper", format!("{} @ {:.1} Hz", SHAPERS[s.kind], s.freq)),
                                    ("Max accel", format!("{:.0} mm/s2", s.max_accel)),
                                    ("Saved", s.when.into()),
                                ]);
                                ui.add_space(6.0);
                            }
                            ui.label(dim("Applied automatically in the planner."));
                        });
                    });
                }
                1 => panel(ui, "Belt comparison", |ui| {
                    let a: Vec<[f32; 2]> = fr.iter().map(|f| [*f, psd(*f, &[(48.2, 3.0, 0.9)])]).collect();
                    let b: Vec<[f32; 2]> = fr.iter().map(|f| [*f, psd(*f, &[(50.1, 3.2, 0.85)])]).collect();
                    plot(ui, 240.0, &[Series::new("belt A", G1, a), Series::new("belt B", G2, b)], (5.0, 149.0), (0.0, 1.1), "PSD vs Hz");
                    kv(ui, "belts", &[("Similarity", "94 %".into()), ("Peak offset", "1.9 Hz (tension difference ~3 %)".into())]);
                }),
                _ => panel(ui, "Vibration vs speed", |ui| {
                    let v: Vec<[f32; 2]> = (0..=50).map(|k| { let s = 20.0 + k as f32 * 10.0; [s, 12.0 + s * 0.02 + 30.0 / (1.0 + ((s - 190.0) / 18.0).powi(2))] }).collect();
                    plot(ui, 240.0, &[Series::new("vibration", G1, v)], (20.0, 520.0), (0.0, 80.0), "mg vs mm/s");
                    kv(ui, "vib", &[("Avoid", "170 - 210 mm/s".into()), ("Motor harmonic", "12000 rpm @ 180 mm/s on 20T pulley".into())]);
                }),
            }
        }
        _ => {
            ui.columns(2, |c| {
                panel(&mut c[0], "Calibration print", |ui| {
                    egui::Grid::new("pa_gen").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                        ui.label(dim("Profile"));
                        egui::ComboBox::from_id_salt("pa_prof").selected_text(m.profiles[m.st.pa_profile].name).show_ui(ui, |ui| {
                            for (k, p) in m.profiles.iter().enumerate() { ui.selectable_value(&mut m.st.pa_profile, k, p.name); }
                        });
                        ui.end_row();
                        ui.label(dim("Start"));
                        ui.add(DragValue::new(&mut m.st.pa_lo).range(0.0..=0.5).speed(0.001));
                        ui.end_row();
                        ui.label(dim("End"));
                        ui.add(DragValue::new(&mut m.st.pa_hi).range(0.0..=0.5).speed(0.001));
                        ui.end_row();
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if button(ui, "Generate and queue print") {
                            m.log(Sev::Info, "pa", "pa_calib_tower.gcode generated and queued");
                            m.st.pa_done = false;
                        }
                        if button(ui, "Analyse result") { m.st.pa_done = true; }
                    });
                });
                panel(&mut c[1], "Analysis", |ui| {
                    let pts: Vec<[f32; 2]> = (0..=40).map(|k| { let x = m.st.pa_lo + (m.st.pa_hi - m.st.pa_lo) * k as f32 / 40.0; [x, 0.6 + 120.0 * (x - 0.044).powi(2)] }).collect();
                    plot(ui, 150.0, &[Series::new("corner bulge", G1, pts)], (m.st.pa_lo, m.st.pa_hi.max(m.st.pa_lo + 0.01)), (0.0, 1.0), "mm vs PA");
                    if m.st.pa_done {
                        ui.label(RichText::new("Best value 0.044 s").monospace().color(WHITE));
                        if primary(ui, "Write to filament profile") {
                            let k = m.st.pa_profile;
                            m.profiles[k].pa = 0.044;
                            m.log(Sev::Info, "pa", &format!("Profile {} PA = 0.044", m.profiles[k].name));
                        }
                    } else {
                        ui.label(dim("Run the print, then analyse."));
                    }
                });
            });
        }
    }
}

fn objects_bounds(m: &Model) -> Option<(f32, f32, f32, f32)> {
    let mut it = m.job.objects.iter().filter(|o| !o.excluded);
    let f = it.next()?;
    let (mut x0, mut y0, mut x1, mut y1) = (f.x, f.y, f.x + f.w, f.y + f.h);
    for o in it {
        x0 = x0.min(o.x); y0 = y0.min(o.y); x1 = x1.max(o.x + o.w); y1 = y1.max(o.y + o.h);
    }
    Some(((x0 - 5.0) / 350.0, (y0 - 5.0) / 350.0, (x1 + 5.0) / 350.0, (y1 + 5.0) / 350.0))
}

pub fn probe(ui: &mut Ui, m: &mut Model) {
    title(ui, "Eddy Probe", "Own STM32 eddy probe on CAN FD B, read by the P4. No dedicated trigger line");
    let t = m.t as f32;
    ui.columns(2, |c| {
        panel(&mut c[0], "Driver and status", |ui| {
            ui.horizontal(|ui| {
                ui.label(dim("Driver"));
                egui::ComboBox::from_id_salt("probe_drv").selected_text(["MACH5 eddy probe", "Beacon (compatibility)", "Cartographer (compatibility)"][m.probe_driver]).show_ui(ui, |ui| {
                    ui.selectable_value(&mut m.probe_driver, 0, "MACH5 eddy probe");
                    ui.selectable_value(&mut m.probe_driver, 1, "Beacon (compatibility)");
                    ui.selectable_value(&mut m.probe_driver, 2, "Cartographer (compatibility)");
                });
            });
            ui.add_space(4.0);
            kv(ui, "probe_stat", &[
                ("Node", "Eddy Probe  0x30  CAN FD B".into()),
                ("Coil frequency", format!("{:.3} MHz", 3.412 + (t * 0.5).sin() * 0.0004)),
                ("Probe temperature", format!("{:.1} C", 52.0 + (t * 0.05).sin())),
                ("Sample rate", "412 frames/s (timestamped)".into()),
                ("Trigger latency", "180 us +/- 20 us (time-synced)".into()),
                ("Height", format!("{:.4} mm", 2.0 + (t * 3.0).sin() * 0.0015)),
            ]);
            if m.probe_driver != 0 { ui.label(dim("Compatibility driver only. Homing, mesh and X-twist still run in the unified routine.")); }
        });
        panel(&mut c[1], "Temperature compensation", |ui| {
            ui.checkbox(&mut m.probe_comp, "Temperature-compensated Z");
            let raw: Vec<[f32; 2]> = (0..=30).map(|k| { let tc = 25.0 + k as f32 * 2.0; [tc, 2.0 + (tc - 25.0) * 0.0021] }).collect();
            let comp: Vec<[f32; 2]> = raw.iter().map(|p| [p[0], if m.probe_comp { 2.0 + (p[0] * 0.3).sin() * 0.0004 } else { p[1] }]).collect();
            plot(ui, 150.0, &[Series::new("raw", G3, raw).dashed(), Series::new("compensated", G1, comp)], (25.0, 85.0), (1.95, 2.15), "mm vs probe C");
        });
    });
    ui.columns(2, |c| {
        panel(&mut c[0], "Live height", |ui| {
            let pts: Vec<[f32; 2]> = (0..200).map(|k| { let x = k as f32; [x, 2.0 + ((t * 8.0 + x * 0.7).sin() * 0.0012 + (x * 2.3).cos() * 0.0007)] }).collect();
            plot(ui, 140.0, &[Series::new("z", G1, pts)], (0.0, 200.0), (1.99, 2.01), "mm");
        });
        panel(&mut c[1], "Procedures", |ui| {
            egui::Grid::new("probe_btns").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                ui.label(dim("Z offset"));
                ui.add(DragValue::new(&mut m.z_offset).speed(0.001).range(-2.0..=2.0).suffix(" mm"));
                ui.end_row();
            });
            ui.horizontal_wrapped(|ui| {
                if button(ui, "Calibrate sensor") { m.log(Sev::Info, "probe", "Sensor calibration curve stored on probe node"); }
                if button(ui, "Accuracy test x10") { m.st.probe_ok = true; }
                if primary(ui, "Run unified auto-calibration") { m.st.tab = 0; m.goto = Some(Page::Calibration); }
            });
            if m.st.probe_ok { kv(ui, "probe_acc", &[("Std dev", "0.0011 mm".into()), ("Range", "0.0034 mm".into()), ("Samples", "10".into())]); }
            ui.label(dim("Optional X/Y accelerometer (ADXL345) is read on the same node."));
        });
    });
}

pub fn drives(ui: &mut Ui, m: &mut Model) {
    title(ui, "Drives", "FOC step-servo drivers: STM32G431 + MT6826S + DRV8262, 48 V, 4 A RMS");
    panel(ui, "Drivers", |ui| {
        egui::Grid::new("drives").num_columns(9).striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
            for h in ["Axis", "Motor", "State", "Current", "Following err.", "Temp", "Vbus", "Mode", "Tuned"] { th(ui, h); }
            ui.end_row();
            for (k, d) in m.drives.iter().enumerate() {
                if ui.selectable_label(m.st.sel_drive == k, d.axis).clicked() { m.st.sel_drive = k; }
                ui.label(dim(d.motor));
                status(ui, d.status, "ready");
                ui.label(mono(format!("{:.2} A", d.cur)));
                ui.label(mono(format!("{:.1} um", d.follow)));
                ui.label(mono(format!("{:.0} C", d.temp)));
                ui.label(mono(format!("{:.1} V", d.vbus)));
                ui.label(["FOC closed-loop", "Step-servo CAN"][d.mode]);
                ui.label(dim(d.tuned));
                ui.end_row();
            }
        });
    });
    let k = m.st.sel_drive.min(m.drives.len() - 1);
    ui.columns(2, |c| {
        panel(&mut c[0], &format!("Motor characterisation - axis {}", m.drives[k].axis), |ui| {
            let d = &mut m.drives[k];
            egui::Grid::new("drive_tune").num_columns(2).spacing([18.0, 6.0]).show(ui, |ui| {
                ui.label(dim("Phase resistance"));
                ui.add(DragValue::new(&mut d.r).speed(0.01).suffix(" ohm"));
                ui.end_row();
                ui.label(dim("Phase inductance"));
                ui.add(DragValue::new(&mut d.l).speed(0.01).suffix(" mH"));
                ui.end_row();
                ui.label(dim("Torque constant"));
                ui.add(DragValue::new(&mut d.kt).speed(0.001).suffix(" Nm/A"));
                ui.end_row();
                ui.label(dim("Pole pairs"));
                ui.add(DragValue::new(&mut d.poles).range(1..=200));
                ui.end_row();
                ui.label(dim("Current (RMS)"));
                ui.add(DragValue::new(&mut d.irms).speed(0.05).range(0.1..=4.0).suffix(" A"));
                ui.end_row();
                ui.label(dim("Mode"));
                egui::ComboBox::from_id_salt("drive_mode").selected_text(["FOC closed-loop", "Step-servo CAN (v2)"][d.mode]).show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.mode, 0, "FOC closed-loop");
                    ui.add_enabled(false, egui::Button::new("Step-servo CAN (v2) - not available"));
                });
                ui.end_row();
            });
            ui.add_space(6.0);
            if primary(ui, "Self-characterise motor") {
                d.tuned = "2026-10-28";
                m.log(Sev::Info, "drive", "Motor characterised, tune values stored in config");
            }
            ui.label(dim("No manual register work. Values are stored per motor in config."));
        });
        panel(&mut c[1], "Interface and following error", |ui| {
            kv(ui, "drive_if", &[
                ("STEP / DIR / EN", "hardwired".into()),
                ("CAN FD A", format!("{} (config and faults)", m.drives[k].node)),
                ("Encoder", m.drives[k].enc.into()),
                ("Fault limit", "200 um".into()),
            ]);
            ui.add_space(4.0);
            plot(ui, 130.0, &[Series::from_ring("following error", G1, &m.h_follow), Series::new("limit", G3, vec![[0.0, 200.0], [HIST as f32, 200.0]]).dashed()], (0.0, HIST as f32), (0.0, 250.0), "um");
        });
    });
}
