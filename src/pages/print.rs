//! Print: a Mainsail-style G-code file browser and the machine's print history.
use crate::data::*;
use crate::fleetsim::JOBS;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, Layout, RichText, Ui};

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

pub fn print(ui: &mut Ui, m: &mut Model) {
    title(ui, "Print", "G-code files on the machine and its print history");
    files(ui, m);
    history(ui, m);
}

fn files(ui: &mut Ui, m: &mut Model) {
    panel(ui, "G-code files", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(mono("gcodes /").color(DIM));
            ui.add(egui::TextEdit::singleline(&mut m.st.file_q).hint_text("search files").desired_width(190.0));
            egui::ComboBox::from_id_salt("file_sort").width(150.0).selected_text(["Sort: name", "Sort: print time", "Sort: filament", "Sort: size"][m.st.file_sort]).show_ui(ui, |ui| {
                for (i, n) in ["Sort: name", "Sort: print time", "Sort: filament", "Sort: size"].iter().enumerate() { ui.selectable_value(&mut m.st.file_sort, i, *n); }
            });
            if button(ui, "Upload") {
                let n = m.files.len() + 1;
                let name: &'static str = Box::leak(format!("uploaded_part_{}.gcode", n).into_boxed_str());
                m.files.push(FileEntry { name, size_mb: 12.0, mins: 95, grams: 41.0, objects: 1, mat: "PLA", slicer: "OrcaSlicer 2.3" });
                m.log(Sev::Info, "files", &format!("Uploaded {}", name));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let used: f32 = m.files.iter().map(|f| f.size_mb).sum();
                ui.label(dim(format!("{:.0} MB used  -  11.2 GB free on SD", used)));
            });
        });
        ui.add_space(8.0);
        let q = m.st.file_q.to_lowercase();
        let mut idx: Vec<usize> = (0..m.files.len()).filter(|&i| q.is_empty() || m.files[i].name.to_lowercase().contains(&q)).collect();
        match m.st.file_sort {
            1 => idx.sort_by_key(|&i| std::cmp::Reverse(m.files[i].mins)),
            2 => idx.sort_by(|&a, &b| m.files[b].grams.partial_cmp(&m.files[a].grams).unwrap()),
            3 => idx.sort_by(|&a, &b| m.files[b].size_mb.partial_cmp(&m.files[a].size_mb).unwrap()),
            _ => idx.sort_by_key(|&i| m.files[i].name),
        }
        let cards: Vec<(usize, FileEntry)> = idx.iter().map(|&i| (i, m.files[i].clone())).collect();
        let mut act: Option<(usize, u8)> = None;
        card_grid(ui, cards.len(), 330.0, |ui, n, w| {
            let (i, f) = &cards[n];
            let col = filament_rgb(file_color(f.name));
            card_frame(ui, w, m.st.sel_file == *i, |ui| {
                ui.horizontal_top(|ui| {
                    thumb(ui, 104.0, f.name, col);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 5.0;
                        ui.add(egui::Label::new(RichText::new(short_name(f.name)).strong().size(14.0).color(WHITE)).truncate());
                        ui.horizontal(|ui| {
                            color_chip(ui, f.mat, col);
                            ui.label(dim(format!("{} object{}", f.objects, if f.objects == 1 { "" } else { "s" })));
                        });
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                            crate::icons::glyph(&ui.painter().clone(), "clock", r.center(), Color32::from_gray(170), 0.0);
                            ui.label(mono(dur(f.mins)).size(13.0).color(WHITE));
                        });
                        ui.label(dim(format!("{:.0} g   {:.1} MB   {}", f.grams, f.size_mb, f.slicer)).size(11.5));
                    });
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if primary(ui, "Print") { act = Some((*i, 0)); }
                    if sbtn(ui, "Add to queue").clicked() { act = Some((*i, 1)); }
                    if sbtn(ui, "Delete").clicked() { act = Some((*i, 2)); }
                });
            });
        });
        match act {
            Some((i, 0)) => {
                let name = m.files[i].name;
                let job = JOBS.iter().position(|j| j.0 == name).unwrap_or(0);
                let a = m.active;
                m.m_start(a, job);
                m.job.objects.iter_mut().for_each(|o| o.excluded = false);
                m.st.sel_file = i;
                m.log(Sev::Info, "job", &format!("Start {}", name));
            }
            Some((i, 1)) => {
                let f = m.files[i].clone();
                if let Some(j) = JOBS.iter().find(|j| j.0 == f.name) {
                    let id = m.next_qid;
                    m.next_qid += 1;
                    m.queue.push(QJob { id, file: j.0, material: j.1, mins: j.2, left: 1, prio: 1, target: None, running: None });
                    m.log(Sev::Info, "farm", &format!("Queued #{} {}", id, f.name));
                }
            }
            Some((i, _)) => { let n = m.files[i].name; m.files.remove(i); m.log(Sev::Warn, "files", &format!("Deleted {}", n)); }
            None => {}
        }
    });
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
            for (i, n) in ["All", "Completed", "Cancelled", "Failed"].iter().enumerate() {
                if choice_btn(ui, m.st.hist_filter == i, n) { m.st.hist_filter = i; }
            }
        });
        ui.add_space(6.0);
        let list: Vec<&HistEntry> = all.iter().filter(|h| match m.st.hist_filter { 1 => h.status == 0, 2 => h.status == 1, 3 => h.status == 2, _ => true }).collect();
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
