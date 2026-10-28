use crate::data::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, Ui};

pub fn can_bus(ui: &mut Ui, m: &mut Model) {
    title(ui, "CAN FD Bus", "Two physical segments, one protocol: A (drive) stays quiet, B (peripherals) carries streams and updates");
    ui.columns(2, |c| {
        for (k, col) in c.iter_mut().enumerate() {
            let (name, seg, ring, arb, dat) = if k == 0 { ("Segment A - drive", Seg::A, &m.h_can_a, 1.0, 5.0) } else { ("Segment B - peripherals", Seg::B, &m.h_can_b, 1.0, 4.0) };
            panel(col, name, |ui| {
                let load = *ring.back().unwrap_or(&0.0);
                let cnt = m.nodes.iter().filter(|n| n.seg == seg).count();
                kv(ui, if k == 0 { "can_a" } else { "can_b" }, &[
                    ("Bit rate", format!("{:.0} Mbit/s arb, {:.0} Mbit/s data, 64 B", arb, dat)),
                    ("Nodes", cnt.to_string()),
                    ("Bus load", format!("{:.1} %", load)),
                    ("Termination", "120 ohm both ends (measured 60.4 ohm)".into()),
                    ("Error counters", "TX 0  RX 0  bus-off 0".into()),
                ]);
                ui.add_space(4.0);
                plot(ui, 100.0, &[Series::from_ring("load", G1, ring)], (0.0, HIST as f32), (0.0, 100.0), "% load");
                if k == 1 && m.fw.running { ui.label(dim("Firmware transfer in progress (bulk class, lowest priority)")); }
            });
        }
    });
    ui.columns(2, |c| {
        panel(&mut c[0], "Message classes", |ui| {
            egui::Grid::new("classes").num_columns(4).striped(true).spacing([16.0, 5.0]).show(ui, |ui| {
                for h in ["Class", "Priority", "Content", "Rate"] { th(ui, h); }
                ui.end_row();
                for (cl, pr, ct, rt) in [
                    ("Fault / e-stop", "Highest", "Over-current, over-temp, encoder loss, power loss", "Event"),
                    ("Time sync", "High", "Master clock, sub-100 us alignment", "10-100 Hz"),
                    ("Heartbeat", "High", "Node state, error counters, firmware hash", "1-10 Hz"),
                    ("Control", "Medium", "Drive enable/mode, heater setpoints, fans", "On change"),
                    ("Telemetry", "Medium", "Temps, bus voltage, motor current, following error", "10-100 Hz"),
                    ("Sensor stream", "Low", "Eddy probe and ADXL samples", "~100s frames/s"),
                    ("Config", "Low", "Typed key-value writes, ack + readback", "On demand"),
                    ("Bulk transfer", "Lowest", "Signed firmware images, logs (B only)", "Segment B"),
                ] {
                    ui.label(mono(cl)); ui.label(pr); wrap(ui, 260.0, dim(ct)); ui.label(mono(rt));
                    ui.end_row();
                }
            });
        });
        panel(&mut c[1], "Time sync and safety behaviour", |ui| {
            let sk = *m.h_sync.back().unwrap_or(&0.0);
            kv(ui, "sync", &[("Master clock", "Host P4, 50 Hz broadcast".into()), ("Current skew", format!("{:.0} us", sk)), ("Limit", "100 us".into())]);
            plot(ui, 80.0, &[Series::from_ring("skew", G1, &m.h_sync), Series::new("limit", G3, vec![[0.0, 100.0], [HIST as f32, 100.0]]).dashed()], (0.0, HIST as f32), (0.0, 120.0), "us");
            ui.add_space(4.0);
            for t in [
                "Missing heartbeat from a heater node: that heater is cut locally.",
                "Missing master time sync: drives enter a controlled stop.",
                "Nodes never act on a fault-cleared state without a Host command.",
            ] { ui.label(format!("- {}", t)); }
        });
    });
    panel(ui, "Frame monitor", |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut m.st.frames_paused, "Pause");
            ui.label(dim("Segment"));
            for (k, n) in ["All", "A", "B"].iter().enumerate() {
                if ui.selectable_label(m.st.filter_node == k, *n).clicked() { m.st.filter_node = k; }
            }
        });
        egui::ScrollArea::vertical().max_height(220.0).auto_shrink([false, true]).show(ui, |ui| {
            egui::Grid::new("frames").num_columns(6).spacing([16.0, 2.0]).show(ui, |ui| {
                for h in ["Time", "Seg", "ID", "Class", "Node", "Data"] { th(ui, h); }
                ui.end_row();
                for f in m.frames.iter().rev().filter(|f| match m.st.filter_node { 1 => f.seg == Seg::A, 2 => f.seg == Seg::B, _ => true }).take(40) {
                    ui.label(mono(format!("{:.3}", f.t)).color(FAINT));
                    ui.label(mono(if f.seg == Seg::A { "A" } else { "B" }));
                    ui.label(mono(format!("{:03X}", f.id)));
                    ui.label(mono(f.class));
                    ui.label(dim(f.node));
                    ui.label(mono(f.data.clone()).color(DIM));
                    ui.end_row();
                }
            });
        });
    });
}

pub fn nodes(ui: &mut Ui, m: &mut Model) {
    title(ui, "Nodes", "Every module shares one CAN FD node core: bootloader, identity, telemetry");
    panel(ui, "Hardware family", |ui| {
        egui::Grid::new("nodes").num_columns(9).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
            for h in ["Board", "Class", "MCU", "Link", "Addr", "State", "Heartbeat", "Temp", "Firmware"] { th(ui, h); }
            ui.end_row();
            for (k, n) in m.nodes.iter().enumerate() {
                if ui.selectable_label(m.st.sel_node == k, n.name).clicked() { m.st.sel_node = k; }
                ui.label(mono(n.class));
                ui.label(dim(n.mcu));
                ui.label(dim(n.seg.label()));
                ui.label(mono(format!("0x{:02X}", n.addr)));
                status(ui, n.status, if n.note.is_empty() { n.status.label() } else { n.note });
                ui.label(mono(format!("{:.1} Hz", n.hb)));
                ui.label(mono(format!("{:.0} C / {}", n.temp, n.rated)));
                ui.label(mono(n.fw).color(if n.fw == n.want { TEXT } else { WARN }));
                ui.end_row();
            }
        });
        ui.add_space(4.0);
        ui.label(dim("Mainboard (ESP32-P4 + ESP32-S3 + STM32) is not a CAN node; it terminates both segments. Rated column: ambient C / part rating C."));
    });
    let k = m.st.sel_node.min(m.nodes.len() - 1);
    ui.columns(2, |c| {
        panel(&mut c[0], &format!("{} - identity", m.nodes[k].name), |ui| {
            let n = &m.nodes[k];
            kv(ui, "node_id", &[
                ("Role", n.role.into()),
                ("Factory ID", format!("{:016X}", n.uid)),
                ("Class code", n.class.into()),
                ("Short address", format!("0x{:02X} (assigned by Host on first contact)", n.addr)),
                ("Firmware hash", n.fw.into()),
                ("Active bank", format!("{}  (other bank holds rollback image)", n.bank)),
                ("Error counters", format!("TX {}  RX {}", n.tx_err, n.rx_err)),
                ("Design status", if n.proposed { "proposed in this plan".into() } else { "carried over".into() }),
            ]);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if button(ui, "Identify (blink LED)") { let nm = m.nodes[k].name; m.log(Sev::Info, "node", &format!("{} identify", nm)); }
                if button(ui, "Reboot") { let nm = m.nodes[k].name; m.log(Sev::Warn, "node", &format!("{} reboot requested", nm)); }
                if button(ui, "Open firmware") { m.goto = Some(Page::Firmware); }
            });
        });
        panel(&mut c[1], "Addressing and discovery", |ui| {
            ui.label("Each node has a factory 64-bit ID and a node-class code. The Host assigns a short address on first contact and stores the mapping in config.");
            ui.add_space(4.0);
            if let Some(f) = m.config.iter_mut().find(|s| s.name == "can").and_then(|s| s.fields.iter_mut().find(|f| f.key == "refuse_on_missing_node")) {
                if let Val::B(v) = &mut f.val { ui.checkbox(v, "Refuse to run if a configured node is missing or the wrong class"); }
            }
            ui.add_space(4.0);
            egui::Grid::new("discover").num_columns(3).striped(true).spacing([16.0, 5.0]).show(ui, |ui| {
                for h in ["Check", "Result", ""] { th(ui, h); }
                ui.end_row();
                for (c1, st, tx) in [
                    ("Configured nodes present", Status::Ok, "13 / 13"),
                    ("Class matches config", Status::Ok, "all match"),
                    ("Unknown nodes on bus", Status::Ok, "none"),
                    ("Duplicate factory IDs", Status::Ok, "none"),
                ] {
                    ui.label(c1); status(ui, st, tx); ui.label("");
                    ui.end_row();
                }
            });
            ui.add_space(4.0);
            if button(ui, "Run discovery") { m.log(Sev::Info, "can", "Discovery sweep: 13 nodes, 0 unknown"); }
        });
    });
}

pub fn firmware(ui: &mut Ui, m: &mut Model) {
    title(ui, "Firmware", "Signed manifest, per-node dual-bank images with rollback");
    let stale = m.nodes.iter().filter(|n| n.fw != n.want).count();
    ui.columns(2, |c| {
        panel(&mut c[0], "Signed manifest", |ui| {
            kv(ui, "manifest", &[
                ("Release", "1.4.2".into()),
                ("Manifest", m.manifest.into()),
                ("Signature", "ed25519, key MACH5-REL-02 - verified".into()),
                ("Out of date", format!("{} node(s)", stale)),
            ]);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_enabled_ui(stale > 0 && !m.fw.running, |ui| {
                    if primary(ui, "Flash out-of-date nodes (CAN B)") {
                        m.fw.running = true;
                        m.fw.idx = 0;
                        m.log(Sev::Info, "fw", "Update started: only nodes whose hash differs from the manifest");
                    }
                });
                if m.fw.running && button(ui, "Stop") { m.fw.running = false; }
            });
            if m.fw.running { bar_full(ui, m.fw.prog, None); }
        });
        panel(&mut c[1], "Update rules", |ui| {
            for t in [
                "Host reads each node's firmware hash from its heartbeat.",
                "Only nodes whose image differs from the signed manifest are flashed.",
                "Images are written to the inactive bank, then boot-checked.",
                "A failed boot check rolls back to the previous bank automatically.",
                "Bulk transfer runs on segment B only, at lowest priority.",
            ] { ui.label(format!("- {}", t)); }
        });
    });
    panel(ui, "Nodes and mainboard MCUs", |ui| {
        egui::Grid::new("fw").num_columns(7).striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
            for h in ["Target", "MCU", "Hash", "Manifest", "Bank", "State", ""] { th(ui, h); }
            ui.end_row();
            for (n, mcu, h) in [("Host", "ESP32-P4", "p4-1.4.2"), ("Network Bridge", "ESP32-S3", "s3-1.4.2"), ("Motion Bridge", "STM32", "mb-1.4.2")] {
                ui.label(n); ui.label(dim(mcu)); ui.label(mono(h)); ui.label(mono(h));
                ui.label(mono("A / B")); status(ui, Status::Ok, "current");
                let _ = ui.small_button("stage OTA");
                ui.end_row();
            }
            for k in 0..m.nodes.len() {
                let n = &m.nodes[k];
                let cur = n.fw == n.want;
                let (nm, mcu, fwh, bank, class, want) = (n.name, n.mcu, n.fw, n.bank, n.class, n.want);
                ui.label(nm); ui.label(dim(mcu)); ui.label(mono(fwh).color(if cur { TEXT } else { WARN })); ui.label(mono(want));
                ui.label(mono(format!("{} -> {}", bank, if bank == 'A' { 'B' } else { 'A' })));
                status(ui, if cur { Status::Ok } else { Status::Warn }, if cur { "current" } else { "update available" });
                if ui.small_button("rollback").clicked() {
                    m.nodes[k].bank = if bank == 'A' { 'B' } else { 'A' };
                    m.log(Sev::Warn, "fw", &format!("{} rolled back to previous bank ({})", nm, class));
                }
                ui.end_row();
            }
        });
    });
}
