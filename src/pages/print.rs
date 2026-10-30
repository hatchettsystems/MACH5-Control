//! Print: a Mainsail-style G-code file browser (cards or list, with filters) and print history.
use crate::data::*;
use crate::fleetsim::JOBS;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, Layout, RichText, Sense, Ui};

const MATS: [&str; 7] = ["All", "PLA", "PETG", "ASA", "PA-CF", "TPU", "ABS"];

pub fn file_color(name: &str) -> &'static str {
    match name {
        "housing_set_v7.gcode" | "housing_set_v6.gcode" => "Black",
        "benchy_calib.gcode" => "Blue",
        n if n.starts_with("enclosure_panel") => "Grey",
        "gear_train_x6.gcode" => "Orange",
        "pa_calib_tower.gcode" => "Red",
        "mmu_test_4color.gcode" => "White",
        _ => "Natural",
    }
}

fn dur(mins: u32) -> String {
    format!("{}h {:02}m", mins / 60, mins % 60)
}

fn short_name(n: &str) -> String {
    n.trim_end_matches(".gcode").to_string()
}

fn hs(a: u32, b: u32) -> u32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 13)
}

/// Fifty example files: the six machine jobs first, then generated ones.
pub fn example_files() -> Vec<FileEntry> {
    let mut v = vec![
        FileEntry { name: "housing_set_v7.gcode", size_mb: 48.2, mins: 460, grams: 312.0, objects: 5, mat: "PA-CF", slicer: "OrcaSlicer 2.3", color: "Black", added: "2026-10-26", printed: 5 },
        FileEntry { name: "benchy_calib.gcode", size_mb: 6.1, mins: 62, grams: 14.0, objects: 1, mat: "PETG", slicer: "PrusaSlicer 2.9", color: "Blue", added: "2026-09-12", printed: 9 },
        FileEntry { name: "enclosure_panel_left.gcode", size_mb: 31.7, mins: 318, grams: 186.0, objects: 1, mat: "ASA", slicer: "OrcaSlicer 2.3", color: "Grey", added: "2026-10-02", printed: 4 },
        FileEntry { name: "gear_train_x6.gcode", size_mb: 22.4, mins: 205, grams: 98.0, objects: 6, mat: "PLA", slicer: "OrcaSlicer 2.3", color: "Orange", added: "2026-10-14", printed: 7 },
        FileEntry { name: "pa_calib_tower.gcode", size_mb: 0.4, mins: 18, grams: 6.0, objects: 1, mat: "PETG", slicer: "MACH5 app", color: "Red", added: "2026-10-20", printed: 12 },
        FileEntry { name: "mmu_test_4color.gcode", size_mb: 18.0, mins: 144, grams: 71.0, objects: 4, mat: "PLA", slicer: "OrcaSlicer 2.3", color: "White", added: "2026-10-18", printed: 3 },
    ];
    let first = ["bracket", "cable_clip", "fan_duct", "spool_holder", "phone_stand", "drawer_pull", "vase", "hinge", "gridfinity_bin", "motor_mount", "knob", "case_lid", "bearing_block", "tool_tray", "wall_hook", "panel_clip", "gear_box", "lamp_shade", "toolhead_cover", "rail_guide"];
    let second = ["v2", "v3", "set", "x4", "mk2", "left", "right", "large", "small", "rev_b", "plate", "pro"];
    let mats = ["PLA", "PLA", "PETG", "PETG", "ASA", "PA-CF", "TPU", "ABS", "PLA", "PETG"];
    let cols = ["Black", "White", "Grey", "Red", "Blue", "Orange", "Natural", "Green", "Yellow"];
    let slicers = ["OrcaSlicer 2.3", "PrusaSlicer 2.9", "OrcaSlicer 2.3", "Cura 5.9"];
    let mut i = 0u32;
    while v.len() < 50 {
        let h = hs(i, 31);
        let name: &'static str = Box::leak(format!("{}_{}.gcode", first[(h % 20) as usize], second[((h >> 5) % 12) as usize]).into_boxed_str());
        if v.iter().any(|f| f.name == name) { i += 1; continue; }
        let mat = mats[((h >> 9) % 10) as usize];
        let mins = 20 + (hs(i, 7) % 700);
        let grams = (mins as f32 * (0.18 + (hs(i, 9) % 40) as f32 / 100.0)).round();
        let col = if mat == "PA-CF" { "Black" } else { cols[((h >> 13) % 9) as usize] };
        v.push(FileEntry {
            name, size_mb: ((grams * 0.16 + 0.3) * 10.0).round() / 10.0, mins, grams, objects: 1 + hs(i, 3) % 8, mat,
            slicer: slicers[((h >> 17) % 4) as usize], color: col,
            added: Box::leak(format!("2026-{:02}-{:02}", 8 + hs(i, 5) % 3, 1 + hs(i, 6) % 28).into_boxed_str()), printed: hs(i, 8) % 6 * (hs(i, 2) % 2),
        });
        i += 1;
    }
    v
}

pub fn print(ui: &mut Ui, m: &mut Model) {
    title(ui, "Print", "G-code files on the machine and its print history");
    files(ui, m);
    history(ui, m);
}

fn passes(f: &FileEntry, m: &Model) -> bool {
    let q = m.st.file_q.to_lowercase();
    (q.is_empty() || f.name.to_lowercase().contains(&q))
        && (m.st.file_mat == 0 || f.mat == MATS[m.st.file_mat])
        && match m.st.file_time { 1 => f.mins < 60, 2 => (60..240).contains(&f.mins), 3 => f.mins >= 240, _ => true }
        && match m.st.file_seen { 1 => f.printed == 0, 2 => f.printed > 0, _ => true }
}

fn start_file(m: &mut Model, i: usize) {
    let f = m.files[i].clone();
    let job = JOBS.iter().position(|j| j.0 == f.name).or_else(|| JOBS.iter().position(|j| j.1 == f.mat)).unwrap_or(0);
    let a = m.active;
    m.m_start(a, job);
    let (mins, mat, name) = (f.mins, f.mat, f.name);
    m.with_machine(a, |fm| {
        fm.file = name.to_string();
        fm.material = JOBS.iter().find(|j| j.1 == mat).map(|j| j.1).unwrap_or(fm.material);
        fm.total = mins as f32 * 60.0;
        fm.layers = (mins as f32 / 1.6).max(1.0) as u32;
    });
    m.job.objects.iter_mut().for_each(|o| o.excluded = false);
    m.st.sel_file = i;
    m.files[i].printed += 1;
    m.log(Sev::Info, "job", &format!("Start {}", name));
}

fn queue_file(m: &mut Model, i: usize) {
    let f = m.files[i].clone();
    let id = m.next_qid;
    m.next_qid += 1;
    m.queue.push(QJob { id, file: f.name, material: f.mat, mins: f.mins, left: 1, prio: 1, target: None, running: None });
    m.log(Sev::Info, "farm", &format!("Queued #{} {}", id, f.name));
}

fn filter_bar(ui: &mut Ui, m: &mut Model, shown: usize, total: usize) {
    ui.horizontal_wrapped(|ui| {
        ui.label(mono("gcodes /").color(DIM));
        ui.add(egui::TextEdit::singleline(&mut m.st.file_q).hint_text("search files").desired_width(180.0));
        egui::ComboBox::from_id_salt("file_sort").width(150.0).selected_text(["Sort: name", "Sort: newest", "Sort: print time", "Sort: filament", "Sort: most printed"][m.st.file_sort]).show_ui(ui, |ui| {
            for (i, n) in ["Sort: name", "Sort: newest", "Sort: print time", "Sort: filament", "Sort: most printed"].iter().enumerate() { ui.selectable_value(&mut m.st.file_sort, i, *n); }
        });
        if icon_toggle(ui, "tiles", "Card view", !m.st.file_list) { m.st.file_list = false; }
        if icon_toggle(ui, "rows", "List view", m.st.file_list) { m.st.file_list = true; }
        if button(ui, "Upload") {
            let n = m.files.len() + 1;
            let name: &'static str = Box::leak(format!("uploaded_part_{}.gcode", n).into_boxed_str());
            m.files.push(FileEntry { name, size_mb: 12.0, mins: 95, grams: 41.0, objects: 1, mat: "PLA", slicer: "OrcaSlicer 2.3", color: "Natural", added: "2026-10-30", printed: 0 });
            m.log(Sev::Info, "files", &format!("Uploaded {}", name));
        }
    });
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(dim("Material"));
        for (i, n) in MATS.iter().enumerate() { if choice_btn(ui, m.st.file_mat == i, n) { m.st.file_mat = i; } }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(dim("Print time"));
        for (i, n) in ["Any", "Under 1 h", "1 to 4 h", "Over 4 h"].iter().enumerate() { if choice_btn(ui, m.st.file_time == i, n) { m.st.file_time = i; } }
        ui.add_space(10.0);
        ui.label(dim("History"));
        for (i, n) in ["Any", "Never printed", "Printed before"].iter().enumerate() { if choice_btn(ui, m.st.file_seen == i, n) { m.st.file_seen = i; } }
    });
    ui.horizontal_wrapped(|ui| {
        let used: f32 = m.files.iter().map(|f| f.size_mb).sum();
        ui.label(dim(format!("{} of {} files  -  {:.0} MB used  -  11.2 GB free on SD", shown, total, used)));
        if (m.st.file_mat != 0 || m.st.file_time != 0 || m.st.file_seen != 0 || !m.st.file_q.is_empty()) && sbtn(ui, "Clear filters").clicked() {
            m.st.file_mat = 0; m.st.file_time = 0; m.st.file_seen = 0; m.st.file_q.clear();
        }
    });
    ui.add_space(6.0);
}

fn files(ui: &mut Ui, m: &mut Model) {
    panel(ui, "G-code files", |ui| {
        let mut idx: Vec<usize> = (0..m.files.len()).filter(|&i| passes(&m.files[i], m)).collect();
        match m.st.file_sort {
            1 => idx.sort_by(|&a, &b| m.files[b].added.cmp(m.files[a].added)),
            2 => idx.sort_by_key(|&i| std::cmp::Reverse(m.files[i].mins)),
            3 => idx.sort_by(|&a, &b| m.files[b].grams.partial_cmp(&m.files[a].grams).unwrap()),
            4 => idx.sort_by_key(|&i| std::cmp::Reverse(m.files[i].printed)),
            _ => idx.sort_by_key(|&i| m.files[i].name),
        }
        filter_bar(ui, m, idx.len(), m.files.len());
        let cards: Vec<(usize, FileEntry)> = idx.iter().map(|&i| (i, m.files[i].clone())).collect();
        if cards.is_empty() { ui.label(dim("No files match these filters.")); }
        let mut act: Option<(usize, u8)> = None;
        if !m.st.file_list {
            card_grid(ui, cards.len(), 196.0, |ui, n, w| {
                let (i, f) = &cards[n];
                let col = filament_rgb(f.color);
                card_frame(ui, w, m.st.sel_file == *i, |ui| {
                    let side = ui.available_width();
                    thumb(ui, side, f.name, col);
                    ui.add_space(6.0);
                    ui.add(egui::Label::new(RichText::new(short_name(f.name)).strong().size(13.5).color(WHITE)).truncate());
                    ui.horizontal(|ui| {
                        color_chip(ui, f.mat, col);
                        let (r, _) = ui.allocate_exact_size(vec2(15.0, 15.0), Sense::hover());
                        crate::icons::glyph(&ui.painter().clone(), "clock", r.center(), Color32::from_gray(170), 0.0);
                        ui.label(mono(dur(f.mins)).size(12.5).color(WHITE));
                    });
                    ui.label(dim(format!("{:.0} g  -  {} obj  -  {}x printed", f.grams, f.objects, f.printed)).size(11.5));
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if icon_btn(ui, "play", "Print this file", true) { act = Some((*i, 0)); }
                        if icon_btn(ui, "queue", "Add to the farm queue", false) { act = Some((*i, 1)); }
                        if icon_btn(ui, "trash", "Delete file", false) { act = Some((*i, 2)); }
                    });
                });
            });
        } else {
            for (i, f) in &cards {
                let col = filament_rgb(f.color);
                let w = ui.available_width();
                card_frame(ui, w, m.st.sel_file == *i, |ui| {
                    ui.horizontal(|ui| {
                        thumb(ui, 56.0, f.name, col);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            ui.add(egui::Label::new(RichText::new(short_name(f.name)).strong().size(13.5).color(WHITE)).truncate());
                            ui.horizontal_wrapped(|ui| {
                                color_chip(ui, f.mat, col);
                                ui.label(mono(dur(f.mins)).size(12.5).color(WHITE));
                                ui.label(dim(format!("{:.0} g   {:.1} MB   {} object{}   {}   added {}   {}x printed", f.grams, f.size_mb, f.objects, if f.objects == 1 { "" } else { "s" }, f.slicer, f.added, f.printed)).size(11.5));
                            });
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icon_btn(ui, "trash", "Delete file", false) { act = Some((*i, 2)); }
                            if icon_btn(ui, "queue", "Add to the farm queue", false) { act = Some((*i, 1)); }
                            if icon_btn(ui, "play", "Print this file", true) { act = Some((*i, 0)); }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        }
        match act {
            Some((i, 0)) => start_file(m, i),
            Some((i, 1)) => queue_file(m, i),
            Some((i, _)) => { let n = m.files[i].name; m.files.remove(i); m.log(Sev::Warn, "files", &format!("Deleted {}", n)); }
            None => {}
        }
    });
}

/// Days since a history timestamp ("2026-10-27 09:13"), relative to 30 Oct 2026.
fn days_ago(ended: &str) -> i32 {
    let d: i32 = ended.get(8..10).and_then(|s| s.parse().ok()).unwrap_or(30);
    let mth: i32 = ended.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(10);
    (10 - mth) * 30 + (30 - d)
}

fn history(ui: &mut Ui, m: &mut Model) {
    let act = m.active;
    let all: Vec<HistEntry> = m.history.iter().filter(|h| h.machine == act).cloned().collect();
    let done: Vec<&HistEntry> = all.iter().filter(|h| h.status == 0).collect();
    let total_h: f32 = all.iter().map(|h| h.secs).sum::<f32>() / 3600.0;
    let total_g: f32 = all.iter().map(|h| h.grams).sum();
    let longest = all.iter().map(|h| h.secs).fold(0.0_f32, f32::max) / 3600.0;
    let rate = if all.is_empty() { 0.0 } else { done.len() as f32 / all.len() as f32 * 100.0 };
    panel(ui, &format!("Print history - {}", m.name), |ui| {
        tiles_min(ui, &[
            ("Total jobs", all.len().to_string()),
            ("Print time", format!("{:.0} h", total_h)),
            ("Filament used", format!("{:.2} kg", total_g / 1000.0)),
            ("Longest print", format!("{:.1} h", longest)),
            ("Success rate", format!("{:.0} %", rate)),
        ], 120.0);
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::TextEdit::singleline(&mut m.st.hist_q).hint_text("search history").desired_width(170.0));
            ui.label(dim("Status"));
            for (i, n) in ["All", "Completed", "Cancelled", "Failed"].iter().enumerate() { if choice_btn(ui, m.st.hist_filter == i, n) { m.st.hist_filter = i; } }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(dim("Material"));
            for (i, n) in MATS.iter().enumerate() { if choice_btn(ui, m.st.hist_mat == i, n) { m.st.hist_mat = i; } }
            ui.add_space(10.0);
            ui.label(dim("Period"));
            for (i, n) in ["All time", "Last 7 days", "Last 30 days"].iter().enumerate() { if choice_btn(ui, m.st.hist_period == i, n) { m.st.hist_period = i; } }
        });
        ui.add_space(6.0);
        let q = m.st.hist_q.to_lowercase();
        let mut list: Vec<&HistEntry> = all.iter().filter(|h| {
            (m.st.hist_filter == 0 || h.status as usize + 1 == m.st.hist_filter)
                && (m.st.hist_mat == 0 || h.mat == MATS[m.st.hist_mat])
                && (q.is_empty() || h.file.to_lowercase().contains(&q))
                && match m.st.hist_period { 1 => days_ago(&h.ended) <= 7, 2 => days_ago(&h.ended) <= 30, _ => true }
        }).collect();
        list.sort_by(|a, b| b.ended.cmp(&a.ended));
        ui.label(dim(format!("{} of {} jobs", list.len(), all.len())));
        if list.is_empty() { ui.label(dim("No jobs match.")); }
        for h in list {
            let col = filament_rgb(h.color);
            row_card(ui, |ui| {
                ui.horizontal_top(|ui| {
                    thumb(ui, 62.0, &h.file, col);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(short_name(&h.file)).strong().size(13.5).color(WHITE));
                            match h.status { 0 => status(ui, Status::Ok, "Completed"), 1 => status(ui, Status::Warn, "Cancelled"), _ => status(ui, Status::Err, "Failed") }
                        });
                        ui.horizontal_wrapped(|ui| {
                            color_chip(ui, h.mat, col);
                            ui.label(mono(dur((h.secs / 60.0) as u32)).size(12.5).color(WHITE));
                            ui.label(dim(format!("{:.0} g", h.grams)));
                            ui.label(dim(&h.ended));
                        });
                    });
                });
            });
        }
    });
}

/// The most recent jobs of the active machine, newest first.
pub fn recent(m: &Model, n: usize) -> Vec<HistEntry> {
    let mut v: Vec<HistEntry> = m.history.iter().filter(|h| h.machine == m.active).cloned().collect();
    v.sort_by(|a, b| b.ended.cmp(&a.ended));
    v.truncate(n);
    v
}
