//! Live simulation of the whole farm. The active machine is the full `Model`;
//! every other machine is advanced here at an accelerated clock.
use crate::data::*;
use crate::theme::Status;

/// Simulated seconds per real second.
pub const SPEEDUP: f32 = 30.0;
pub const GROUPS: [&str; 3] = ["Row A", "Row B", "Dev bench"];
pub const TARGET_FW: &str = "1.4.3";
/// Jobs the farm can run: (file, material, minutes).
pub const JOBS: [(&str, &str, u32, f32); 6] = [
    ("housing_set_v7.gcode", "PA-CF", 460, 312.0),
    ("benchy_calib.gcode", "PETG", 62, 14.0),
    ("enclosure_panel_left.gcode", "ASA", 318, 186.0),
    ("gear_train_x6.gcode", "PLA", 205, 98.0),
    ("pa_calib_tower.gcode", "PETG", 18, 6.0),
    ("mmu_test_4color.gcode", "PLA", 144, 71.0),
];

pub fn mat_temps(mat: &str) -> [f32; 3] {
    match mat {
        "PLA" => [215.0, 60.0, 0.0],
        "PETG" => [240.0, 80.0, 0.0],
        "ASA" => [255.0, 100.0, 55.0],
        "PA-CF" => [275.0, 90.0, 60.0],
        _ => [220.0, 60.0, 0.0],
    }
}

fn hs(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 13;
    (x & 0xFFFF) as f32 / 65535.0
}

pub fn eta(f: &FleetMachine) -> f32 {
    ((1.0 - f.prog) * f.total).max(0.0)
}

#[allow(clippy::too_many_arguments)]
pub fn machine(idx: u32, name: String, ip: String, model: &'static str, group: usize, state: MState, job: Option<(usize, f32)>, fw: &str, nodes: (u8, u8), health: Status, wave: u8) -> FleetMachine {
    let (file, mat, mins, prog) = match job {
        Some((j, p)) => (JOBS[j].0.to_string(), JOBS[j].1, JOBS[j].2, p),
        None => (String::new(), "PLA", 0, 0.0),
    };
    let total = mins as f32 * 60.0;
    let hot = matches!(state, MState::Printing | MState::Paused);
    let targets = if hot { mat_temps(mat) } else if state == MState::Calibrating { [150.0, 60.0, 0.0] } else { [0.0; 3] };
    let temps = if state == MState::Fault { [95.0, 55.0, 40.0] } else if hot || state == MState::Calibrating { targets } else { [24.0, 24.0, 24.0] };
    FleetMachine {
        name, model, ip, group, state, file, material: mat, prog,
        layer: (prog * (mins as f32 / 1.6)) as u32, layers: (mins as f32 / 1.6).max(1.0) as u32,
        elapsed: prog * total, total, temps, targets, fw: fw.into(), nodes, sel: false, health,
        fp: format!("SHA256:{:04x}...{:04x}", (hs(idx, 1) * 65535.0) as u32, (hs(idx, 2) * 65535.0) as u32),
        key_age: (hs(idx, 3) * 90.0) as u32 + 5, wave,
        jobs_done: 40 + (hs(idx, 4) * 160.0) as u32, hours: 300.0 + hs(idx, 5) * 1500.0,
        grams: 8000.0 + hs(idx, 6) * 80000.0, fails: 1 + (hs(idx, 7) * 4.0) as u32,
        util: std::array::from_fn(|d| 0.35 + 0.6 * hs(idx * 7 + d as u32, 9)),
        nozzle: 0.4, slots: if model == "Dev bench" { 1 } else { 4 }, notes: String::new(),
    }
}

/// Ten Tomahawks running at once, four idle Tomahawks to play with, plus two dev benches.
pub fn make_fleet() -> Vec<FleetMachine> {
    use MState::*;
    let ok = Status::Ok;
    let rows: [(&str, &str, &str, usize, MState, Option<(usize, f32)>, &str, (u8, u8), Status, u8); 16] = [
        ("Tomahawk-01", "10.0.4.21", "Tomahawk", 0, Printing, Some((0, 0.42)), "1.4.2", (13, 13), ok, 2),
        ("Tomahawk-02", "10.0.4.22", "Tomahawk", 0, Printing, Some((2, 0.74)), "1.4.2", (13, 13), ok, 2),
        ("Tomahawk-03", "10.0.4.23", "Tomahawk", 0, Printing, Some((3, 0.91)), "1.4.2", (13, 13), ok, 1),
        ("Tomahawk-04", "10.0.4.24", "Tomahawk", 0, Printing, Some((5, 0.17)), "1.4.1", (13, 13), ok, 3),
        ("Tomahawk-05", "10.0.4.25", "Tomahawk", 0, Printing, Some((0, 0.63)), "1.4.2", (13, 13), ok, 3),
        ("Tomahawk-06", "10.0.4.26", "Tomahawk", 1, Printing, Some((2, 0.31)), "1.4.1", (13, 13), ok, 3),
        ("Tomahawk-07", "10.0.4.27", "Tomahawk", 1, Printing, Some((1, 0.55)), "1.4.1", (13, 13), ok, 2),
        ("Tomahawk-08", "10.0.4.28", "Tomahawk", 1, Printing, Some((3, 0.08)), "1.4.2", (13, 13), ok, 3),
        ("Tomahawk-09", "10.0.4.29", "Tomahawk", 1, Printing, Some((0, 0.05)), "1.4.2", (13, 13), ok, 3),
        ("Tomahawk-10", "10.0.4.30", "Tomahawk", 1, Printing, Some((5, 0.48)), "1.4.1", (13, 13), ok, 2),
        ("Tomahawk-11", "10.0.4.31", "Tomahawk", 0, Idle, None, "1.4.2", (13, 13), ok, 3),
        ("Tomahawk-12", "10.0.4.32", "Tomahawk", 0, Idle, None, "1.4.2", (13, 13), ok, 3),
        ("Tomahawk-13", "10.0.4.33", "Tomahawk", 1, Idle, None, "1.4.1", (13, 13), ok, 3),
        ("Tomahawk-14", "10.0.4.34", "Tomahawk", 1, Idle, None, "1.4.1", (13, 13), ok, 3),
        ("Bench-A", "10.0.9.2", "Dev bench", 2, Idle, None, "1.5.0-rc1", (6, 6), ok, 0),
        ("Bench-B", "10.0.9.3", "Dev bench", 2, Idle, None, "1.5.0-rc1", (4, 6), Status::Warn, 0),
    ];
    rows.iter().enumerate().map(|(i, r)| machine(i as u32, r.0.into(), r.1.into(), if r.2 == "Tomahawk" { "Tomahawk" } else { "Dev bench" }, r.3, r.4, r.5, r.6, r.7, r.8, r.9)).collect()
}

fn fmt_ts(t: f64) -> String {
    let s = (t + 4360.0) as u32;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

impl Model {
    // ---- active machine <-> fleet record -------------------------------------------------

    pub fn save_active(&mut self) {
        let a = self.active;
        let f = &mut self.fleet[a];
        f.state = self.state;
        f.file = self.job.file.clone();
        f.prog = self.job.progress;
        f.layer = self.job.layer;
        f.layers = self.job.layers;
        f.elapsed = self.job.elapsed;
        f.total = self.job.total;
        for k in 0..3 {
            f.temps[k] = self.heaters[k].cur;
            f.targets[k] = self.heaters[k].target;
        }
    }

    pub fn load_active(&mut self) {
        let f = &self.fleet[self.active];
        self.name = f.name.clone();
        self.state = f.state;
        self.job.file = f.file.clone();
        self.job.progress = f.prog;
        self.job.layer = f.layer;
        self.job.layers = f.layers.max(1);
        self.job.elapsed = f.elapsed;
        self.job.total = f.total.max(1.0);
        for k in 0..3 {
            self.heaters[k].cur = f.temps[k];
            self.heaters[k].target = f.targets[k];
        }
        self.clear_needed = f.state == MState::Fault;
        let h = f.health;
        if let Some(n) = self.nodes.iter_mut().find(|n| n.class == "CHAMBER_IO") {
            n.status = if h == Status::Ok { Status::Ok } else { Status::Warn };
            n.note = if h == Status::Ok { "" } else { "Door sensor open" };
        }
    }

    pub fn switch_machine(&mut self, i: usize) {
        if i >= self.fleet.len() || i == self.active {
            return;
        }
        self.save_active();
        self.active = i;
        self.load_active();
    }

    /// Apply a change to machine `i`, keeping the active `Model` in step.
    pub fn with_machine(&mut self, i: usize, f: impl FnOnce(&mut FleetMachine)) {
        if i >= self.fleet.len() {
            return;
        }
        if i == self.active {
            self.save_active();
        }
        f(&mut self.fleet[i]);
        if i == self.active {
            self.load_active();
        }
    }

    pub fn m_pause(&mut self, i: usize) {
        self.with_machine(i, |f| if f.state == MState::Printing { f.state = MState::Paused });
    }
    pub fn m_resume(&mut self, i: usize) {
        self.with_machine(i, |f| if f.state == MState::Paused { f.state = MState::Printing });
    }
    pub fn m_cancel(&mut self, i: usize) {
        self.release_job(i);
        self.with_machine(i, |f| {
            if matches!(f.state, MState::Printing | MState::Paused) {
                f.state = MState::Idle;
                f.file.clear();
                f.prog = 0.0;
                f.targets = [0.0; 3];
            }
        });
    }
    pub fn m_cool(&mut self, i: usize) {
        self.with_machine(i, |f| f.targets = [0.0; 3]);
    }
    pub fn m_clear(&mut self, i: usize) {
        self.with_machine(i, |f| {
            if f.state == MState::Fault {
                f.state = MState::Idle;
                f.targets = [0.0; 3];
            }
            f.health = Status::Ok;
        });
        let n = self.fleet[i].name.clone();
        self.log(Sev::Info, "farm", &format!("{}: fault cleared by host command", n));
    }
    pub fn m_start(&mut self, i: usize, job: usize) {
        let (file, mat, mins, _) = JOBS[job];
        self.with_machine(i, |f| {
            f.state = MState::Printing;
            f.file = file.into();
            f.material = mat;
            f.prog = 0.0;
            f.elapsed = 0.0;
            f.total = mins as f32 * 60.0;
            f.layers = (mins as f32 / 1.6).max(1.0) as u32;
            f.layer = 0;
            f.targets = mat_temps(mat);
        });
    }
    fn release_job(&mut self, i: usize) {
        for q in self.queue.iter_mut() {
            if q.running == Some(i) {
                q.running = None;
            }
        }
    }

    pub fn alert(&mut self, machine: usize, sev: Sev, msg: &str) {
        if machine == self.active {
            let l = match sev { Sev::Err => format!("error: {}", msg), Sev::Warn => format!("warning: {}", msg), Sev::Info => format!("// {}", msg) };
            self.st.console.push(l);
        }
        let ts = fmt_ts(self.t);
        self.alerts.push(FAlert { ts, machine, sev, msg: msg.into(), ack: false });
        if self.alerts.len() > 120 {
            self.alerts.remove(0);
        }
    }

    pub fn pair(&mut self, d: Disc) {
        let idx = self.fleet.len() as u32;
        let m = machine(idx, d.name.clone(), d.ip, if d.name.starts_with("Bench") { "Dev bench" } else { "Tomahawk" }, if d.name.starts_with("Bench") { 2 } else { 1 }, MState::Idle, None, "1.4.1", (13, 13), Status::Ok, if d.name.starts_with("Bench") { 0 } else { 3 });
        self.fleet.push(m);
        self.log(Sev::Info, "farm", &format!("Paired {} (per-machine key issued)", d.name));
    }

    pub fn unpair(&mut self, i: usize) {
        if i == self.active || i >= self.fleet.len() {
            return;
        }
        let name = self.fleet.remove(i).name;
        let fix = |o: &mut Option<usize>| match *o {
            Some(x) if x == i => *o = None,
            Some(x) if x > i => *o = Some(x - 1),
            _ => {}
        };
        for q in self.queue.iter_mut() {
            fix(&mut q.target);
            fix(&mut q.running);
        }
        for s in self.fspools.iter_mut() {
            match s.loc {
                Loc::Machine(x) if x == i => { s.loc = Loc::Shelf; s.slot = 0; s.history.push(format!("Returned to shelf: {} removed from the farm", name)); }
                Loc::Machine(x) if x > i => s.loc = Loc::Machine(x - 1),
                _ => {}
            }
        }
        self.alerts.retain(|a| a.machine != i);
        for a in self.alerts.iter_mut() {
            if a.machine > i {
                a.machine -= 1;
            }
        }
        if self.active > i {
            self.active -= 1;
        }
        self.log(Sev::Warn, "farm", &format!("Unpaired {}", name));
    }

    // ---- simulation -----------------------------------------------------------------------

    pub fn farm_tick(&mut self, dt: f64) {
        if self.fleet.is_empty() {
            return;
        }
        self.save_active();
        let d = dt as f32;
        let active = self.active;
        for i in 0..self.fleet.len() {
            let f = &mut self.fleet[i];
            if i != active {
                if f.state == MState::Printing {
                    f.prog = (f.prog + d * SPEEDUP / f.total.max(1.0)).min(1.0);
                    f.elapsed += d * SPEEDUP;
                    f.layer = (f.prog * f.layers as f32) as u32;
                }
                for k in 0..3 {
                    let tgt = if f.targets[k] > 0.0 { f.targets[k] } else { 24.0 };
                    let rate = if f.targets[k] > 0.0 { 0.8 } else { 0.05 };
                    f.temps[k] += (tgt - f.temps[k]) * rate * d;
                }
            }
            if f.state == MState::Printing {
                f.hours += d * SPEEDUP / 3600.0;
            }
        }
        // completions (active machine included)
        for i in 0..self.fleet.len() {
            if self.fleet[i].state == MState::Printing && self.fleet[i].prog >= 1.0 {
                self.finish_job(i);
            }
        }
        self.poll_prints();
        self.farm_t += dt;
        if self.farm_t >= 1.0 {
            self.farm_t -= 1.0;
            self.farm_second();
        }
        self.rollout_tick(d);
    }

    fn finish_job(&mut self, i: usize) {
        let (name, file) = (self.fleet[i].name.clone(), self.fleet[i].file.clone());
        self.with_machine(i, |f| {
            f.state = MState::Idle;
            f.jobs_done += 1;
            f.targets = [0.0; 3];
            f.grams += 120.0;
        });
        let mut line = format!("{}  {}  complete", name, file);
        for q in self.queue.iter_mut() {
            if q.running == Some(i) {
                q.running = None;
                q.left = q.left.saturating_sub(1);
                line = format!("{}  {}  complete (queue #{})", name, file, q.id);
            }
        }
        self.queue.retain(|q| q.left > 0);
        self.done_log.push(line);
        if self.done_log.len() > 12 {
            self.done_log.remove(0);
        }
        if let Some(si) = self.loaded_with(i, self.fleet[i].material) {
            let (id, g) = (self.fspools[si].id, JOBS.iter().find(|j| j.0 == file).map(|j| j.3).unwrap_or(0.0));
            self.spool_log(id, &format!("Used {:.0} g printing {} on {}", g, file, name));
        }
        let (mat, secs) = (self.fleet[i].material, self.fleet[i].total);
        let grams = JOBS.iter().find(|j| j.0 == file).map(|j| j.3).unwrap_or(0.0);
        let ended = format!("2026-10-30 {}", &fmt_ts(self.t)[..5]);
        self.history.push(HistEntry { file: file.clone(), mat, color: crate::pages::print_file_color(&file), status: 0, secs, grams, ended, machine: i });
        self.alert(i, Sev::Info, &format!("Print complete: {}", file));
    }

    fn farm_second(&mut self) {
        // spool consumption: each printing machine draws from the loaded spool of its material
        let mut low = Vec::new();
        let mut dry = Vec::new();
        for i in 0..self.fleet.len() {
            if self.fleet[i].state != MState::Printing {
                continue;
            }
            let (mat, total, file) = (self.fleet[i].material, self.fleet[i].total.max(1.0), self.fleet[i].file.clone());
            let grams = JOBS.iter().find(|j| j.0 == file).map(|j| j.3).unwrap_or(100.0);
            let g = grams * SPEEDUP / total;
            match self.loaded_with(i, mat) {
                Some(si) => {
                    let s = &mut self.fspools[si];
                    let prev = s.left;
                    s.left = (s.left - g).max(0.0);
                    s.used += g;
                    if prev >= self.low_thr && s.left < self.low_thr { low.push((i, s.left, s.mat.clone(), s.id)); }
                    if prev > 5.0 && s.left <= 5.0 { dry.push((i, s.mat.clone(), s.id)); }
                }
                None => {}
            }
        }
        for (i, g, m, id) in low {
            self.alert(i, Sev::Warn, &format!("Spool low: {} {} {:.0} g left", crate::spools::code(id), m, g));
            self.spool_log(id, &format!("Low: {:.0} g remaining", g));
        }
        for (i, m, id) in dry {
            self.alert(i, Sev::Err, &format!("Spool empty: {} {} - swap spool", crate::spools::code(id), m));
            self.spool_log(id, "Ran empty during a print");
            self.m_pause(i);
        }
        for f in self.fleet.iter_mut() {
            if f.state == MState::Printing {
                f.grams += 0.05 * SPEEDUP;
            }
        }
        // histories
        let n = self.fleet.iter().filter(|f| f.state == MState::Printing).count() as f32;
        self.h_farm_n.push_back(n);
        let jitter = 0.9 + 0.2 * self.rand();
        self.h_farm_g.push_back(n * 0.05 * 3600.0 * jitter);
        while self.h_farm_n.len() > HIST {
            self.h_farm_n.pop_front();
            self.h_farm_g.pop_front();
        }
        self.dispatch();
        if self.t >= self.next_event {
            self.random_event();
            self.next_event = self.t + 14.0 + self.rand() as f64 * 22.0;
        }
    }

    /// A machine can take a job only if it has a spool of the right material with enough left.
    pub fn has_material(&self, machine: usize, mat: &str, file: &str) -> bool {
        let need = JOBS.iter().find(|j| j.0 == file).map(|j| j.3).unwrap_or(100.0);
        self.loaded(machine).iter().any(|&i| self.fspools[i].mat == mat && self.fspools[i].left >= need * 0.9)
    }

    fn dispatch(&mut self) {
        if !self.auto_dispatch {
            return;
        }
        for i in 0..self.fleet.len() {
            if self.fleet[i].state != MState::Idle || self.fleet[i].health == Status::Err || self.fleet[i].group == 2 {
                continue;
            }
            let pick = self
                .queue
                .iter()
                .enumerate()
                .filter(|(_, q)| q.running.is_none() && q.left > 0 && q.target.map_or(true, |t| t == i) && self.has_material(i, q.material, q.file))
                .max_by_key(|(_, q)| (q.prio, std::cmp::Reverse(q.id)))
                .map(|(k, _)| k);
            if let Some(k) = pick {
                let file = self.queue[k].file;
                let job = JOBS.iter().position(|j| j.0 == file).unwrap_or(0);
                self.queue[k].running = Some(i);
                self.m_start(i, job);
                let (n, id) = (self.fleet[i].name.clone(), self.queue[k].id);
                self.log(Sev::Info, "farm", &format!("Queue #{} dispatched to {}", id, n));
            }
        }
    }

    fn random_event(&mut self) {
        let i = (self.rand() * self.fleet.len() as f32) as usize % self.fleet.len();
        let r = self.rand();
        let name = self.fleet[i].name.clone();
        match self.fleet[i].state {
            MState::Printing => {
                if r < 0.30 {
                    let score = 0.7 + self.rand() * 0.28;
                    self.alert(i, Sev::Err, &format!("Failure detected (spaghetti, score {:.2}){}", score, if self.auto_pause { " - print paused via API" } else { "" }));
                    if self.auto_pause {
                        self.m_pause(i);
                    }
                } else if r < 0.5 {
                    self.alert(i, Sev::Warn, "Chamber door opened");
                } else if r < 0.7 {
                    self.alert(i, Sev::Warn, "Hotend deviation 6 C, recovered");
                } else if r < 0.85 {
                    self.alert(i, Sev::Warn, "Following error spike on Drive Z2, recovered");
                } else if r < 0.93 {
                    self.alert(i, Sev::Err, "Over-current on Drive Y: fault latched, print stopped");
                    self.with_machine(i, |f| {
                        f.state = MState::Fault;
                        f.health = Status::Err;
                        f.targets = [0.0; 3];
                    });
                    self.release_job(i);
                } else {
                    self.alert(i, Sev::Warn, "MMU heartbeat missed (1 s), restored");
                }
            }
            MState::Paused => {
                if r < 0.4 {
                    self.m_resume(i);
                    self.alert(i, Sev::Info, "Operator-less resume after door closed");
                }
            }
            _ => self.alert(i, Sev::Info, "Filter service due in 40 h"),
        }
        let _ = name;
    }

    fn rollout_tick(&mut self, d: f32) {
        if !self.rollout.running {
            return;
        }
        self.rollout.prog += d * 0.08;
        if self.rollout.prog < 1.0 {
            return;
        }
        self.rollout.prog = 0.0;
        let w = self.rollout.wave as u8 + 1;
        let mut n = 0;
        for k in 0..self.fleet.len() {
            if self.fleet[k].wave == w {
                self.fleet[k].fw = TARGET_FW.into();
                n += 1;
            }
        }
        self.log(Sev::Info, "fleet", &format!("Roll-out wave {} complete: {} machine(s) on {}", w, n, TARGET_FW));
        self.rollout.wave += 1;
        if self.rollout.wave >= 3 {
            self.rollout.running = false;
            self.rollout.done = true;
        } else {
            self.rollout_gate();
        }
    }

    /// Health gate before a wave: refuse to continue if a machine in it is faulted.
    pub fn rollout_gate(&mut self) {
        let w = self.rollout.wave as u8 + 1;
        let bad = self.fleet.iter().find(|f| f.wave == w && f.state == MState::Fault).map(|f| f.name.clone());
        self.rollout.halted = bad.clone();
        if let Some(n) = bad {
            self.rollout.running = false;
            self.log(Sev::Warn, "fleet", &format!("Roll-out halted before wave {}: {} is faulted", w, n));
        }
    }

    /// Seed queue, spool stock and alert history so every farm tab has content.
    pub fn seed_farm(&mut self) {
        let brands = ["Polymaker", "Prusament", "Elegoo", "eSun", "Sunlu", "Fiberon", "Bambu Lab", "Overture"];
        let colors = ["Black", "White", "Grey", "Red", "Natural", "Galaxy Black", "Blue", "Orange"];
        let mats_cycle = ["PLA", "PETG", "ASA", "PA-CF"];
        let nm = self.fleet.len();
        let make = |me: &mut Model, mat: &str, loc: Loc, slot: u8, shelf: &str, k: u32| {
            let id = me.next_spool;
            me.next_spool += 1;
            let net = if mat == "PA-CF" { 750.0 } else { 1000.0 };
            let left = if k % 11 == 3 { 92.0 } else { 140.0 + hs(id, 11) * (net - 140.0) };
            let loaded = matches!(loc, Loc::Machine(_));
            let day = 1 + (id % 27);
            let hyg = crate::spools::hygroscopic(mat);
            me.fspools.push(FSpool {
                id, brand: brands[(id as usize) % 8].into(), mat: mat.into(), color: colors[(id as usize * 3) % 8].into(),
                dia: 1.75, net, tare: 230.0, left, cost: if mat == "PA-CF" { 54.0 } else { 21.0 + (id % 5) as f32 * 3.0 },
                supplier: ["Filament Direct", "Amazon", "Manufacturer"][(id % 3) as usize].into(), lot: format!("L{}-{:03}", 2400 + id % 9, id * 7 % 1000),
                bought: format!("2026-{:02}-{:02}", 7 + id % 3, day), dried: if hyg && id % 4 != 0 { format!("2026-10-{:02}", 1 + id % 20) } else if hyg { "2026-08-12".into() } else { String::new() },
                shelf: shelf.into(), loc, slot, notes: String::new(), printed: id % 7 != 0, used: net - left,
                history: vec![format!("2026-10-{:02} 09:00:00  Created via entry workflow ({:.0} g net)", day, net)],
            });
            let _ = loaded;
        };
        let mut k = 0;
        for i in 0..nm {
            if self.fleet[i].group == 2 {
                continue;
            }
            let first = if self.fleet[i].file.is_empty() { mats_cycle[(i + 1) % 4] } else { self.fleet[i].material };
            let second = mats_cycle[(i + 2) % 4];
            make(self, first, Loc::Machine(i), 0, "A1", k);
            make(self, second, Loc::Machine(i), 1, "A1", k + 1);
            k += 2;
        }
        for j in 0..14 {
            let mat = ["PLA", "PETG", "ASA", "PA-CF", "TPU", "PLA", "PETG"][j % 7];
            let (loc, shelf) = if j % 7 == 6 { (Loc::Dryer, "Dry box") } else { (Loc::Shelf, SHELF[j % 5]) };
            make(self, mat, loc, 0, shelf, k + j as u32);
        }
        make(self, "PA12-CF", Loc::Archived, 0, "C1", 99);
        let aid = self.next_spool - 1;
        self.fspools.last_mut().unwrap().left = 0.0;
        let _ = aid;
        let ids: Vec<u32> = self.fspools.iter().map(|s| s.id).collect();
        for id in ids {
            let l = self.fspools[self.spool_idx(id).unwrap()].loc.clone();
            if let Loc::Machine(m) = l {
                let n = self.mname(m);
                let slot = self.fspools[self.spool_idx(id).unwrap()].slot;
                self.spool_log(id, &format!("Loaded on {} slot T{}", n, slot));
            }
        }
        const SHELF: [&str; 5] = ["A1", "A2", "B1", "B2", "C1"];
        for (f, copies, prio) in [(3usize, 4u32, 1usize), (1, 6, 0), (5, 2, 1), (2, 2, 2), (4, 1, 2), (0, 1, 1)] {
            let (file, material, mins, _) = JOBS[f];
            self.queue.push(QJob { id: self.next_qid, file, material, mins, left: copies, prio, target: None, running: None });
            self.next_qid += 1;
        }
        for (i, sev, msg) in [
            (3usize, Sev::Err, "Failure detected (spaghetti, score 0.82) - print paused via API"),
            (5, Sev::Warn, "Chamber door opened"),
            (7, Sev::Warn, "Spool low: PLA 118 g left"),
            (1, Sev::Warn, "Hotend deviation 5 C, recovered"),
            (9, Sev::Info, "Print complete: pa_calib_tower.gcode"),
            (15, Sev::Warn, "Node MMU offline (bench rig)"),
        ] {
            self.alert(i, sev, msg);
        }
        self.alerts.iter_mut().skip(4).for_each(|a| a.ack = true);
        let names = ["housing_set_v6.gcode", "pa_calib_tower.gcode", "benchy_calib.gcode", "enclosure_panel_left.gcode", "gear_train_x6.gcode", "mmu_test_4color.gcode", "housing_set_v7.gcode"];
        let mats = ["PA-CF", "PETG", "PETG", "ASA", "PLA", "PLA", "PA-CF"];
        let secs = [27120.0, 1080.0, 3720.0, 19080.0, 12300.0, 3120.0, 27600.0];
        let grams = [308.0, 6.0, 14.0, 186.0, 98.0, 22.0, 312.0];
        for mi in 0..self.fleet.len() {
            for k in 0..9usize {
                let j = (mi * 3 + k * 5) % names.len();
                let st = if (mi + k * 3) % 11 == 0 { 2 } else if (mi + k * 2) % 7 == 0 { 1 } else { 0 };
                let part = if st == 0 { 1.0 } else { 0.25 + hs(mi as u32 * 17 + k as u32, 4) * 0.5 };
                self.history.push(HistEntry {
                    file: names[j].to_string(), mat: mats[j], color: crate::pages::print_file_color(names[j]),
                    status: st, secs: secs[j] * part, grams: grams[j] * part,
                    ended: format!("2026-10-{:02} {:02}:{:02}", 29 - k as u32 * 2 % 27, 6 + (k * 3 + mi) % 16, (k * 13 + mi * 7) % 60),
                    machine: mi,
                });
            }
        }
        self.done_log.push("Tomahawk-09  pa_calib_tower.gcode  complete".into());
        self.done_log.push("Tomahawk-02  benchy_calib.gcode  complete".into());
    }
}
