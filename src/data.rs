#![allow(clippy::too_many_arguments)]
//! Mock machine model. Everything here is placeholder data that a real
//! build would receive through the signed API.
use crate::theme::Status;
use std::collections::VecDeque;

pub const HIST: usize = 240;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Page {
    Dashboard, Print, Thermal,
    Motion, Calibration, Probe, Drives,
    Materials, Spools, Camera,
    CanBus, Nodes, Firmware,
    Builder, Config, Rules, Api, Fleet, Simulator, Spec, Logs,
}

impl Page {
    /// Sidebar entries (12). A few of them stack closely related pages as sections.
    pub const GROUPS: &'static [(&'static str, &'static [Page])] = &[
        ("Machine", &[Page::Dashboard, Page::Print, Page::Camera]),
        ("Tuning", &[Page::Motion, Page::Calibration]),
        ("Materials", &[Page::Spools]),
        ("Hardware", &[Page::CanBus, Page::Firmware]),
        ("System", &[Page::Fleet, Page::Builder, Page::Api, Page::Logs]),
    ];

    /// The sidebar entry this page lives under.
    pub fn hub(self) -> Page {
        match self {
            Page::Thermal | Page::Drives => Page::Motion,
            Page::Probe => Page::Calibration,
            Page::Materials => Page::Spools,
            Page::Nodes => Page::CanBus,
            Page::Config | Page::Rules => Page::Builder,
            Page::Simulator | Page::Spec => Page::Api,
            p => p,
        }
    }

    /// Sections stacked on a merged page: (page, heading). Empty for single-section pages.
    pub fn tabs(self) -> &'static [(Page, &'static str)] {
        match self {
            Page::Motion => &[(Page::Motion, "Motion"), (Page::Thermal, "Heaters"), (Page::Drives, "Drives")],
            Page::Calibration => &[(Page::Calibration, "Calibration"), (Page::Probe, "Eddy probe")],
            Page::Spools => &[(Page::Spools, "Spool inventory"), (Page::Materials, "MMU and profiles")],
            Page::CanBus => &[(Page::CanBus, "CAN FD bus"), (Page::Nodes, "Nodes")],
            Page::Builder => &[(Page::Builder, "Config builder"), (Page::Config, "Typed editor"), (Page::Rules, "Rules")],
            Page::Api => &[(Page::Api, "API and security"), (Page::Simulator, "Simulator"), (Page::Spec, "Spec and roadmap")],
            _ => &[],
        }
    }

    /// Name shown in the sidebar and top bar.
    pub fn nav_title(self) -> &'static str {
        match self {
            Page::Motion => "Tuning",
            Page::Spools => "Filament",
            Page::CanBus => "Hardware",
            Page::Builder => "Config",
            Page::Api => "System",
            p => p.title(),
        }
    }

    pub fn hub_blurb(self) -> &'static str {
        match self {
            Page::Motion => "Planner, input shaping, pressure advance, heater control and drives",
            Page::Calibration => "Auto-calibration, mesh, leveling, resonance and the eddy probe",
            Page::Spools => "Shared spool database, labels and tracking, MMU and filament profiles",
            Page::CanBus => "Two CAN FD segments and every node on them",
            Page::Builder => "Block-based machine config, the typed editor and the rules engine",
            Page::Api => "Clients and keys, simulator and the spec",
            _ => "",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Page::Dashboard => "Dashboard", Page::Print => "Print",
            Page::Thermal => "Thermal", Page::Motion => "Motion", Page::Calibration => "Calibration",
            Page::Probe => "Eddy Probe", Page::Drives => "Drives", Page::Materials => "Filament", Page::Spools => "Spool Inventory",
            Page::Camera => "Camera", Page::CanBus => "CAN FD Bus", Page::Nodes => "Nodes",
            Page::Firmware => "Firmware", Page::Builder => "Config Builder", Page::Config => "Config", Page::Rules => "Rules",
            Page::Api => "API & Security", Page::Fleet => "Fleet", Page::Simulator => "Simulator",
            Page::Spec => "Spec & Roadmap", Page::Logs => "Event Log",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Page::Dashboard => "01", Page::Print => "03", Page::Thermal => "04",
            Page::Motion => "05", Page::Calibration => "06", Page::Probe => "07", Page::Drives => "08",
            Page::Materials => "09", Page::Camera => "10", Page::CanBus => "11", Page::Nodes => "12",
            Page::Firmware => "13", Page::Fleet => "14", Page::Config => "15", Page::Rules => "16", Page::Api => "17",
            Page::Simulator => "18", Page::Logs => "19", Page::Spec => "20", Page::Builder => "21", Page::Spools => "22",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Seg { A, B, Wifi }
impl Seg {
    pub fn label(self) -> &'static str {
        match self { Seg::A => "CAN FD A", Seg::B => "CAN FD B", Seg::Wifi => "WiFi" }
    }
}

#[derive(Clone)]
pub struct Node {
    pub name: &'static str,
    pub class: &'static str,
    pub mcu: &'static str,
    pub role: &'static str,
    pub uid: u64,
    pub addr: u8,
    pub seg: Seg,
    pub fw: &'static str,
    pub want: &'static str,
    pub bank: char,
    pub status: Status,
    pub hb: f32,
    pub temp: f32,
    pub rated: u16,
    pub tx_err: u32,
    pub rx_err: u32,
    pub proposed: bool,
    pub note: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MState { Idle, Printing, Paused, Calibrating, Fault }
impl MState {
    pub fn label(self) -> &'static str {
        match self {
            MState::Idle => "IDLE", MState::Printing => "PRINTING", MState::Paused => "PAUSED",
            MState::Calibrating => "CALIBRATING", MState::Fault => "FAULT LATCHED",
        }
    }
    pub fn status(self) -> Status {
        match self {
            MState::Idle => Status::Idle, MState::Printing => Status::Ok, MState::Paused => Status::Warn,
            MState::Calibrating => Status::Ok, MState::Fault => Status::Err,
        }
    }
}

#[derive(Clone)]
pub struct Obj { pub id: usize, pub name: &'static str, pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub excluded: bool }

pub struct Job {
    pub file: String,
    pub progress: f32,
    pub layer: u32,
    pub layers: u32,
    pub elapsed: f32,
    pub total: f32,
    pub speed: f32,
    pub flow: f32,
    pub objects: Vec<Obj>,
}

pub struct Heater {
    pub name: &'static str, pub node: &'static str, pub cur: f32, pub target: f32, pub power: f32,
    pub max: f32, pub ctl: usize, pub wd_ms: u32,
}
pub struct Fan { pub name: &'static str, pub node: &'static str, pub pct: f32, pub rpm_max: f32 }

pub struct Drive {
    pub axis: &'static str, pub node: &'static str, pub motor: &'static str, pub status: Status,
    pub cur: f32, pub follow: f32, pub enc: &'static str, pub temp: f32, pub vbus: f32,
    pub mode: usize, pub r: f32, pub l: f32, pub kt: f32, pub poles: u32, pub tuned: &'static str,
    pub irms: f32,
}

pub struct FileEntry { pub name: &'static str, pub size_mb: f32, pub mins: u32, pub grams: f32, pub objects: u32, pub mat: &'static str, pub slicer: &'static str }

pub struct Profile { pub name: &'static str, pub nozzle: f32, pub bed: f32, pub chamber: f32, pub pa: f32, pub flow: f32, pub fan: f32 }

#[derive(Clone)]
pub struct FleetMachine {
    pub name: String, pub model: &'static str, pub ip: String, pub group: usize,
    pub state: MState, pub file: String, pub material: &'static str,
    pub prog: f32, pub layer: u32, pub layers: u32, pub elapsed: f32, pub total: f32,
    pub temps: [f32; 3], pub targets: [f32; 3],
    pub fw: String, pub nodes: (u8, u8), pub sel: bool, pub health: Status,
    pub fp: String, pub key_age: u32, pub wave: u8,
    pub jobs_done: u32, pub hours: f32, pub grams: f32, pub fails: u32, pub util: [f32; 7],
    pub nozzle: f32, pub slots: u8, pub notes: String,
}
pub struct QJob { pub id: u32, pub file: &'static str, pub material: &'static str, pub mins: u32, pub left: u32, pub prio: usize, pub target: Option<usize>, pub running: Option<usize> }
pub struct FAlert { pub ts: String, pub machine: usize, pub sev: Sev, pub msg: String, pub ack: bool }
#[derive(Clone, PartialEq)]
pub enum Loc { Shelf, Dryer, Machine(usize), Archived }

#[derive(Clone)]
pub struct FSpool {
    pub id: u32, pub brand: String, pub mat: String, pub color: String, pub dia: f32,
    pub net: f32, pub tare: f32, pub left: f32, pub cost: f32,
    pub supplier: String, pub lot: String, pub bought: String, pub dried: String,
    pub shelf: String, pub loc: Loc, pub slot: u8, pub notes: String,
    pub printed: bool, pub used: f32, pub history: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Conn { Network, Usb, Bluetooth }
#[derive(Clone, Copy, PartialEq)]
pub enum Lang { Zpl, Tspl, EscPos, Cpcl }
pub struct Printer { pub name: String, pub conn: Conn, pub lang: Lang, pub addr: String, pub demo: bool }
pub struct LabelJob { pub ts: String, pub codes: Vec<u32>, pub copies: u32, pub printer: String, pub status: String }

pub struct LabelCfg {
    pub printer: usize, pub size: usize, pub layout: usize, pub copies: u32,
    pub show_brand: bool, pub show_dia: bool, pub show_lot: bool, pub show_weight: bool,
    pub sel: Vec<u32>, pub msg: String,
    pub np_name: String, pub np_addr: String, pub np_conn: usize, pub np_lang: usize,
}

#[derive(Clone)]
pub struct SpoolEntry {
    pub step: usize, pub source: usize, pub gtin: String,
    pub brand: String, pub mat: usize, pub color: String, pub dia: usize,
    pub net: f32, pub tare: f32, pub cost: f32, pub supplier: String, pub lot: String,
    pub bought: String, pub shelf: String, pub qty: u32, pub notes: String,
    pub print_after: bool, pub created: Vec<u32>,
}
pub struct Disc { pub name: String, pub ip: String }
pub struct Rollout { pub wave: usize, pub prog: f32, pub running: bool, pub halted: Option<String>, pub done: bool }
pub struct Client { pub name: &'static str, pub kind: &'static str, pub fp: &'static str, pub scope: &'static str, pub seen: &'static str, pub revoked: bool, pub age: u32 }
pub struct Rule { pub name: &'static str, pub on: bool, pub trigger: usize, pub cond: String, pub actions: Vec<String>, pub runs: u32 }

#[derive(Clone, PartialEq)]
pub enum Val { F(f64), I(i64), B(bool), S(String), E(usize, &'static [&'static str]) }
pub struct Field { pub key: &'static str, pub val: Val, pub orig: Val, pub unit: &'static str, pub min: f64, pub max: f64, pub desc: &'static str }
pub struct Section { pub name: &'static str, pub fields: Vec<Field> }

#[derive(Clone, Copy, PartialEq)]
pub enum Sev { Info, Warn, Err }
pub struct Log { pub ts: String, pub sev: Sev, pub src: String, pub msg: String }
pub struct Frame { pub t: f64, pub seg: Seg, pub id: u32, pub class: &'static str, pub node: &'static str, pub data: String }

pub struct Shaper { pub axis: &'static str, pub kind: usize, pub freq: f32, pub damp: f32, pub max_accel: f32, pub when: &'static str }

pub struct Cal { pub running: bool, pub step: usize, pub prog: f32 }
pub struct FwRun { pub running: bool, pub prog: f32, pub idx: usize }
pub struct Sim { pub running: bool, pub prog: f32, pub results: Vec<(&'static str, f32, f32, f32)>, pub faults: [bool; 4], pub log: Vec<String> }

#[derive(Default)]
pub struct St {
    pub step: usize,
    pub console: Vec<String>,
    pub cmd: String,
    pub sel_file: usize,
    pub sel_node: usize,
    pub sel_rule: usize,
    pub sel_sec: usize,
    pub tab: usize,
    pub tab2: usize,
    pub log_filter: usize,
    pub log_q: String,
    pub frames_paused: bool,
    pub new_client: String,
    pub extr_len: f32,
    pub pa_lo: f32,
    pub pa_hi: f32,
    pub filter_node: usize,
    pub pa_profile: usize,
    pub pa_done: bool,
    pub res_done: bool,
    pub probe_ok: bool,
    pub sel_drive: usize,
    pub sel_fleet: usize,
    pub rel_mode: bool,
    pub con_filter: usize,
    pub hist: Vec<String>,
    pub hist_idx: usize,
    pub con_t: f64,
    pub sp_q: String,
    pub sp_mat: usize,
    pub sp_loc: usize,
    pub sp_low: bool,
    pub sel_spool_id: u32,
    pub scan_text: String,
    pub scan_msg: String,
    pub weigh_gross: f32,
    pub add_open: bool,
    pub add_name: String,
    pub add_ip: String,
    pub add_model: usize,
    pub add_group: usize,
    pub confirm_rm: Option<usize>,
    pub led: f32,
    pub farm_tab: usize,
    pub q_file: usize,
    pub q_copies: u32,
    pub q_prio: usize,
    pub q_target: usize,
    pub grp_filter: usize,
    pub al_filter: usize,
    pub send_file: usize,
    pub set_group: usize,
    pub only_printing: bool,
}

pub struct Model {
    pub t: f64,
    acc: f64,
    rng: u64,
    pub goto: Option<Page>,
    pub name: String,
    pub state: MState,
    pub pos: [f32; 4],
    pub home: [bool; 3],
    pub job: Job,
    pub heaters: Vec<Heater>,
    pub fans: Vec<Fan>,
    pub drives: Vec<Drive>,
    pub nodes: Vec<Node>,
    pub files: Vec<FileEntry>,
    pub profiles: Vec<Profile>,
    pub fleet: Vec<FleetMachine>,
    pub clients: Vec<Client>,
    pub rules: Vec<Rule>,
    pub config: Vec<Section>,
    pub logs: Vec<Log>,
    pub frames: VecDeque<Frame>,
    pub shapers: Vec<Shaper>,
    pub cal: Cal,
    pub fw: FwRun,
    pub sim: Sim,
    pub st: St,
    pub h_hot: VecDeque<f32>,
    pub h_bed: VecDeque<f32>,
    pub h_cham: VecDeque<f32>,
    pub h_t_hot: VecDeque<f32>,
    pub h_can_a: VecDeque<f32>,
    pub h_can_b: VecDeque<f32>,
    pub h_follow: VecDeque<f32>,
    pub h_sync: VecDeque<f32>,
    pub mesh: Vec<Vec<f32>>,
    pub manifest: &'static str,
    pub pa_value: f32,
    pub probe_driver: usize,
    pub probe_comp: bool,
    pub clear_needed: bool,
    pub decisions: [bool; 8],
    pub phases_done: usize,
    pub rollout: Rollout,
    pub bld: crate::builder::Builder,
    pub dash_order: Vec<u8>,
    pub hub_sel: std::collections::HashMap<Page, Page>,
    pub active: usize,
    pub queue: Vec<QJob>,
    pub next_qid: u32,
    pub auto_dispatch: bool,
    pub auto_pause: bool,
    pub alerts: Vec<FAlert>,
    pub fspools: Vec<FSpool>,
    pub next_spool: u32,
    pub printers: Vec<Printer>,
    pub label_jobs: Vec<LabelJob>,
    pub label: LabelCfg,
    pub entry: SpoolEntry,
    pub print_tx: std::sync::mpsc::Sender<String>,
    pub print_rx: std::sync::mpsc::Receiver<String>,
    pub disc: Vec<Disc>,
    pub h_farm_n: VecDeque<f32>,
    pub h_farm_g: VecDeque<f32>,
    pub low_thr: f32,
    pub farm_t: f64,
    pub next_event: f64,
    pub done_log: Vec<String>,
    pub scan_n: u32,
    pub accel: [f32; 3],
    pub jerk: f32,
    pub snap: f32,
    pub vmax: f32,
    pub blend: f32,
    pub pa_smooth: f32,
    pub pa_on: bool,
    pub mesh_adaptive: bool,
    pub purge_adaptive: bool,
    pub z_offset: f32,
    pub det_sens: f32,
    pub det_action: usize,
    pub det_on: bool,
    pub lapse_on: bool,
    pub lapse_every: usize,
    pub mmu_state: usize,
    pub tilt: [f32; 3],
    pub xtwist: f32,
    pub mmu_verbs: Vec<String>,
    pub resume_ready: bool,
}

fn node(name: &'static str, class: &'static str, mcu: &'static str, role: &'static str, uid: u64, addr: u8, seg: Seg, fw: &'static str, status: Status, hb: f32, temp: f32, rated: u16, proposed: bool, note: &'static str) -> Node {
    Node { name, class, mcu, role, uid, addr, seg, fw, want: fw, bank: 'A', status, hb, temp, rated, tx_err: 0, rx_err: 0, proposed, note }
}

fn f(key: &'static str, v: f64, unit: &'static str, min: f64, max: f64, desc: &'static str) -> Field {
    Field { key, val: Val::F(v), orig: Val::F(v), unit, min, max, desc }
}
fn i(key: &'static str, v: i64, unit: &'static str, min: f64, max: f64, desc: &'static str) -> Field {
    Field { key, val: Val::I(v), orig: Val::I(v), unit, min, max, desc }
}
fn b(key: &'static str, v: bool, desc: &'static str) -> Field {
    Field { key, val: Val::B(v), orig: Val::B(v), unit: "", min: 0.0, max: 0.0, desc }
}
fn s(key: &'static str, v: &str, desc: &'static str) -> Field {
    Field { key, val: Val::S(v.into()), orig: Val::S(v.into()), unit: "", min: 0.0, max: 0.0, desc }
}
fn e(key: &'static str, idx: usize, opts: &'static [&'static str], desc: &'static str) -> Field {
    Field { key, val: Val::E(idx, opts), orig: Val::E(idx, opts), unit: "", min: 0.0, max: 0.0, desc }
}

impl Model {
    pub fn new() -> Self {
        let nodes = vec![
            node("Drive X", "FOC_DRIVE", "STM32G431", "Closed-loop drive, X carriage", 0x5A3E_01C4_77D2_0011, 0x10, Seg::A, "9f3a1c42", Status::Ok, 10.0, 41.0, 125, false, ""),
            node("Drive Y", "FOC_DRIVE", "STM32G431", "Closed-loop drive, Y gantry", 0x5A3E_01C4_77D2_0012, 0x11, Seg::A, "9f3a1c42", Status::Ok, 10.0, 43.0, 125, false, ""),
            node("Drive Z1", "FOC_DRIVE", "STM32G431", "Closed-loop drive, Z front-left", 0x5A3E_01C4_77D2_0013, 0x12, Seg::A, "9f3a1c42", Status::Ok, 10.0, 38.0, 125, false, ""),
            node("Drive Z2", "FOC_DRIVE", "STM32G431", "Closed-loop drive, Z front-right", 0x5A3E_01C4_77D2_0014, 0x13, Seg::A, "9f3a1c42", Status::Ok, 10.0, 39.0, 125, false, ""),
            node("Drive Z3", "FOC_DRIVE", "STM32G431", "Closed-loop drive, Z rear", 0x5A3E_01C4_77D2_0015, 0x14, Seg::A, "9f3a1c42", Status::Ok, 10.0, 38.5, 125, false, ""),
            node("Drive E", "FOC_DRIVE", "STM32G431", "Closed-loop drive, extruder", 0x5A3E_01C4_77D2_0016, 0x15, Seg::A, "9f3a1c42", Status::Ok, 10.0, 47.0, 125, false, ""),
            node("Power Board", "POWER", "STM32 (FDCAN)", "48 V / 5 V distribution, e-stop, power-loss detect", 0x5A3E_02A1_0B00_0021, 0x20, Seg::A, "c04e77a1", Status::Ok, 10.0, 36.0, 125, true, ""),
            node("Eddy Probe", "EDDY_PROBE", "STM32 (FDCAN)", "Z sensing, temperature compensation, ADXL345", 0x5A3E_03B0_44E1_0031, 0x30, Seg::B, "d81f0b3e", Status::Ok, 10.0, 52.0, 125, false, ""),
            node("Toolhead", "TOOLHEAD", "STM32 (FDCAN)", "Hotend heater, part/heatsink fans, filament sensor, LED", 0x5A3E_04C2_90AA_0041, 0x31, Seg::B, "a27c5e90", Status::Ok, 10.0, 58.0, 125, true, ""),
            node("Chamber IO", "CHAMBER_IO", "STM32 (FDCAN)", "Chamber heater, exhaust, filter, door sensors", 0x5A3E_05D3_11F0_0051, 0x32, Seg::B, "b6a91d07", Status::Warn, 9.8, 46.0, 125, true, "Door sensor open"),
            node("MMU", "MMU", "STM32 (FDCAN)", "Multi-material feed and spool sensing", 0x5A3E_06E4_7C3B_0061, 0x33, Seg::B, "71be0d95", Status::Ok, 10.0, 33.0, 85, true, "Image differs from manifest"),
            node("Wireless ADXL", "ADXL_WIFI", "ESP32", "Diagnostic resonance capture, ms-level sync", 0x5A3E_07F5_2D8C_0071, 0x40, Seg::Wifi, "e90a44c8", Status::Idle, 0.0, 29.0, 85, false, "Sleeping"),
            node("Camera", "CAMERA", "ESP32", "Video stream and snapshots", 0x5A3E_08A6_6E19_0081, 0x41, Seg::Wifi, "3f55b2aa", Status::Ok, 1.0, 44.0, 85, false, ""),
        ];

        let objects = vec![
            Obj { id: 0, name: "bracket_A", x: 40.0, y: 50.0, w: 70.0, h: 55.0, excluded: false },
            Obj { id: 1, name: "bracket_B", x: 140.0, y: 60.0, w: 70.0, h: 55.0, excluded: false },
            Obj { id: 2, name: "housing", x: 60.0, y: 160.0, w: 90.0, h: 80.0, excluded: false },
            Obj { id: 3, name: "cover", x: 190.0, y: 170.0, w: 85.0, h: 70.0, excluded: false },
            Obj { id: 4, name: "clip_x4", x: 235.0, y: 55.0, w: 45.0, h: 40.0, excluded: false },
        ];

        let mut hot = VecDeque::new();
        let mut bed = VecDeque::new();
        let mut cham = VecDeque::new();
        let mut tgt = VecDeque::new();
        let mut ca = VecDeque::new();
        let mut cb = VecDeque::new();
        let mut fo = VecDeque::new();
        let mut sy = VecDeque::new();
        for k in 0..HIST {
            let x = k as f32;
            hot.push_back(275.0 + (x * 0.7).sin() * 0.4);
            bed.push_back(90.0 + (x * 0.2).sin() * 0.2);
            cham.push_back(52.0 + x * 0.002);
            tgt.push_back(275.0);
            ca.push_back(11.0 + (x * 0.3).sin() * 1.0);
            cb.push_back(24.0 + (x * 0.15).sin() * 5.0 + (x * 0.9).cos() * 2.0);
            fo.push_back(8.0 + (x * 0.5).sin() * 5.0);
            sy.push_back(18.0 + (x * 1.3).sin() * 6.0);
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let mut m = Model {
            t: 0.0, acc: 0.0, rng: 0x2545_F491_4F6C_DD1D, goto: None,
            name: "Tomahawk-01".into(),
            state: MState::Printing,
            pos: [182.4, 96.1, 14.6, 1203.4],
            home: [true, true, true],
            job: Job { file: "housing_set_v7.gcode".into(), progress: 0.42, layer: 118, layers: 280, elapsed: 0.42 * 27600.0, total: 27600.0, speed: 100.0, flow: 98.0, objects },
            heaters: vec![
                Heater { name: "Hotend", node: "Toolhead", cur: 275.0, target: 275.0, power: 38.0, max: 300.0, ctl: 1, wd_ms: 1500 },
                Heater { name: "Bed", node: "Mainboard", cur: 90.0, target: 90.0, power: 22.0, max: 120.0, ctl: 0, wd_ms: 3000 },
                Heater { name: "Chamber", node: "Chamber IO", cur: 52.0, target: 60.0, power: 71.0, max: 80.0, ctl: 0, wd_ms: 3000 },
            ],
            fans: vec![
                Fan { name: "Part fan", node: "Toolhead", pct: 60.0, rpm_max: 9000.0 },
                Fan { name: "Heatsink fan", node: "Toolhead", pct: 100.0, rpm_max: 7500.0 },
                Fan { name: "Exhaust", node: "Chamber IO", pct: 25.0, rpm_max: 4200.0 },
                Fan { name: "Filter", node: "Chamber IO", pct: 40.0, rpm_max: 3600.0 },
                Fan { name: "Board fan", node: "Mainboard", pct: 35.0, rpm_max: 5000.0 },
            ],
            drives: vec![
                drv("X", "Drive X", "LDO 42STH48 closed-loop", 1.9, 14.0, 41.0, 3.4, 0.98),
                drv("Y", "Drive Y", "LDO 42STH48 closed-loop", 2.1, 16.0, 43.0, 3.6, 1.02),
                drv("Z1", "Drive Z1", "LDO 42STH40", 0.8, 5.0, 38.0, 2.2, 0.55),
                drv("Z2", "Drive Z2", "LDO 42STH40", 0.8, 6.0, 39.0, 2.2, 0.55),
                drv("Z3", "Drive Z3", "LDO 42STH40", 0.8, 5.0, 38.5, 2.2, 0.55),
                drv("E", "Drive E", "Moons 36 pancake", 1.2, 9.0, 47.0, 1.0, 0.31),
            ],
            nodes,
            files: vec![
                FileEntry { name: "housing_set_v7.gcode", size_mb: 48.2, mins: 460, grams: 312.0, objects: 5, mat: "PA-CF", slicer: "OrcaSlicer 2.3" },
                FileEntry { name: "benchy_calib.gcode", size_mb: 6.1, mins: 62, grams: 14.0, objects: 1, mat: "PETG", slicer: "PrusaSlicer 2.9" },
                FileEntry { name: "enclosure_panel_left.gcode", size_mb: 31.7, mins: 318, grams: 186.0, objects: 1, mat: "ASA", slicer: "OrcaSlicer 2.3" },
                FileEntry { name: "gear_train_x6.gcode", size_mb: 22.4, mins: 205, grams: 98.0, objects: 6, mat: "PLA", slicer: "OrcaSlicer 2.3" },
                FileEntry { name: "pa_calib_tower.gcode", size_mb: 0.4, mins: 18, grams: 6.0, objects: 1, mat: "PETG", slicer: "MACH5 app" },
                FileEntry { name: "mmu_test_4color.gcode", size_mb: 18.0, mins: 144, grams: 71.0, objects: 4, mat: "PLA x4", slicer: "OrcaSlicer 2.3" },
            ],
            profiles: vec![
                Profile { name: "PLA", nozzle: 215.0, bed: 60.0, chamber: 0.0, pa: 0.036, flow: 0.98, fan: 100.0 },
                Profile { name: "PETG", nozzle: 240.0, bed: 80.0, chamber: 0.0, pa: 0.052, flow: 0.96, fan: 50.0 },
                Profile { name: "ASA", nozzle: 255.0, bed: 100.0, chamber: 55.0, pa: 0.044, flow: 0.97, fan: 20.0 },
                Profile { name: "PA-CF", nozzle: 275.0, bed: 90.0, chamber: 60.0, pa: 0.031, flow: 0.95, fan: 30.0 },
                Profile { name: "TPU", nozzle: 230.0, bed: 50.0, chamber: 0.0, pa: 0.120, flow: 1.00, fan: 40.0 },
            ],
            fleet: crate::fleetsim::make_fleet(),
            clients: vec![
                Client { name: "Desktop - JH-WORKSTATION", kind: "Management Interface", fp: "SHA256:7Qk2...c9Ad", scope: "admin", seen: "now", revoked: false, age: 41 },
                Client { name: "Phone - JH-PIXEL", kind: "Phone port", fp: "SHA256:Lm81...Xv0f", scope: "operate", seen: "12 min ago", revoked: false, age: 40 },
                Client { name: "Tablet - SHOP-IPAD", kind: "Tablet port", fp: "SHA256:aB3e...92Qz", scope: "view", seen: "3 d ago", revoked: false, age: 88 },
                Client { name: "Failure detector", kind: "Service (PC)", fp: "SHA256:Zp04...d7Wn", scope: "pause", seen: "now", revoked: false, age: 12 },
                Client { name: "Old laptop", kind: "Management Interface", fp: "SHA256:Q91x...pL2s", scope: "admin", seen: "212 d ago", revoked: true, age: 301 },
            ],
            rules: vec![
                Rule { name: "Door open pauses print", on: true, trigger: 1, cond: "chamber.door == open AND state == printing".into(), actions: vec!["pause".into(), "notify(\"Door opened\")".into()], runs: 2 },
                Rule { name: "Filter fan after ASA", on: true, trigger: 2, cond: "job.material == ASA".into(), actions: vec!["fan.filter = 100% for 10 min".into()], runs: 14 },
                Rule { name: "Low spool warning", on: true, trigger: 3, cond: "spool.remaining < 60 g".into(), actions: vec!["notify(\"Spool low\")".into(), "pause_at_layer_end".into()], runs: 6 },
                Rule { name: "Chamber soak before PA-CF", on: false, trigger: 0, cond: "job.material == PA-CF".into(), actions: vec!["chamber.target = 60".into(), "wait chamber >= 55 C".into()], runs: 0 },
                Rule { name: "Cool down on idle 20 min", on: true, trigger: 4, cond: "idle > 20 min".into(), actions: vec!["heaters.off".into(), "led.dim(10%)".into()], runs: 31 },
            ],
            config: config_sections(),
            logs: Vec::new(),
            frames: VecDeque::new(),
            shapers: vec![
                Shaper { axis: "X", kind: 1, freq: 48.2, damp: 0.10, max_accel: 12000.0, when: "2026-10-26 09:14" },
                Shaper { axis: "Y", kind: 3, freq: 41.6, damp: 0.10, max_accel: 8500.0, when: "2026-10-26 09:16" },
            ],
            cal: Cal { running: false, step: 0, prog: 0.0 },
            fw: FwRun { running: false, prog: 0.0, idx: 0 },
            sim: Sim { running: false, prog: 0.0, results: Vec::new(), faults: [false; 4], log: Vec::new() },
            st: St { led: 60.0, q_copies: 1, extr_len: 10.0, pa_lo: 0.0, pa_hi: 0.1, tab: 0, ..Default::default() },
            h_hot: hot, h_bed: bed, h_cham: cham, h_t_hot: tgt,
            h_can_a: ca, h_can_b: cb, h_follow: fo, h_sync: sy,
            mesh: Vec::new(),
            manifest: "MF-2026.10.27-r2",
            pa_value: 0.044,
            probe_driver: 0, probe_comp: true, clear_needed: false,
            decisions: [false; 8], phases_done: 1,
            bld: crate::builder::Builder::new(),
            dash_order: load_dash_order(),
            hub_sel: std::collections::HashMap::new(),
            rollout: Rollout { wave: 0, prog: 0.0, running: false, halted: None, done: false },
            active: 0, queue: Vec::new(), next_qid: 1, auto_dispatch: false, auto_pause: true,
            alerts: Vec::new(), fspools: Vec::new(), next_spool: 1,
            printers: crate::spools::default_printers(), label_jobs: Vec::new(),
            label: crate::spools::default_label(), entry: crate::spools::default_entry(),
            print_tx: tx, print_rx: rx,
            disc: vec![Disc { name: "Tomahawk-15".into(), ip: "10.0.4.35".into() }, Disc { name: "Bench-C".into(), ip: "10.0.9.4".into() }],
            h_farm_n: std::iter::repeat(10.0).take(HIST).collect(), h_farm_g: (0..HIST).map(|k| 1750.0 + (k as f32 * 0.7).sin() * 120.0).collect(),
            low_thr: 150.0, farm_t: 0.0, next_event: 8.0, done_log: Vec::new(), scan_n: 0,
            accel: [6000.0, 6000.0, 400.0], jerk: 90000.0, snap: 8.0e6, vmax: 500.0, blend: 0.08,
            pa_smooth: 0.040, pa_on: true,
            mesh_adaptive: true, purge_adaptive: true, z_offset: -0.012,
            det_sens: 0.6, det_action: 0, det_on: true,
            lapse_on: true, lapse_every: 0,
            mmu_state: 3, tilt: [0.012, -0.008, 0.004], xtwist: 0.018,
            mmu_verbs: Vec::new(),
            resume_ready: true,
        };
        m.mesh = m.make_mesh();
        if let Some(n) = m.nodes.iter_mut().find(|n| n.class == "MMU") {
            n.want = "c04e77a1";
        }
        m.st.console.push("MACH5 console - typed commands are validated against the API schema".into());
        m.st.console.push("ok".into());
        m.seed_logs();
        m.seed_farm();
        m
    }

    fn make_mesh(&self) -> Vec<Vec<f32>> {
        (0..9)
            .map(|j| {
                (0..9)
                    .map(|k| {
                        let (x, y) = (k as f32 / 8.0 - 0.5, j as f32 / 8.0 - 0.5);
                        0.09 * (x * x + y * y) * 4.0 - 0.03 + 0.012 * (k as f32 * 1.7).sin() * (j as f32 * 1.1).cos() + 0.02 * x
                    })
                    .collect()
            })
            .collect()
    }

    pub fn remesh(&mut self) {
        self.mesh = self.make_mesh();
        for row in self.mesh.iter_mut() {
            for v in row.iter_mut() {
                *v += (self.rng % 7) as f32 * 0.0006;
                self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            }
        }
    }

    pub fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % 10000) as f32 / 10000.0
    }

    pub fn stamp(&self) -> String {
        let s = (self.t + 4360.0) as u32;
        format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    }

    pub fn log(&mut self, sev: Sev, src: &str, msg: &str) {
        let ts = self.stamp();
        self.logs.push(Log { ts, sev, src: src.into(), msg: msg.into() });
    }

    fn seed_logs(&mut self) {
        let rows: &[(Sev, &str, &str)] = &[
            (Sev::Info, "boot", "Host P4 up, schema v1.4.2 (hash 5e1a9c), signed manifest verified"),
            (Sev::Info, "can", "Segment A online 1M/5M, 6 nodes, termination OK"),
            (Sev::Info, "can", "Segment B online 1M/4M, 5 nodes"),
            (Sev::Warn, "nodes", "MMU image 71be0d95 differs from signed manifest (want c04e77a1)"),
            (Sev::Info, "sync", "Master clock locked, max skew 38 us"),
            (Sev::Info, "thermal", "Hotend target 275 C, within 0.4 C"),
            (Sev::Info, "job", "Start housing_set_v7.gcode (5 objects, PA-CF)"),
            (Sev::Warn, "chamber", "Door sensor reports open"),
            (Sev::Info, "chamber", "Door closed"),
            (Sev::Info, "job", "Layer 100/280 - checkpoint written to flash"),
            (Sev::Info, "rules", "Rule 'Filter fan after ASA' not matched"),
            (Sev::Err, "drive.Z2", "Following error 240 um exceeded threshold (200 um) - recovered"),
            (Sev::Info, "job", "Layer 118/280 - checkpoint written to flash"),
        ];
        for (k, (sv, src, msg)) in rows.iter().enumerate() {
            let s = 4360 - (rows.len() - k) as u32 * 173;
            self.logs.push(Log { ts: format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60), sev: *sv, src: src.to_string(), msg: msg.to_string() });
        }
    }

    pub fn estop(&mut self) {
        self.state = MState::Fault;
        self.clear_needed = true;
        for h in self.heaters.iter_mut() {
            h.target = 0.0;
        }
        self.log(Sev::Err, "safety", "E-STOP asserted: heaters off, drives disabled, fault latched");
    }

    pub fn clear_faults(&mut self) {
        self.clear_needed = false;
        self.state = MState::Idle;
        self.log(Sev::Info, "host", "Fault clear command sent to nodes (host-initiated)");
    }

    pub fn tick(&mut self, dt: f64) {
        self.t += dt;
        // slow state, every frame
        if self.state == MState::Printing {
            self.job.progress = (self.job.progress + dt as f32 * crate::fleetsim::SPEEDUP / self.job.total.max(1.0)).min(1.0);
            self.job.elapsed += dt as f32 * crate::fleetsim::SPEEDUP;
            self.job.layer = (self.job.progress * self.job.layers as f32) as u32;
            let t = self.t as f32;
            self.pos = [150.0 + 80.0 * (t * 0.9).sin(), 120.0 + 70.0 * (t * 0.7).cos(), 0.0 + self.job.layer as f32 * 0.12, self.pos[3] + dt as f32 * 6.0];
        }
        if self.cal.running {
            self.cal.prog += dt as f32 * 0.22;
            if self.cal.prog >= 1.0 {
                self.cal.prog = 0.0;
                self.cal.step += 1;
                if self.cal.step >= 6 {
                    self.cal.running = false;
                    self.cal.step = 6;
                    self.remesh();
                    self.state = MState::Idle;
                    self.log(Sev::Info, "autocal", "Auto-calibration complete: Z home, mesh, X twist, gantry level");
                }
            }
        }
        if self.fw.running {
            self.fw.prog += dt as f32 * 0.35;
            if self.fw.prog >= 1.0 {
                self.fw.prog = 0.0;
                let mut done = true;
                for (k, n) in self.nodes.iter_mut().enumerate() {
                    if n.fw != n.want && k >= self.fw.idx {
                        n.fw = n.want;
                        n.bank = if n.bank == 'A' { 'B' } else { 'A' };
                        n.note = "";
                        self.fw.idx = k + 1;
                        done = false;
                        break;
                    }
                }
                if done {
                    self.fw.running = false;
                    self.fw.idx = 0;
                    self.log(Sev::Info, "fw", "All nodes match signed manifest");
                }
            }
        }
        if self.sim.running {
            self.sim.prog += dt as f32 * 0.12;
            if self.sim.prog >= 1.0 {
                self.sim.running = false;
                self.sim.prog = 1.0;
                self.sim.results = vec![
                    ("Total move time", 4521.3, 4526.8, 0.5),
                    ("Peak velocity", 497.2, 497.9, 1.0),
                    ("Corner speed RMS", 142.0, 139.4, 3.0),
                    ("Max |accel|", 6000.0, 5994.0, 1.0),
                    ("Extruded length", 18204.1, 18204.1, 0.1),
                    ("Layer time (median)", 58.4, 58.1, 1.0),
                ];
                self.sim.log.push("planner run complete: 2,418,771 segments, 0 queue underruns".into());
            }
        }
        self.farm_tick(dt);
        // sampled state at 10 Hz
        self.acc += dt;
        if self.acc < 0.1 {
            return;
        }
        self.acc = 0.0;
        if self.t - self.st.con_t > 6.0 {
            self.st.con_t = self.t;
            let h = (self.heaters[0].cur, self.heaters[0].target);
            let line = format!("ok T:{:.1} /{:.1} B:{:.1} /{:.1} C:{:.1} /{:.1}", h.0, h.1, self.heaters[1].cur, self.heaters[1].target, self.heaters[2].cur, self.heaters[2].target);
            self.st.console.push(line);
            if h.1 > 0.0 && (h.0 - h.1).abs() > 4.0 {
                let msg = format!("warning: hotend {:.1} C is {:.1} C {} target", h.0, (h.0 - h.1).abs(), if h.0 < h.1 { "below" } else { "above" });
                self.st.console.push(msg);
            }
            if self.st.console.len() > 700 {
                self.st.console.drain(0..100);
            }
        }
        let t = self.t as f32;
        let n1 = self.rand() - 0.5;
        let n2 = self.rand() - 0.5;
        let n3 = self.rand() - 0.5;
        for h in self.heaters.iter_mut() {
            let k = if h.name == "Chamber" { 0.0008 } else { 0.01 };
            h.cur += (h.target - h.cur) * k * 6.0 + if h.target > 0.0 { 0.0 } else { (22.0 - h.cur) * 0.002 };
            h.power = if h.target > 0.0 { ((h.target - h.cur).abs() * 4.0 + 25.0).clamp(0.0, 100.0) } else { 0.0 };
        }
        push(&mut self.h_hot, self.heaters[0].cur + n1 * 0.6);
        push(&mut self.h_bed, self.heaters[1].cur + n2 * 0.3);
        push(&mut self.h_cham, self.heaters[2].cur + n3 * 0.05);
        push(&mut self.h_t_hot, self.heaters[0].target);
        let busy = if self.fw.running { 55.0 } else { 24.0 };
        push(&mut self.h_can_a, 11.0 + (t * 1.2).sin() * 1.2 + n1);
        push(&mut self.h_can_b, busy + (t * 0.5).sin() * 6.0 + n2 * 4.0);
        push(&mut self.h_follow, 8.0 + (t * 3.1).sin() * 5.0 + n3 * 4.0);
        push(&mut self.h_sync, 18.0 + (t * 4.3).sin() * 7.0 + n1 * 5.0);
        for d in self.drives.iter_mut() {
            d.cur = d.irms * (0.8 + 0.4 * ((t * 2.0).sin().abs()));
            d.follow = 8.0 + (t * 3.1 + d.irms).sin() * 5.0 + n3 * 3.0;
        }
        if !self.st.frames_paused {
            let ids: [(Seg, u32, &str, &str); 8] = [
                (Seg::A, 0x010, "HEARTBEAT", "Drive X"),
                (Seg::A, 0x011, "HEARTBEAT", "Drive Y"),
                (Seg::A, 0x0A0, "TIME_SYNC", "Host"),
                (Seg::A, 0x310, "TELEMETRY", "Drive X"),
                (Seg::B, 0x031, "HEARTBEAT", "Toolhead"),
                (Seg::B, 0x330, "TELEMETRY", "Toolhead"),
                (Seg::B, 0x530, "SENSOR_STREAM", "Eddy Probe"),
                (Seg::B, 0x250, "CONTROL", "Host"),
            ];
            for _ in 0..3 {
                let r = (self.rand() * 8.0) as usize % 8;
                let (seg, id, class, node) = ids[r];
                let data = (0..8).map(|_| format!("{:02X}", (self.rand() * 255.0) as u8)).collect::<Vec<_>>().join(" ");
                self.frames.push_back(Frame { t: self.t, seg, id, class, node, data });
            }
            while self.frames.len() > 200 {
                self.frames.pop_front();
            }
        }
    }
}

fn push(q: &mut VecDeque<f32>, v: f32) {
    q.push_back(v);
    while q.len() > HIST {
        q.pop_front();
    }
}

fn drv(axis: &'static str, node: &'static str, motor: &'static str, irms: f32, follow: f32, temp: f32, ld: f32, _k: f32) -> Drive {
    Drive { axis, node, motor, status: Status::Ok, cur: irms, follow, enc: "MT6826S 15-bit OK", temp, vbus: 48.1, mode: 0, r: 1.1 + ld * 0.05, l: ld, kt: 0.42, poles: 50, tuned: "2026-10-20", irms }
}

fn config_sections() -> Vec<Section> {
    vec![
        Section { name: "machine", fields: vec![
            s("name", "Tomahawk-01", "Display name announced on the network"),
            e("kinematics", 0, &["corexy", "cartesian", "corexz"], "Kinematic model"),
            f("bed_x", 350.0, "mm", 100.0, 1000.0, "Bed X size"),
            f("bed_y", 350.0, "mm", 100.0, 1000.0, "Bed Y size"),
            f("max_z", 330.0, "mm", 50.0, 1000.0, "Maximum Z travel"),
            i("z_motors", 3, "", 1.0, 8.0, "Number of Z motors (leveling adapts)"),
        ]},
        Section { name: "motion", fields: vec![
            f("max_velocity", 500.0, "mm/s", 10.0, 1500.0, "Toolhead velocity limit"),
            f("max_accel", 6000.0, "mm/s2", 100.0, 40000.0, "Toolhead acceleration limit"),
            f("max_jerk", 90000.0, "mm/s3", 1000.0, 1.0e6, "Third-order planner jerk limit"),
            f("blend_tolerance", 0.08, "mm", 0.0, 1.0, "Corner blend tolerance (no junction deviation)"),
            b("input_shaper.enabled", true, "Shaping applied inside the planner"),
            b("pressure_advance.enabled", true, "Pressure advance applied inside the planner"),
            f("pressure_advance.smooth_time", 0.040, "s", 0.0, 0.2, "PA smoothing window"),
        ]},
        Section { name: "thermal", fields: vec![
            f("hotend.max_temp", 300.0, "C", 0.0, 500.0, "Hard cutoff enforced on STM32 and node"),
            f("bed.max_temp", 120.0, "C", 0.0, 200.0, "Hard cutoff enforced on STM32"),
            f("chamber.max_temp", 80.0, "C", 0.0, 120.0, "Hard cutoff enforced on node"),
            i("watchdog.heartbeat_timeout", 1500, "ms", 100.0, 10000.0, "Heater node cuts heater after this many ms without Host heartbeat"),
            e("hotend.control", 1, &["pid", "mpc"], "Heater control algorithm"),
        ]},
        Section { name: "probe", fields: vec![
            e("driver", 0, &["mach5_eddy", "beacon", "cartographer"], "Probe driver (Beacon/Cartographer: compatibility only)"),
            b("thermal_compensation", true, "Temperature-compensated Z"),
            f("z_offset", -0.012, "mm", -2.0, 2.0, "Nozzle to probe offset"),
            i("samples", 5, "", 1.0, 50.0, "Samples per point"),
            f("sample_tolerance", 0.006, "mm", 0.0005, 0.1, "Reject outliers beyond this"),
        ]},
        Section { name: "can", fields: vec![
            i("segment_a.arb_bitrate", 1_000_000, "bit/s", 125_000.0, 1_000_000.0, "Arbitration bitrate (drive)"),
            i("segment_a.data_bitrate", 5_000_000, "bit/s", 1_000_000.0, 8_000_000.0, "Data-phase bitrate (drive)"),
            i("segment_b.arb_bitrate", 1_000_000, "bit/s", 125_000.0, 1_000_000.0, "Arbitration bitrate (peripherals)"),
            i("segment_b.data_bitrate", 4_000_000, "bit/s", 1_000_000.0, 8_000_000.0, "Data-phase bitrate (peripherals)"),
            f("time_sync.rate", 50.0, "Hz", 10.0, 100.0, "Master clock broadcast rate"),
            b("refuse_on_missing_node", true, "Refuse to run if a configured node is missing or the wrong class"),
        ]},
        Section { name: "network", fields: vec![
            s("hostname", "tomahawk-01", "mDNS hostname"),
            e("link", 1, &["wifi", "ethernet"], "Primary link on the Network Bridge"),
            b("discovery.announce", true, "Announce on the local network"),
            e("api.tls", 0, &["required"], "API transport policy"),
            i("api.port", 8443, "", 1024.0, 65535.0, "API port"),
        ]},
        Section { name: "safety", fields: vec![
            b("estop.latched", true, "E-stop latches until the Host sends a clear"),
            f("drive.following_error_limit", 200.0, "um", 10.0, 2000.0, "Drive fault threshold"),
            b("power_loss.checkpoint", true, "Checkpoint print state to flash"),
            i("power_loss.checkpoint_layers", 1, "layers", 1.0, 100.0, "Checkpoint every N layers"),
        ]},
    ]
}

pub const DASH_CARDS: usize = 14;

pub fn default_dash_order() -> Vec<u8> {
    (0..DASH_CARDS as u8).collect()
}

fn dash_path() -> std::path::PathBuf {
    let base = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join(".mach5-dashboard")
}

/// The saved dashboard card order, or the default if missing or invalid.
pub fn load_dash_order() -> Vec<u8> {
    let parsed: Option<Vec<u8>> = std::fs::read_to_string(dash_path()).ok().map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect());
    match parsed {
        Some(v) if v.len() == DASH_CARDS && (0..DASH_CARDS as u8).all(|i| v.contains(&i)) => v,
        _ => default_dash_order(),
    }
}

pub fn save_dash_order(order: &[u8]) {
    let _ = std::fs::write(dash_path(), order.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
}
