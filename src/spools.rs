//! Shared spool inventory: auto IDs, QR codes, thermal-printer labels and per-spool tracking.
use crate::data::*;
use crate::theme::Status;

pub const TODAY: &str = "2026-10-28";
pub const BRANDS: [&str; 9] = ["Polymaker", "Prusament", "Elegoo", "eSun", "Sunlu", "Fiberon", "Bambu Lab", "Overture", "Other"];
pub const MATS: [&str; 10] = ["PLA", "PETG", "ASA", "ABS", "PA-CF", "PA12-CF", "PA6", "TPU", "PC", "PVA"];
const DENS: [f32; 10] = [1.24, 1.27, 1.07, 1.04, 1.18, 1.17, 1.14, 1.21, 1.20, 1.23];
pub const SHELVES: [&str; 6] = ["A1", "A2", "B1", "B2", "C1", "Dry box"];

/// Manufacturer barcodes the lookup knows (GTIN, brand, material, colour, net g, tare g, cost).
pub const CATALOG: [(&str, &str, &str, &str, f32, f32, f32); 6] = [
    ("4712345000011", "Polymaker", "PLA", "Black", 1000.0, 230.0, 21.0),
    ("4712345000028", "Prusament", "PETG", "Galaxy Black", 1000.0, 240.0, 29.0),
    ("4712345000035", "Polymaker", "ASA", "Grey", 1000.0, 235.0, 32.0),
    ("4712345000042", "Polymaker", "PA-CF", "Black", 750.0, 200.0, 54.0),
    ("4712345000059", "eSun", "TPU", "Natural", 500.0, 150.0, 27.0),
    ("4712345000066", "Elegoo", "PLA", "White", 1000.0, 215.0, 17.0),
];

pub fn code(id: u32) -> String {
    format!("M5-SP-{:05}", id)
}
pub fn qr_payload(id: u32) -> String {
    format!("mach5:spool:{}", code(id))
}
pub fn density(mat: &str) -> f32 {
    MATS.iter().position(|m| *m == mat).map(|i| DENS[i]).unwrap_or(1.2)
}
pub fn length_m(mat: &str, dia: f32, g: f32) -> f32 {
    let area_cm2 = std::f32::consts::PI * (dia / 20.0).powi(2);
    g / (density(mat) * area_cm2 * 100.0)
}
pub fn profile_for(mat: &str) -> usize {
    match mat {
        "PLA" => 0,
        "PETG" => 1,
        "ASA" | "ABS" => 2,
        "PA-CF" | "PA12-CF" | "PA6" | "PC" => 3,
        "TPU" => 4,
        _ => 0,
    }
}
pub fn hygroscopic(mat: &str) -> bool {
    matches!(mat, "PA-CF" | "PA12-CF" | "PA6" | "TPU" | "PVA" | "PETG" | "PC")
}
fn days_between(a: &str, b: &str) -> i32 {
    let p = |s: &str| -> i32 {
        let v: Vec<i32> = s.split('-').filter_map(|x| x.parse().ok()).collect();
        if v.len() == 3 { v[0] * 372 + v[1] * 31 + v[2] } else { 0 }
    };
    p(b) - p(a)
}

pub fn default_entry() -> SpoolEntry {
    SpoolEntry {
        step: 0, source: 0, gtin: String::new(), brand: "Polymaker".into(), mat: 0, color: String::new(), dia: 0,
        net: 1000.0, tare: 230.0, cost: 20.0, supplier: String::new(), lot: String::new(), bought: TODAY.into(),
        shelf: "A1".into(), qty: 1, notes: String::new(), print_after: true, created: Vec::new(),
    }
}

pub fn default_label() -> LabelCfg {
    LabelCfg {
        printer: 0, size: 1, layout: 0, copies: 1, show_brand: true, show_dia: true, show_lot: true, show_weight: true,
        sel: Vec::new(), msg: String::new(), np_name: String::new(), np_addr: "10.0.4.70:9100".into(), np_conn: 0, np_lang: 0,
    }
}

pub fn default_printers() -> Vec<Printer> {
    vec![
        Printer { name: "Zebra ZD421 (demo)".into(), conn: Conn::Network, lang: Lang::Zpl, addr: "10.0.4.60:9100".into(), demo: true },
        Printer { name: "Munbyn ITPP941 (demo)".into(), conn: Conn::Usb, lang: Lang::Tspl, addr: "USB001".into(), demo: true },
        Printer { name: "Epson TM-L90 (demo)".into(), conn: Conn::Network, lang: Lang::EscPos, addr: "10.0.4.61:9100".into(), demo: true },
        Printer { name: "Zebra ZQ320 (demo)".into(), conn: Conn::Bluetooth, lang: Lang::Cpcl, addr: "BT:ZQ320".into(), demo: true },
    ]
}

impl Lang {
    pub fn name(self) -> &'static str {
        match self { Lang::Zpl => "ZPL (Zebra)", Lang::Tspl => "TSPL (TSC, Munbyn, Xprinter)", Lang::EscPos => "ESC/POS (Epson-style)", Lang::Cpcl => "CPCL (mobile)" }
    }
    pub fn ext(self) -> &'static str {
        match self { Lang::Zpl => "zpl", Lang::Tspl => "tspl", Lang::EscPos => "escpos.bin", Lang::Cpcl => "cpcl" }
    }
}
impl Conn {
    pub fn name(self) -> &'static str {
        match self { Conn::Network => "Network (RAW 9100)", Conn::Usb => "USB", Conn::Bluetooth => "Bluetooth" }
    }
}

pub struct LabelSize { pub name: &'static str, pub w: f32, pub h: f32 }
pub const SIZES: [LabelSize; 5] = [
    LabelSize { name: "40 x 30 mm", w: 40.0, h: 30.0 },
    LabelSize { name: "50 x 30 mm", w: 50.0, h: 30.0 },
    LabelSize { name: "57 x 32 mm", w: 57.0, h: 32.0 },
    LabelSize { name: "62 x 29 mm", w: 62.0, h: 29.0 },
    LabelSize { name: "76 x 51 mm", w: 76.0, h: 51.0 },
];
pub const LAYOUTS: [&str; 3] = ["QR left, text right", "Text left, QR right", "QR only"];

/// QR module matrix: (width, dark modules row-major).
pub fn qr_modules(data: &str) -> (usize, Vec<bool>) {
    use qrcode::{Color, EcLevel, QrCode};
    match QrCode::with_error_correction_level(data.as_bytes(), EcLevel::M) {
        Ok(c) => (c.width(), c.to_colors().into_iter().map(|x| x == Color::Dark).collect()),
        Err(_) => (0, Vec::new()),
    }
}

pub fn label_lines(s: &FSpool, c: &LabelCfg) -> Vec<String> {
    let mut l = vec![code(s.id)];
    l.push(if c.show_brand { format!("{} {}", s.brand, s.mat) } else { s.mat.clone() });
    let mut third = s.color.clone();
    if c.show_dia { third.push_str(&format!("  {:.2}mm", s.dia)); }
    l.push(third);
    if c.show_weight { l.push(format!("{:.0} g net", s.net)); }
    if c.show_lot && !s.lot.is_empty() { l.push(format!("Lot {}", s.lot)); }
    l
}

pub struct Geo { pub w: i32, pub h: i32, pub qr_x: i32, pub qr_y: i32, pub mag: i32, pub text_x: i32, pub text_w: i32, pub show_text: bool }

pub fn geometry(size: &LabelSize, layout: usize, n_modules: usize) -> Geo {
    let (w, h) = ((size.w * 8.0) as i32, (size.h * 8.0) as i32);
    let m = 10;
    let mag = (((h - 2 * m) as f32 / (n_modules.max(1) as f32 + 2.0)) as i32).clamp(2, 10);
    let qr = mag * (n_modules as i32 + 2);
    let qr_y = (h - qr) / 2;
    match layout {
        1 => Geo { w, h, qr_x: w - qr - m, qr_y, mag, text_x: m, text_w: (w - qr - 3 * m).max(40), show_text: true },
        2 => Geo { w, h, qr_x: (w - qr) / 2, qr_y, mag, text_x: 0, text_w: 0, show_text: false },
        _ => Geo { w, h, qr_x: m, qr_y, mag, text_x: qr + 2 * m, text_w: (w - qr - 3 * m).max(40), show_text: true },
    }
}

/// Printer-ready bytes for one spool label (203 dpi assumed).
pub fn render(lang: Lang, s: &FSpool, c: &LabelCfg) -> Vec<u8> {
    let size = &SIZES[c.size.min(SIZES.len() - 1)];
    let payload = qr_payload(s.id);
    let (n, _) = qr_modules(&payload);
    let g = geometry(size, c.layout, n);
    let lines = label_lines(s, c);
    match lang {
        Lang::Zpl => {
            let mut o = format!("^XA\n^CI28\n^PW{}\n^LL{}\n^LH0,0\n", g.w, g.h);
            o.push_str(&format!("^FO{},{}^BQN,2,{}^FDMA,{}^FS\n", g.qr_x, g.qr_y, g.mag, payload));
            if g.show_text {
                let mut y = 14;
                for (i, l) in lines.iter().enumerate() {
                    let fh = if i == 0 { 32 } else { 24 };
                    o.push_str(&format!("^FO{},{}^A0N,{},{}^FB{},1,0,L^FD{}^FS\n", g.text_x, y, fh, fh, g.text_w, l));
                    y += fh + 10;
                }
            }
            o.push_str(&format!("^PQ{}\n^XZ\n", c.copies.max(1)));
            o.into_bytes()
        }
        Lang::Tspl => {
            let mut o = format!("SIZE {} mm,{} mm\r\nGAP 2 mm,0 mm\r\nDIRECTION 0\r\nCLS\r\n", size.w, size.h);
            o.push_str(&format!("QRCODE {},{},M,{},A,0,\"{}\"\r\n", g.qr_x, g.qr_y, g.mag, payload));
            if g.show_text {
                let mut y = 14;
                for (i, l) in lines.iter().enumerate() {
                    let (font, mul, step) = if i == 0 { ("3", 1, 40) } else { ("2", 1, 28) };
                    o.push_str(&format!("TEXT {},{},\"{}\",0,{},{},\"{}\"\r\n", g.text_x, y, font, mul, mul, l));
                    y += step;
                }
            }
            o.push_str(&format!("PRINT {},1\r\n", c.copies.max(1)));
            o.into_bytes()
        }
        Lang::Cpcl => {
            let mut o = format!("! 0 200 200 {} {}\r\nPW {}\r\n", g.h, c.copies.max(1), g.w);
            o.push_str(&format!("BARCODE QR {} {} M 2 U {}\r\nMA,{}\r\nENDQR\r\n", g.qr_x, g.qr_y, g.mag, payload));
            if g.show_text {
                let mut y = 14;
                for (i, l) in lines.iter().enumerate() {
                    o.push_str(&format!("TEXT {} 0 {} {} {}\r\n", if i == 0 { 4 } else { 0 }, g.text_x, y, l));
                    y += if i == 0 { 40 } else { 28 };
                }
            }
            o.push_str("FORM\r\nPRINT\r\n");
            o.into_bytes()
        }
        Lang::EscPos => {
            let mut b: Vec<u8> = vec![0x1B, 0x40];
            for (i, l) in lines.iter().enumerate() {
                if i == 0 { b.extend_from_slice(&[0x1B, 0x45, 0x01]); }
                b.extend_from_slice(l.as_bytes());
                b.push(b'\n');
                if i == 0 { b.extend_from_slice(&[0x1B, 0x45, 0x00]); }
            }
            let d = payload.as_bytes();
            let len = d.len() + 3;
            b.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x04, 0x00, 0x31, 0x41, 0x32, 0x00]);
            b.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x43, g.mag.clamp(3, 8) as u8]);
            b.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x45, 0x31]);
            b.extend_from_slice(&[0x1D, 0x28, 0x6B, (len & 0xFF) as u8, (len >> 8) as u8, 0x31, 0x50, 0x30]);
            b.extend_from_slice(d);
            b.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x51, 0x30]);
            b.extend_from_slice(&[0x1B, 0x64, 0x03, 0x1D, 0x56, 0x42, 0x00]);
            let one = b.clone();
            for _ in 1..c.copies.max(1) {
                b.extend_from_slice(&one);
            }
            b
        }
    }
}

pub fn home_dir_labels() -> std::path::PathBuf {
    let base = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("MACH5-labels")
}

impl Model {
    pub fn spool_idx(&self, id: u32) -> Option<usize> {
        self.fspools.iter().position(|s| s.id == id)
    }
    pub fn spool_log(&mut self, id: u32, msg: &str) {
        let ts = self.stamp();
        if let Some(i) = self.spool_idx(id) {
            self.fspools[i].history.push(format!("{} {}  {}", TODAY, ts, msg));
        }
    }
    pub fn mname(&self, i: usize) -> String {
        self.fleet.get(i).map(|f| f.name.clone()).unwrap_or_else(|| "?".into())
    }
    pub fn spool_on(&self, machine: usize, slot: u8) -> Option<usize> {
        self.fspools.iter().position(|s| s.loc == Loc::Machine(machine) && s.slot == slot)
    }
    /// Spools loaded on a machine, ordered by slot.
    pub fn loaded(&self, machine: usize) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.fspools.len()).filter(|&i| self.fspools[i].loc == Loc::Machine(machine)).collect();
        v.sort_by_key(|&i| self.fspools[i].slot);
        v
    }
    pub fn loaded_with(&self, machine: usize, mat: &str) -> Option<usize> {
        self.loaded(machine).into_iter().find(|&i| self.fspools[i].mat == mat)
    }

    pub fn spool_status(&self, s: &FSpool) -> (Status, String) {
        if s.loc == Loc::Archived { return (Status::Off, "archived".into()); }
        if s.left <= 5.0 { return (Status::Err, "empty".into()); }
        if s.left < self.low_thr { return (Status::Warn, "low".into()); }
        if hygroscopic(&s.mat) && !s.dried.is_empty() && days_between(&s.dried, TODAY) > 45 && s.loc != Loc::Dryer {
            return (Status::Warn, "dry before use".into());
        }
        (Status::Ok, "ok".into())
    }

    pub fn new_spool(&mut self, e: &SpoolEntry) -> u32 {
        let id = self.next_spool;
        self.next_spool += 1;
        let mat = MATS[e.mat.min(MATS.len() - 1)].to_string();
        self.fspools.push(FSpool {
            id, brand: e.brand.clone(), mat, color: if e.color.is_empty() { "Unnamed".into() } else { e.color.clone() },
            dia: if e.dia == 0 { 1.75 } else { 2.85 }, net: e.net, tare: e.tare, left: e.net, cost: e.cost,
            supplier: e.supplier.clone(), lot: e.lot.clone(), bought: e.bought.clone(), dried: String::new(),
            shelf: e.shelf.clone(), loc: Loc::Shelf, slot: 0, notes: e.notes.clone(), printed: false, used: 0.0, history: Vec::new(),
        });
        self.spool_log(id, &format!("Created via entry workflow ({:.0} g net, shelf {})", e.net, e.shelf));
        id
    }

    pub fn load_spool(&mut self, id: u32, machine: usize, slot: u8) -> Result<(), String> {
        let si = self.spool_idx(id).ok_or("unknown spool")?;
        let m = self.fleet.get(machine).ok_or("unknown machine")?;
        if slot >= m.slots {
            return Err(format!("{} has only {} slot(s)", m.name, m.slots));
        }
        if self.fspools[si].left <= 5.0 {
            return Err("spool is empty".into());
        }
        if let Some(oi) = self.spool_on(machine, slot) {
            if oi != si {
                let oid = self.fspools[oi].id;
                self.unload_spool(oid);
            }
        }
        let mn = self.mname(machine);
        let s = &mut self.fspools[si];
        s.loc = Loc::Machine(machine);
        s.slot = slot;
        self.spool_log(id, &format!("Loaded on {} slot T{}", mn, slot));
        Ok(())
    }

    pub fn unload_spool(&mut self, id: u32) {
        if let Some(i) = self.spool_idx(id) {
            let was = if let Loc::Machine(m) = self.fspools[i].loc { self.mname(m) } else { String::new() };
            self.fspools[i].loc = Loc::Shelf;
            self.fspools[i].slot = 0;
            self.spool_log(id, &format!("Unloaded from {} to shelf {}", was, self.fspools.get(i).map(|s| s.shelf.clone()).unwrap_or_default()));
        }
    }

    /// Weigh-in: enter the gross weight on the scale; remaining = gross - tare.
    pub fn weigh_spool(&mut self, id: u32, gross: f32) {
        if let Some(i) = self.spool_idx(id) {
            let left = (gross - self.fspools[i].tare).clamp(0.0, self.fspools[i].net);
            let old = self.fspools[i].left;
            self.fspools[i].left = left;
            self.spool_log(id, &format!("Weighed: {:.0} g gross, {:.0} g remaining (was {:.0} g)", gross, left, old));
        }
    }
    pub fn dry_spool(&mut self, id: u32) {
        if let Some(i) = self.spool_idx(id) {
            self.fspools[i].dried = TODAY.into();
            self.spool_log(id, "Dried (marked)");
        }
    }
    pub fn archive_spool(&mut self, id: u32) {
        if let Some(i) = self.spool_idx(id) {
            if matches!(self.fspools[i].loc, Loc::Machine(_)) { self.unload_spool(id); }
            self.fspools[i].loc = Loc::Archived;
            self.spool_log(id, "Archived");
        }
    }
    pub fn restore_spool(&mut self, id: u32) {
        if let Some(i) = self.spool_idx(id) {
            self.fspools[i].loc = Loc::Shelf;
            self.spool_log(id, "Restored from archive");
        }
    }
    pub fn delete_spool(&mut self, id: u32) {
        self.fspools.retain(|s| s.id != id);
        self.label.sel.retain(|x| *x != id);
    }

    /// Send labels for the selected spools. Demo printers simulate; real network printers get RAW data.
    pub fn print_labels(&mut self, ids: &[u32]) {
        let pi = self.label.printer.min(self.printers.len().saturating_sub(1));
        if self.printers.is_empty() || ids.is_empty() {
            self.label.msg = "Choose a printer and at least one spool".into();
            return;
        }
        let (name, lang, conn, addr, demo) = {
            let p = &self.printers[pi];
            (p.name.clone(), p.lang, p.conn, p.addr.clone(), p.demo)
        };
        let mut data: Vec<u8> = Vec::new();
        for id in ids {
            if let Some(i) = self.spool_idx(*id) {
                data.extend(render(lang, &self.fspools[i], &self.label));
            }
        }
        let status;
        if demo || conn != Conn::Network {
            // USB / Bluetooth need the OS spooler: write the raw file for the user to send.
            let dir = home_dir_labels();
            let _ = std::fs::create_dir_all(&dir);
            let file = dir.join(format!("labels-{}.{}", self.stamp().replace(':', ""), lang.ext()));
            let wrote = std::fs::write(&file, &data).is_ok();
            status = if demo { format!("Sent to {} (simulated); raw data saved to {}", name, file.display()) } else if wrote { format!("Saved raw {} data to {} - send it to the printer with the OS spooler", lang.ext(), file.display()) } else { "Could not write the label file".into() };
        } else {
            status = format!("Sending {} bytes to {} ...", data.len(), addr);
            let tx = self.print_tx.clone();
            let (a, n) = (addr.clone(), name.clone());
            std::thread::spawn(move || {
                use std::io::Write;
                use std::net::ToSocketAddrs;
                let res = a.to_socket_addrs().ok().and_then(|mut it| it.next()).ok_or("bad address".to_string()).and_then(|sa| {
                    std::net::TcpStream::connect_timeout(&sa, std::time::Duration::from_secs(3)).map_err(|e| e.to_string())
                }).and_then(|mut s| s.write_all(&data).map_err(|e| e.to_string()));
                let _ = tx.send(match res { Ok(()) => format!("Printed on {}", n), Err(e) => format!("Print to {} failed: {}", n, e) });
            });
        }
        let ts = self.stamp();
        for id in ids {
            if let Some(i) = self.spool_idx(*id) { self.fspools[i].printed = true; }
            self.spool_log(*id, &format!("Label printed on {} x{}", name, self.label.copies));
        }
        self.label_jobs.push(LabelJob { ts, codes: ids.to_vec(), copies: self.label.copies, printer: name, status: status.clone() });
        self.label.msg = status;
    }

    /// Called every tick: collect results of background print threads.
    pub fn poll_prints(&mut self) {
        while let Ok(msg) = self.print_rx.try_recv() {
            if let Some(j) = self.label_jobs.last_mut() { j.status = msg.clone(); }
            self.label.msg = msg.clone();
            self.log(Sev::Info, "labels", &msg);
        }
    }

    /// Look up a typed or scanned code (spool code or raw payload) and select the spool.
    pub fn scan(&mut self, text: &str) -> Option<u32> {
        let t = text.trim();
        let c = t.strip_prefix("mach5:spool:").unwrap_or(t).to_uppercase();
        let id = self.fspools.iter().find(|s| code(s.id) == c || code(s.id).replace('-', "") == c.replace('-', "")).map(|s| s.id);
        if let Some(id) = id { self.st.sel_spool_id = id; }
        id
    }
}

/// Write the raw label data for `ids` to a file under ~/MACH5-labels and return its path.
pub fn save_label_file(m: &Model, ids: &[u32]) -> Result<String, String> {
    let p = m.printers.get(m.label.printer).ok_or("no printer")?;
    let mut data = Vec::new();
    for id in ids {
        if let Some(i) = m.spool_idx(*id) { data.extend(render(p.lang, &m.fspools[i], &m.label)); }
    }
    let dir = home_dir_labels();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("labels-{}.{}", ids.len(), p.lang.ext()));
    std::fs::write(&file, data).map_err(|e| e.to_string())?;
    Ok(file.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_render_for_every_language_and_qr_is_valid() {
        let mut m = Model::new();
        let sp = m.fspools[0].clone();
        let cfg = default_label();
        let zpl = String::from_utf8(render(Lang::Zpl, &sp, &cfg)).unwrap();
        assert!(zpl.starts_with("^XA") && zpl.contains("^BQN") && zpl.contains(&code(sp.id)) && zpl.trim_end().ends_with("^XZ"));
        let tspl = String::from_utf8(render(Lang::Tspl, &sp, &cfg)).unwrap();
        assert!(tspl.contains("QRCODE") && tspl.contains("PRINT 1"));
        let cpcl = String::from_utf8(render(Lang::Cpcl, &sp, &cfg)).unwrap();
        assert!(cpcl.contains("BARCODE QR") && cpcl.contains("ENDQR"));
        let esc = render(Lang::EscPos, &sp, &cfg);
        assert_eq!(&esc[..2], &[0x1B, 0x40]);
        let (n, mods) = qr_modules(&qr_payload(sp.id));
        assert!(n >= 21 && mods.len() == n * n);
        // ids are sequential and unique
        let e = default_entry();
        let (a, b) = (m.new_spool(&e), m.new_spool(&e));
        assert_eq!(b, a + 1);
        assert_eq!(code(a), format!("M5-SP-{:05}", a));
    }

    #[test]
    fn load_unload_and_weigh_track_history() {
        let mut m = Model::new();
        let id = m.fspools.iter().find(|s| s.loc == Loc::Shelf).unwrap().id;
        m.load_spool(id, 10, 0).unwrap();
        assert_eq!(m.spool_on(10, 0), Some(m.spool_idx(id).unwrap()));
        m.weigh_spool(id, 500.0);
        let s = &m.fspools[m.spool_idx(id).unwrap()];
        assert_eq!(s.left, 500.0 - s.tare);
        m.unload_spool(id);
        assert!(m.fspools[m.spool_idx(id).unwrap()].history.len() >= 4);
        assert!(m.load_spool(id, 10, 9).is_err(), "slot out of range is refused");
    }
}
