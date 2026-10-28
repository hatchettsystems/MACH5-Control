//! Guided walkthrough: a spotlight overlay that navigates the app, points at real
//! on-screen elements (panels register their rectangles) and runs small live demos.
use crate::data::*;
use crate::theme::*;
use crate::ui::{bar, dim, get_anchor, mono};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Context, FontId, Id, Key, LayerId, Margin, Order, Pos2, Rect, RichText, Sense, Stroke};

#[derive(Clone, Copy, PartialEq)]
pub enum Demo {
    SwitchMachine,
    ExcludeObject,
    FwFlash,
    Alert,
    Rollout,
    SimRun,
}

impl Demo {
    fn hint(self) -> &'static str {
        match self {
            Demo::SwitchMachine => "Live demo: switching to Tomahawk-05, then back",
            Demo::ExcludeObject => "Live demo: excluding bracket_B from the running job",
            Demo::FwFlash => "Live demo: flashing the out-of-date MMU node",
            Demo::Alert => "Live demo: a failure alert arrives for Tomahawk-04",
            Demo::Rollout => "Live demo: roll-out advancing wave by wave",
            Demo::SimRun => "Live demo: running the planner simulation",
        }
    }
    fn apply(self, m: &mut Model) {
        match self {
            Demo::SwitchMachine => m.switch_machine(4),
            Demo::ExcludeObject => m.job.objects[1].excluded = true,
            Demo::FwFlash => {
                m.fw.running = true;
                m.fw.idx = 0;
            }
            Demo::Alert => {
                m.alert(3, Sev::Err, "Failure detected (spaghetti, score 0.91) - print paused via API");
                m.m_pause(3);
            }
            Demo::Rollout => {
                for f in m.fleet.iter_mut().filter(|f| f.wave > 0) {
                    f.fw = "1.4.1".into();
                }
                m.rollout = Rollout { wave: 0, prog: 0.0, running: false, halted: None, done: false };
                m.rollout_gate();
                m.rollout.running = m.rollout.halted.is_none();
            }
            Demo::SimRun => {
                m.sim.running = true;
                m.sim.prog = 0.0;
                m.sim.results.clear();
            }
        }
    }
    fn revert(self, m: &mut Model) {
        match self {
            Demo::SwitchMachine => m.switch_machine(0),
            Demo::ExcludeObject => m.job.objects[1].excluded = false,
            Demo::FwFlash => m.fw.running = false,
            Demo::Alert => m.m_resume(3),
            Demo::Rollout => m.rollout.running = false,
            Demo::SimRun => {}
        }
    }
}

pub struct Step {
    page: Page,
    tab: Option<usize>,
    tab2: Option<usize>,
    target: &'static str,
    title: &'static str,
    body: &'static str,
    demo: Option<Demo>,
    nav: bool,
}

const fn s(page: Page, tab: Option<usize>, tab2: Option<usize>, target: &'static str, title: &'static str, body: &'static str, demo: Option<Demo>, nav: bool) -> Step {
    Step { page, tab, tab2, target, title, body, demo, nav }
}

use Page as P;

pub const STEPS: &[Step] = &[
    s(P::Dashboard, None, None, "", "Welcome to MACH5 Control", "This tour walks through every major area of the Management Interface. Everything you see is mock data driving a simulated farm of 10 machines. Use the Right and Left arrow keys or the buttons below. Esc leaves the tour at any time.", None, true),
    s(P::Dashboard, None, None, "nav", "Navigation", "Pages are grouped by what you are doing: Machine (run it), Tuning (make it print well), Materials, Hardware (the CAN FD bus and nodes) and System (fleet, config, rules, API). A square next to a page marks something that needs attention.", None, true),
    s(P::Dashboard, None, None, "navtoggle", "Closable sidebar", "Close the bar for more room with this button or Ctrl+B. The current page always stays named in the title bar.", None, true),
    s(P::Dashboard, None, None, "switcher", "Machine switcher", "Every page shows one machine at a time. This list shows every machine in the farm with its live state. Watch: the tour switches to Tomahawk-05 and the name, job and temperatures change everywhere, then it switches back.", Some(Demo::SwitchMachine), true),
    s(P::Dashboard, None, None, "estop", "E-STOP", "Latches a fault: heaters off, drives disabled. Safety is enforced on the STM32 and on the nodes themselves, so it holds even if the main controller or the network fails. A latched fault clears only by an explicit Host command.", None, true),
    s(P::Dashboard, None, None, "Job", "Dashboard: job", "Live job progress, layers, elapsed and remaining time. Pause, Resume and Cancel act on the machine; Speed and Flow scale the running print. Time runs 30x faster than real time in this prototype.", None, true),
    s(P::Dashboard, None, None, "Temperatures", "Dashboard: temperatures", "Current and target temperature per heater; the bar is heater output. Cutoff limits are enforced on the STM32 and the node, independent of the main controller.", None, true),
    s(P::Dashboard, None, None, "Thermal history (60 s)", "Live plots", "Rolling plot of hotend, target (dashed), bed and chamber, drawn straight from telemetry streamed over the API.", None, true),
    s(P::Dashboard, None, None, "Safety", "Safety panel", "Heater watchdog, node heartbeats, master time sync (skew must stay under 100 us) and drive faults. The P4 owns decisions; the STM32 owns deadlines. Nodes never clear a fault without a Host command.", None, true),
    s(P::Control, None, None, "Jog", "Control: jog", "Manual motion. Pick a step size, jog X, Y or Z, home axes or park. Extrusion is blocked until the hotend is above 170 C.", None, true),
    s(P::Control, None, None, "Console", "Typed console", "Commands are validated against the API schema, so an unknown command is rejected instead of guessed. Try G28, M104 S250 or STATUS.", None, true),
    s(P::Print, None, None, "Files (SD)", "Print: files", "Files on the machine's SD card, with material, time and object count read from G-code metadata. Print starts a job; the queue holds what runs next.", None, true),
    s(P::Print, None, None, "Object exclusion - click an object to cancel it mid-print", "Object exclusion", "The planner knows every object's boundary from the G-code metadata, so any object can be cancelled mid-print. Watch: the tour excludes bracket_B. Clicking an object, or its Exclude button, does the same.", Some(Demo::ExcludeObject), true),
    s(P::Print, None, None, "Objects", "Power-loss recovery", "The Host writes a checkpoint to flash every layer. After a power loss you can resume from the last layer with the Z height and thermal state restored.", None, true),
    s(P::Thermal, None, None, "Heaters", "Thermal: heaters", "Target, control mode (PID or MPC), maximum temperature and watchdog timeout per heater. If a heater node misses the Host heartbeat for that long, it cuts the heater locally.", None, true),
    s(P::Motion, None, None, "Planner limits", "Motion: planner", "A third-order, jerk-limited planner. There is no junction deviation: corners are blended inside the motion profile. The preview compares a trapezoid with the smoother third-order velocity curve.", None, true),
    s(P::Motion, None, None, "Input shaper (applied in planner)", "Shaping inside the planner", "Input shaping and pressure advance are applied inside the planner, not as post-filters. Values come from the resonance test and are applied automatically per axis.", None, true),
    s(P::Calibration, Some(0), None, "Auto-calibration routine", "Auto-calibration", "One routine does Z home, probe thermal compensation, gantry leveling, X twist and the adaptive mesh. It needs an idle machine, so it is disabled while a print is running.", None, true),
    s(P::Calibration, Some(1), None, "Bed mesh (um deviation)", "Adaptive bed mesh", "Mesh heat map in micrometres. With adaptive mesh on, only the white outline - the print footprint taken from the G-code - is probed, which saves time.", None, true),
    s(P::Calibration, Some(3), Some(0), "Power spectrum", "Resonance testing", "The ADXL on the eddy probe (or the wireless module) records vibration, the P4 computes the spectrum, and the best shaper per axis is chosen and applied. Reports also cover belt comparison and vibration versus speed.", None, true),
    s(P::Probe, None, None, "Temperature compensation", "Eddy probe", "An eddy probe reading drifts with temperature. The probe node reports its own temperature and Z is compensated: the flat solid line against the dashed raw curve.", None, true),
    s(P::Drives, None, None, "Motor characterisation - axis X", "FOC drivers", "Drivers measure their motor at setup - resistance, inductance, torque constant - and store the tune per motor, with no manual register work. STEP/DIR/EN stay hardwired; CAN carries config and faults.", None, true),
    s(P::Materials, Some(0), None, "MMU node (CAN FD B, 0x33)", "Multi-material", "The MMU is a CAN node with its own state machine: idle, select, feed, loaded, retract, fault. Spools are tracked per slot and in the spool inventory tab.", None, true),
    s(P::Camera, Some(0), Some(1), "Detection (runs on PC)", "Failure detection", "The camera node streams to this app. Detection runs on the PC, not on the machine, and pauses the print through the API when it triggers.", None, true),
    s(P::CanBus, None, None, "Segment A - drive", "Two CAN FD segments", "Segment A (drive) stays quiet and deterministic; segment B (peripherals) carries sensor streams and firmware transfers, so a flash can never delay a drive fault. Both show live bus load.", None, true),
    s(P::CanBus, None, None, "Message classes", "Message priorities", "Priority follows message class: faults and e-stop highest, then time sync and heartbeat, down to bulk firmware transfer. The frame monitor below shows live traffic.", None, true),
    s(P::Nodes, None, None, "Hardware family", "Nodes and discovery", "Every board shares one node core: identity, heartbeat and bootloader. The Host assigns short addresses and refuses to run if a configured node is missing or the wrong class.", None, true),
    s(P::Firmware, None, None, "Signed manifest", "Signed, dual-bank updates", "Only nodes whose firmware hash differs from the signed manifest are flashed. Images go to the inactive bank and are boot-checked; a failed check rolls back. Watch the MMU update.", Some(Demo::FwFlash), true),
    s(P::Fleet, Some(0), None, "Machines (live)", "Fleet overview", "The farm: ten Tomahawks printing at once plus dev benches. Tick machines for bulk actions, open one to drive it, or pause and resume from its row. Totals sit in the strip above.", None, true),
    s(P::Fleet, Some(1), None, "Dispatcher", "Queue and dispatcher", "Queue jobs for the farm. The dispatcher sends the highest-priority job to the next idle machine and skips faulted ones, so a machine that finishes a print picks up the next job on its own.", None, true),
    s(P::Fleet, Some(2), None, "tabs", "Camera wall", "A live wall of every machine. Each tile animates from that machine's real state: the stack grows with progress and the nozzle moves. Click a tile to open that machine.", None, true),
    s(P::Fleet, Some(3), None, "Farm alerts and failure-detection feed", "Farm alerts", "Failures, low spools and door events from every machine land here. With auto-pause on, a detected failure pauses that print through the API. Watch a new alert arrive for Tomahawk-04.", Some(Demo::Alert), true),
    s(P::Fleet, Some(4), None, "Stock by material", "Shared spool stock", "Spool stock is shared across the farm: loaded versus in stock per material, a low-stock threshold and reorder warnings. Machines consume filament as they print.", None, true),
    s(P::Fleet, Some(6), None, "Staged firmware roll-out to 1.4.3", "Staged roll-out", "Firmware rolls out in waves with a health gate before each: a wave will not start if a machine in it has a latched fault. Watch the waves advance.", Some(Demo::Rollout), true),
    s(P::Config, None, None, "Sections", "Typed config editor", "The editor is built from a typed schema: every field has a type, unit, range and description. Editing marks a field modified; out-of-range values block Apply; Apply writes with ack and readback.", None, true),
    s(P::Rules, None, None, "Editor", "Rules engine", "The sandboxed rules engine replaces shell macros: a trigger, a condition and ordered actions, with CPU and memory limits and no way to override safety limits.", None, true),
    s(P::Api, Some(0), None, "Paired clients", "API and keys", "Every client - desktop, phone, services - has its own key; rotate or revoke it here. The API is the only way into a machine, and its schema is signed.", None, true),
    s(P::Simulator, None, None, "Planner run", "Simulator", "Runs the planner and a virtual CAN bus on the PC so firmware can be tested before hardware exists, and compares the result with a reference run. Fault injection checks the safe-state behaviour.", Some(Demo::SimRun), true),
    s(P::Spec, Some(0), None, "Roadmap - each phase ends in a gate that must pass before the next starts", "Spec and roadmap", "The spec lives here too: roadmap phases and their gates, feature coverage with links to each panel, design principles and the open decisions.", None, true),
    s(P::Dashboard, None, None, "", "That is the tour", "Restart it any time from the Tutorial button in the sidebar, or press F1.", None, true),
];

pub struct Tour {
    pub step: usize,
    entered: Option<usize>,
    t0: f64,
    demo_done: bool,
    cur_demo: Option<Demo>,
    rect: Option<Rect>,
    card: Option<Pos2>,
    restore: (Page, bool),
}

const CARD_W: f32 = 372.0;
const CARD_H: f32 = 205.0;

fn lerp_rect(a: Rect, b: Rect, k: f32) -> Rect {
    Rect::from_min_max(a.min + (b.min - a.min) * k, a.max + (b.max - a.max) * k)
}

fn nearest(r: Rect, p: Pos2) -> Pos2 {
    pos2(p.x.clamp(r.left(), r.right()), p.y.clamp(r.top(), r.bottom()))
}

impl Tour {
    pub fn new(step: usize, page: Page, nav_open: bool) -> Self {
        Tour { step: step.min(STEPS.len() - 1), entered: None, t0: 0.0, demo_done: false, cur_demo: None, rect: None, card: None, restore: (page, nav_open) }
    }

    fn finish(&mut self, m: &mut Model, page: &mut Page, nav_open: &mut bool) {
        if let Some(d) = self.cur_demo.take() {
            d.revert(m);
        }
        *page = self.restore.0;
        *nav_open = self.restore.1;
        m.st.tab = 0;
        m.st.tab2 = 0;
        m.st.farm_tab = 0;
    }

    /// Draw the tour for one frame. Returns false when the tour is over.
    pub fn show(&mut self, ctx: &Context, m: &mut Model, page: &mut Page, nav_open: &mut bool) -> bool {
        let now = ctx.input(|i| i.time);
        let n = STEPS.len();
        let (key_next, key_back, key_esc) = ctx.input(|i| (i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::Space) || i.key_pressed(Key::Enter), i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::Escape)));

        if self.entered != Some(self.step) {
            if let Some(d) = self.cur_demo.take() {
                d.revert(m);
            }
            let st = &STEPS[self.step];
            *page = st.page;
            if st.nav {
                *nav_open = true;
            }
            m.st.tab = st.tab.unwrap_or(0);
            m.st.farm_tab = st.tab.unwrap_or(0);
            m.st.tab2 = st.tab2.unwrap_or(0);
            self.entered = Some(self.step);
            self.t0 = now;
            self.demo_done = false;
        }
        let st = &STEPS[self.step];
        if !self.demo_done && now - self.t0 > 1.0 {
            if let Some(d) = st.demo {
                d.apply(m);
                self.cur_demo = Some(d);
            }
            self.demo_done = true;
        }

        // Spotlight target (smoothly follows when the target changes).
        ctx.data_mut(|d| d.insert_temp(Id::new("tour_focus"), st.target.to_string()));
        let target = if st.target.is_empty() { None } else { get_anchor(ctx, st.target) };
        let k = (ctx.input(|i| i.stable_dt) * 12.0).min(1.0);
        match (target, self.rect) {
            (Some(t), Some(c)) => self.rect = Some(lerp_rect(c, t, k)),
            (Some(t), None) => self.rect = Some(t),
            (None, _) if st.target.is_empty() => self.rect = None,
            _ => {}
        }

        let screen = ctx.screen_rect();
        let t = now as f32;
        let age = (now - self.t0) as f32;
        let fade = (age * 4.0).min(1.0);

        // Input blocker + dimming + spotlight.
        egui::Area::new(Id::new("tour_block")).order(Order::Foreground).fixed_pos(pos2(0.0, 0.0)).show(ctx, |ui| {
            ui.allocate_exact_size(screen.size(), Sense::click_and_drag());
            let p = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("tour_paint")));
            let dimc = Color32::from_black_alpha((175.0 * fade) as u8);
            match self.rect {
                Some(r) => {
                    let sp = r.expand(6.0).intersect(screen);
                    p.rect_filled(Rect::from_min_max(screen.min, pos2(screen.right(), sp.top())), 0.0, dimc);
                    p.rect_filled(Rect::from_min_max(pos2(screen.left(), sp.bottom()), screen.max), 0.0, dimc);
                    p.rect_filled(Rect::from_min_max(pos2(screen.left(), sp.top()), pos2(sp.left(), sp.bottom())), 0.0, dimc);
                    p.rect_filled(Rect::from_min_max(pos2(sp.right(), sp.top()), pos2(screen.right(), sp.bottom())), 0.0, dimc);
                    let pulse = 0.5 + 0.5 * (t * 4.0).sin();
                    p.rect_stroke(sp, RAD + 2.0, Stroke::new(2.0_f32, Color32::from_white_alpha((150.0 + 100.0 * pulse) as u8)), egui::StrokeKind::Outside);
                    let ph = (t * 0.9).fract();
                    p.rect_stroke(sp.expand(ph * 22.0), RAD + 2.0, Stroke::new(1.5_f32, Color32::from_white_alpha(((1.0 - ph) * 130.0) as u8)), egui::StrokeKind::Outside);
                    // corner brackets
                    let b = 12.0;
                    let w = Stroke::new(3.0_f32, Color32::WHITE);
                    for (c, dx, dy) in [(sp.left_top(), 1.0, 1.0), (sp.right_top(), -1.0, 1.0), (sp.left_bottom(), 1.0, -1.0), (sp.right_bottom(), -1.0, -1.0)] {
                        p.line_segment([c, c + vec2(b * dx, 0.0)], w);
                        p.line_segment([c, c + vec2(0.0, b * dy)], w);
                    }
                }
                None => {
                    p.rect_filled(screen, 0.0, dimc);
                }
            }
        });

        // Card position: next to the target if it fits, otherwise centred / bottom.
        let margin = 14.0;
        let fits = |p: Pos2| p.x >= margin && p.y >= margin && p.x + CARD_W <= screen.right() - margin && p.y + CARD_H <= screen.bottom() - margin;
        let (want, leader) = match self.rect {
            None => (screen.center() - vec2(CARD_W / 2.0, CARD_H / 2.0), false),
            Some(r) => {
                let cands = [
                    pos2(r.right() + 30.0, r.center().y - CARD_H / 2.0),
                    pos2(r.center().x - CARD_W / 2.0, r.bottom() + 30.0),
                    pos2(r.left() - 30.0 - CARD_W, r.center().y - CARD_H / 2.0),
                    pos2(r.center().x - CARD_W / 2.0, r.top() - 30.0 - CARD_H),
                ];
                let clamp = |p: Pos2| pos2(p.x.clamp(margin, (screen.right() - margin - CARD_W).max(margin)), p.y.clamp(margin, (screen.bottom() - margin - CARD_H).max(margin)));
                match cands.iter().find(|c| fits(**c)) {
                    Some(c) => (clamp(*c), true),
                    None => (pos2(screen.right() - margin - CARD_W, screen.bottom() - margin - CARD_H), false),
                }
            }
        };
        let kc = (ctx.input(|i| i.stable_dt) * 10.0).min(1.0);
        let cur = match self.card { Some(c) => c + (want - c) * kc, None => want };
        self.card = Some(cur);

        // Leader line with travelling dots and an arrowhead.
        if let (true, Some(r)) = (leader, self.rect) {
            let card_rect = Rect::from_min_size(cur, vec2(CARD_W, CARD_H));
            let sp = r.expand(6.0);
            let a = nearest(card_rect, sp.center());
            let b = nearest(sp, card_rect.center());
            let p = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("tour_line")));
            let n_dots = 6;
            for i in 0..n_dots {
                let f = ((i as f32 + (t * 1.6).fract()) / n_dots as f32).clamp(0.0, 1.0);
                let q = a + (b - a) * f;
                p.rect_filled(Rect::from_center_size(q, vec2(4.0, 4.0)), 0.0, Color32::from_white_alpha((80.0 + 175.0 * f) as u8));
            }
            let d = (b - a).normalized();
            let nrm = vec2(-d.y, d.x);
            p.add(egui::Shape::convex_polygon(vec![b, b - d * 12.0 + nrm * 6.0, b - d * 12.0 - nrm * 6.0], Color32::WHITE, Stroke::NONE));
        }

        // Callout card.
        let mut action = 0; // 1 next, 2 back, 3 exit
        let typed: String = st.body.chars().take((age * 110.0) as usize).collect();
        let (title, hint, last) = (st.title, st.demo.map(|d| d.hint()), self.step + 1 == n);
        let step_no = self.step;
        egui::Area::new(Id::new("tour_card")).order(Order::Tooltip).fixed_pos(cur).show(ctx, |ui| {
            ui.set_opacity(fade);
            egui::Frame::new().fill(Color32::from_rgb(14, 14, 14)).stroke(Stroke::new(1.0_f32, Color32::from_gray(210))).corner_radius(RAD as u8).inner_margin(Margin::same(14)).show(ui, |ui| {
                ui.set_width(CARD_W - 30.0);
                ui.set_min_height(CARD_H - 30.0);
                ui.horizontal(|ui| {
                    ui.label(mono(format!("STEP {} / {}", step_no + 1, n)).size(11.0).color(DIM));
                });
                bar(ui, (step_no + 1) as f32 / n as f32, CARD_W - 30.0, None);
                ui.add_space(4.0);
                ui.label(RichText::new(title).size(17.0).strong().color(WHITE));
                ui.add_space(2.0);
                ui.label(RichText::new(typed).size(13.0).color(TEXT));
                if let Some(h) = hint {
                    ui.add_space(4.0);
                    let dots = ".".repeat(1 + ((t * 2.5) as usize % 3));
                    ui.label(dim(format!("{}{}", h, dots)));
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(step_no > 0, |ui| {
                        if ui.button("Back").clicked() { action = 2; }
                    });
                    if crate::ui::primary(ui, if last { "Finish" } else { "Next" }) { action = 1; }
                    if ui.button("Skip tour").clicked() { action = 3; }
                    ui.label(dim("<- ->  Esc"));
                });
            });
        });

        if key_next { action = 1; }
        if key_back && self.step > 0 { action = 2; }
        if key_esc { action = 3; }
        match action {
            1 => {
                if self.step + 1 < n { self.step += 1; } else { self.finish(m, page, nav_open); return false; }
            }
            2 => self.step -= 1,
            3 => { self.finish(m, page, nav_open); return false; }
            _ => {}
        }
        let _ = FontId::monospace(10.0);
        let _ = Align2::CENTER_CENTER;
        true
    }
}
