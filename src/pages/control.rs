use crate::data::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, DragValue, Ui};

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

pub fn thermal(ui: &mut Ui, m: &mut Model) {
    title(ui, "Thermal", "Heater control mode and cutoff limits. Live temperatures, presets and fans are on the Dashboard");
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
}
