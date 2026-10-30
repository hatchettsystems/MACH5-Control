//! Spool Inventory: one shared database for the whole farm, with an entry workflow,
//! automatic IDs and QR codes, thermal-printer labels, and per-spool tracking.
use crate::data::*;
use crate::spools::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, vec2, Align2, Color32, DragValue, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui};

/// Draw a QR code for `payload` into `rect` (dark modules on a white plate).
fn draw_qr(p: &egui::Painter, rect: Rect, payload: &str) {
    let (n, mods) = qr_modules(payload);
    p.rect_filled(rect, 3.0, Color32::WHITE);
    if n == 0 {
        return;
    }
    let quiet = 2.0;
    let cell = (rect.width().min(rect.height())) / (n as f32 + quiet * 2.0);
    let origin = rect.center() - vec2(n as f32 * cell / 2.0, n as f32 * cell / 2.0);
    for y in 0..n {
        for x in 0..n {
            if mods[y * n + x] {
                p.rect_filled(Rect::from_min_size(origin + vec2(x as f32 * cell, y as f32 * cell), vec2(cell + 0.4, cell + 0.4)), 0.0, Color32::BLACK);
            }
        }
    }
}

fn qr_widget(ui: &mut Ui, payload: &str, size: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    draw_qr(&ui.painter_at(r), r, payload);
}

fn pct(s: &FSpool) -> f32 {
    s.left / s.net.max(1.0)
}

fn loc_text(m: &Model, s: &FSpool) -> String {
    match &s.loc {
        Loc::Shelf => format!("Shelf {}", s.shelf),
        Loc::Dryer => "Dry box".into(),
        Loc::Machine(i) => format!("{}  T{}", m.mname(*i), s.slot),
        Loc::Archived => "Archived".into(),
    }
}

pub fn spools(ui: &mut Ui, m: &mut Model) {
    title(ui, "Spool Inventory", "One shared spool database for every machine: entry workflow, automatic IDs and QR labels, tracking from shelf to empty");
    // quick totals
    let active: Vec<&FSpool> = m.fspools.iter().filter(|s| s.loc != Loc::Archived).collect();
    let loaded = active.iter().filter(|s| matches!(s.loc, Loc::Machine(_))).count();
    let attention = active.iter().filter(|s| m.spool_status(s).0 != Status::Ok).count();
    let kg: f32 = active.iter().map(|s| s.left).sum::<f32>() / 1000.0;
    let value: f32 = active.iter().map(|s| s.cost * pct(s)).sum();
    panel(ui, "", |ui| {
        ui.horizontal(|ui| {
            for (big, cap) in [(format!("{}", active.len()), "spools tracked"), (format!("{}", loaded), "loaded on machines"), (format!("{:.1} kg", kg), "filament remaining"), (format!("${:.0}", value), "stock value"), (format!("{}", attention), "need attention")] {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(mono(big).size(19.0).color(if cap == "need attention" && attention > 0 { WARN } else { WHITE }));
                    ui.label(dim(cap));
                });
                ui.add_space(26.0);
            }
        });
    });
    tabs(ui, &mut m.st.tab, &["Inventory", "Add spools", "Labels", "Usage & tracking", "Scan & assign"]);
    match m.st.tab {
        0 => inventory(ui, m),
        1 => add_spools(ui, m),
        2 => labels(ui, m),
        3 => usage(ui, m),
        _ => scan(ui, m),
    }
}

fn inventory(ui: &mut Ui, m: &mut Model) {
    panel(ui, "Spool inventory", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::TextEdit::singleline(&mut m.st.sp_q).hint_text("search code, brand, colour, lot").desired_width(220.0));
            egui::ComboBox::from_id_salt("sp_mat").width(110.0).selected_text(if m.st.sp_mat == 0 { "All materials" } else { MATS[m.st.sp_mat - 1] }).show_ui(ui, |ui| {
                ui.selectable_value(&mut m.st.sp_mat, 0, "All materials");
                for (i, n) in MATS.iter().enumerate() { ui.selectable_value(&mut m.st.sp_mat, i + 1, *n); }
            });
            egui::ComboBox::from_id_salt("sp_loc").width(120.0).selected_text(["Any location", "On a machine", "Shelf", "Dry box", "Archived"][m.st.sp_loc]).show_ui(ui, |ui| {
                for (i, n) in ["Any location", "On a machine", "Shelf", "Dry box", "Archived"].iter().enumerate() { ui.selectable_value(&mut m.st.sp_loc, i, *n); }
            });
            ui.checkbox(&mut m.st.sp_low, "Needs attention only");
        });
        ui.add_space(4.0);
        let q = m.st.sp_q.to_lowercase();
        let rows: Vec<usize> = (0..m.fspools.len()).filter(|&i| {
            let s = &m.fspools[i];
            (q.is_empty() || code(s.id).to_lowercase().contains(&q) || s.brand.to_lowercase().contains(&q) || s.color.to_lowercase().contains(&q) || s.lot.to_lowercase().contains(&q) || s.mat.to_lowercase().contains(&q))
                && (m.st.sp_mat == 0 || s.mat == MATS[m.st.sp_mat - 1])
                && match m.st.sp_loc { 1 => matches!(s.loc, Loc::Machine(_)), 2 => s.loc == Loc::Shelf, 3 => s.loc == Loc::Dryer, 4 => s.loc == Loc::Archived, _ => s.loc != Loc::Archived }
                && (!m.st.sp_low || m.spool_status(s).0 != Status::Ok)
        }).collect();
        ui.label(dim(format!("{} spool(s)", rows.len())));
        egui::ScrollArea::vertical().id_salt("sp_tbl").max_height(520.0).auto_shrink([false, true]).show(ui, |ui| {
            let sel = m.st.sel_spool_id;
            let cards: Vec<(u32, String, String, f32, f32, String, (Status, String), bool, bool)> = rows.iter().map(|&i| {
                let s = &m.fspools[i];
                (s.id, format!("{} {}", s.brand, s.mat), s.color.clone(), s.left, pct(s), loc_text(m, s), m.spool_status(s), s.printed, s.left < m.low_thr)
            }).collect();
            let mut pick = None;
            card_grid(ui, cards.len(), 250.0, |ui, n, w| {
                let (id, title, color, left, frac, loc, st, printed, low) = &cards[n];
                card_frame(ui, w, sel == *id, |ui| {
                    if ui.selectable_label(sel == *id, RichText::new(code(*id)).monospace().strong().size(13.0)).clicked() { pick = Some(*id); }
                    ui.label(RichText::new(title.clone()).size(13.5).color(WHITE));
                    ui.label(dim(color.clone()));
                    ui.add_space(4.0);
                    accent_bar(ui, *frac, ui.available_width());
                    ui.label(mono(format!("{:.0} g left", left)).size(12.0).color(if *low { WARN } else { WHITE }));
                    ui.add_space(2.0);
                    ui.label(dim(loc.clone()).size(11.5));
                    ui.horizontal(|ui| {
                        status(ui, st.0, &st.1);
                        if !printed { ui.label(dim("no label")); }
                    });
                });
            });
            if let Some(id) = pick { m.st.sel_spool_id = id; }
        });
    });
    detail(ui, m);
}

fn detail(ui: &mut Ui, m: &mut Model) {
    let Some(si) = m.spool_idx(m.st.sel_spool_id) else {
        panel(ui, "Spool detail", |ui| { ui.label(dim("Select a spool above, or scan its label.")); });
        return;
    };
    let id = m.fspools[si].id;
    // actions are applied after drawing
    #[derive(Clone)]
    enum A { Load(usize, u8), Unload, Dry, Shelf(String), Archive, Restore, Delete, Dup, Weigh(f32), Label }
    let mut act: Option<A> = None;
    adaptive(ui, 2, |c| {
        panel(c.at(0), &format!("Spool {}", code(id)), |ui| {
            ui.horizontal(|ui| {
                qr_widget(ui, &qr_payload(id), 120.0);
                ui.vertical(|ui| {
                    let s = &m.fspools[si];
                    ui.label(RichText::new(format!("{} {}", s.brand, s.mat)).size(16.0).strong().color(WHITE));
                    ui.label(format!("{}  -  {:.2} mm", s.color, s.dia));
                    ui.add_space(2.0);
                    bar_full(ui, pct(s), if s.left < m.low_thr { Some(Status::Warn) } else { None });
                    ui.label(mono(format!("{:.0} g of {:.0} g  (~{:.0} m)", s.left, s.net, length_m(&s.mat, s.dia, s.left))));
                    let (st, tx) = m.spool_status(s);
                    status(ui, st, &tx);
                    ui.label(dim(format!("{}   QR: {}", loc_text(m, s), qr_payload(id))));
                });
            });
            ui.add_space(6.0);
            egui::Grid::new("sp_edit").num_columns(4).spacing([14.0, 6.0]).show(ui, |ui| {
                let s = &mut m.fspools[si];
                ui.label(dim("Brand")); ui.add(egui::TextEdit::singleline(&mut s.brand).desired_width(120.0));
                ui.label(dim("Colour")); ui.add(egui::TextEdit::singleline(&mut s.color).desired_width(120.0)); ui.end_row();
                ui.label(dim("Supplier")); ui.add(egui::TextEdit::singleline(&mut s.supplier).desired_width(120.0));
                ui.label(dim("Lot")); ui.add(egui::TextEdit::singleline(&mut s.lot).desired_width(120.0)); ui.end_row();
                ui.label(dim("Cost")); ui.add(DragValue::new(&mut s.cost).range(0.0..=500.0).prefix("$"));
                ui.label(dim("Tare")); ui.add(DragValue::new(&mut s.tare).range(0.0..=600.0).suffix(" g")); ui.end_row();
                ui.label(dim("Bought")); ui.label(mono(s.bought.clone()));
                ui.label(dim("Dried")); ui.label(mono(if s.dried.is_empty() { "-".to_string() } else { s.dried.clone() })); ui.end_row();
                ui.label(dim("Notes")); ui.add(egui::TextEdit::singleline(&mut s.notes).desired_width(120.0));
                ui.label(dim("Used")); ui.label(mono(format!("{:.0} g", s.used))); ui.end_row();
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(dim("Weigh-in (gross)"));
                if m.st.weigh_gross <= 0.0 { m.st.weigh_gross = m.fspools[si].left + m.fspools[si].tare; }
                ui.add(DragValue::new(&mut m.st.weigh_gross).range(0.0..=3000.0).suffix(" g"));
                if button(ui, "Record weight") { act = Some(A::Weigh(m.st.weigh_gross)); }
            });
            ui.horizontal_wrapped(|ui| {
                let on_machine = matches!(m.fspools[si].loc, Loc::Machine(_));
                if on_machine {
                    if button(ui, "Unload to shelf") { act = Some(A::Unload); }
                } else {
                    ui.menu_button("Load on machine...", |ui| {
                        for (mi, f) in m.fleet.iter().enumerate().filter(|(_, f)| f.group != 2 || f.slots > 0) {
                            ui.menu_button(format!("{}  ({} slots)", f.name, f.slots), |ui| {
                                for slot in 0..f.slots {
                                    let occ = m.spool_on(mi, slot).map(|x| code(m.fspools[x].id));
                                    let lbl = match occ { Some(c) => format!("T{}  (replace {})", slot, c), None => format!("T{}  empty", slot) };
                                    if ui.button(lbl).clicked() { act = Some(A::Load(mi, slot)); ui.close_menu(); }
                                }
                            });
                        }
                    });
                }
                ui.menu_button("Move to shelf...", |ui| {
                    for sh in SHELVES { if ui.button(sh).clicked() { act = Some(A::Shelf(sh.into())); ui.close_menu(); } }
                });
                if button(ui, "Mark dried") { act = Some(A::Dry); }
                if primary(ui, "Print label") { act = Some(A::Label); }
            });
            ui.horizontal_wrapped(|ui| {
                if button(ui, "Duplicate (new ID)") { act = Some(A::Dup); }
                if m.fspools[si].loc == Loc::Archived { if button(ui, "Restore") { act = Some(A::Restore); } } else if button(ui, "Archive") { act = Some(A::Archive); }
                if button(ui, "Delete") { act = Some(A::Delete); }
            });
        });
        panel(c.at(1), "Tracking history", |ui| {
            egui::ScrollArea::vertical().id_salt("sp_hist").max_height(380.0).auto_shrink([false, true]).show(ui, |ui| {
                for l in m.fspools[si].history.iter().rev() {
                    ui.horizontal_wrapped(|ui| { ui.label(mono(l.clone()).size(12.0)); });
                }
            });
        });
    });
    if let Some(a) = act {
        match a {
            A::Load(mi, slot) => { if let Err(e) = m.load_spool(id, mi, slot) { m.log(Sev::Warn, "spools", &e); } }
            A::Unload => m.unload_spool(id),
            A::Dry => m.dry_spool(id),
            A::Shelf(s) => { m.fspools[si].shelf = s.clone(); m.fspools[si].loc = Loc::Shelf; m.spool_log(id, &format!("Moved to shelf {}", s)); }
            A::Archive => m.archive_spool(id),
            A::Restore => m.restore_spool(id),
            A::Delete => { m.delete_spool(id); m.st.sel_spool_id = 0; }
            A::Dup => {
                let s = m.fspools[si].clone();
                let nid = m.next_spool; m.next_spool += 1;
                m.fspools.push(FSpool { id: nid, left: s.net, used: 0.0, loc: Loc::Shelf, slot: 0, printed: false, dried: String::new(), history: Vec::new(), ..s });
                m.spool_log(nid, &format!("Created as a copy of {}", code(id)));
                m.st.sel_spool_id = nid;
            }
            A::Weigh(g) => m.weigh_spool(id, g),
            A::Label => { m.label.sel = vec![id]; m.st.tab = 2; }
        }
        m.st.weigh_gross = 0.0;
    }
}

fn add_spools(ui: &mut Ui, m: &mut Model) {
    let steps = ["1  Source", "2  Details", "3  Review & labels"];
    panel(ui, "Add spools - step", |ui| {
        ui.horizontal(|ui| {
            for (i, s) in steps.iter().enumerate() {
                let cur = m.entry.step == i;
                ui.label(RichText::new(*s).color(if cur { ACCENT } else if i < m.entry.step { WHITE } else { DIM }).strong());
                if i < 2 { ui.label(dim(">")); }
            }
        });
    });
    match m.entry.step {
        0 => {
            panel(ui, "Where is the spool information coming from?", |ui| {
                ui.radio_value(&mut m.entry.source, 0, "Scan the manufacturer barcode (GTIN) - fills in brand, material, colour and weights");
                ui.radio_value(&mut m.entry.source, 1, "Enter details manually");
                ui.radio_value(&mut m.entry.source, 2, "Copy an existing spool (same product, new ID)");
                ui.add_space(6.0);
                match m.entry.source {
                    0 => {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut m.entry.gtin).hint_text("scan or type barcode").desired_width(200.0));
                            if button(ui, "Look up") {
                                if let Some(c) = CATALOG.iter().find(|c| c.0 == m.entry.gtin.trim()) { fill_from(&mut m.entry, c); m.st.scan_msg = format!("Found: {} {} {}", c.1, c.2, c.3); } else { m.st.scan_msg = "Barcode not in the catalog - enter details manually".into(); }
                            }
                            ui.label(dim("USB barcode scanners type into this field."));
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(dim("Sample barcodes"));
                            for c in CATALOG.iter() {
                                if sbtn(ui, &format!("{} {}", c.1, c.2)).clicked() { m.entry.gtin = c.0.into(); fill_from(&mut m.entry, c); m.st.scan_msg = format!("Found: {} {} {}", c.1, c.2, c.3); }
                            }
                        });
                        if !m.st.scan_msg.is_empty() { ui.label(dim(m.st.scan_msg.clone())); }
                    }
                    2 => {
                        egui::ComboBox::from_id_salt("entry_copy").width(300.0).selected_text("choose a spool to copy").show_ui(ui, |ui| {
                            for s in m.fspools.clone().iter() {
                                if ui.selectable_label(false, format!("{}  {} {} {}", code(s.id), s.brand, s.mat, s.color)).clicked() {
                                    m.entry.brand = s.brand.clone();
                                    m.entry.mat = MATS.iter().position(|x| *x == s.mat).unwrap_or(0);
                                    m.entry.color = s.color.clone();
                                    m.entry.net = s.net; m.entry.tare = s.tare; m.entry.cost = s.cost; m.entry.supplier = s.supplier.clone();
                                    m.entry.dia = if s.dia > 2.0 { 1 } else { 0 };
                                }
                            }
                        });
                    }
                    _ => {}
                }
                ui.add_space(6.0);
                if primary(ui, "Next") { m.entry.step = 1; }
            });
        }
        1 => {
            panel(ui, "Spool details", |ui| {
                let e = &mut m.entry;
                egui::Grid::new("entry_grid").num_columns(4).spacing([16.0, 8.0]).show(ui, |ui| {
                    ui.label(dim("Brand"));
                    egui::ComboBox::from_id_salt("e_brand").width(170.0).selected_text(e.brand.clone()).show_ui(ui, |ui| { for b in BRANDS { ui.selectable_value(&mut e.brand, b.to_string(), b); } });
                    ui.label(dim("Material"));
                    egui::ComboBox::from_id_salt("e_mat").width(170.0).selected_text(MATS[e.mat]).show_ui(ui, |ui| { for (i, n) in MATS.iter().enumerate() { ui.selectable_value(&mut e.mat, i, *n); } });
                    ui.end_row();
                    ui.label(dim("Colour")); ui.add(egui::TextEdit::singleline(&mut e.color).desired_width(170.0).hint_text("e.g. Galaxy Black"));
                    ui.label(dim("Diameter"));
                    egui::ComboBox::from_id_salt("e_dia").width(170.0).selected_text(["1.75 mm", "2.85 mm"][e.dia]).show_ui(ui, |ui| { ui.selectable_value(&mut e.dia, 0, "1.75 mm"); ui.selectable_value(&mut e.dia, 1, "2.85 mm"); });
                    ui.end_row();
                    ui.label(dim("Net weight")); ui.add(DragValue::new(&mut e.net).range(50.0..=10000.0).suffix(" g"));
                    ui.label(dim("Empty spool (tare)")); ui.add(DragValue::new(&mut e.tare).range(0.0..=800.0).suffix(" g")); ui.end_row();
                    ui.label(dim("Cost")); ui.add(DragValue::new(&mut e.cost).range(0.0..=1000.0).prefix("$"));
                    ui.label(dim("Supplier")); ui.add(egui::TextEdit::singleline(&mut e.supplier).desired_width(170.0)); ui.end_row();
                    ui.label(dim("Lot / batch")); ui.add(egui::TextEdit::singleline(&mut e.lot).desired_width(170.0));
                    ui.label(dim("Purchase date")); ui.add(egui::TextEdit::singleline(&mut e.bought).desired_width(170.0)); ui.end_row();
                    ui.label(dim("Shelf"));
                    egui::ComboBox::from_id_salt("e_shelf").width(170.0).selected_text(e.shelf.clone()).show_ui(ui, |ui| { for s in SHELVES { ui.selectable_value(&mut e.shelf, s.to_string(), s); } });
                    ui.label(dim("Quantity (identical spools)")); ui.add(DragValue::new(&mut e.qty).range(1..=50)); ui.end_row();
                    ui.label(dim("Notes")); ui.add(egui::TextEdit::singleline(&mut e.notes).desired_width(170.0));
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if button(ui, "Back") { e.step = 0; }
                    if primary(ui, "Next") { e.step = 2; }
                });
            });
        }
        _ => {
            let (first, last) = (m.next_spool, m.next_spool + m.entry.qty - 1);
            panel(ui, "Review", |ui| {
                let e = m.entry.clone();
                kv(ui, "entry_rev", &[
                    ("Spools", format!("{} x {} {} {}", e.qty, e.brand, MATS[e.mat], if e.color.is_empty() { "(no colour)".into() } else { e.color.clone() })),
                    ("Weights", format!("{:.0} g net, {:.0} g empty spool", e.net, e.tare)),
                    ("Cost / supplier", format!("${:.2} each, {}", e.cost, if e.supplier.is_empty() { "-" } else { &e.supplier })),
                    ("Stored", format!("shelf {}", e.shelf)),
                    ("IDs (automatic)", if e.qty == 1 { code(first) } else { format!("{} .. {}", code(first), code(last)) }),
                ]);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    qr_widget(ui, &qr_payload(first), 96.0);
                    ui.vertical(|ui| {
                        ui.label(dim("QR payload for the first spool"));
                        ui.label(mono(qr_payload(first)));
                        ui.add_space(4.0);
                        ui.checkbox(&mut m.entry.print_after, "Print labels immediately");
                        ui.label(dim(format!("Printer: {}   Label: {}", m.printers.get(m.label.printer).map(|p| p.name.as_str()).unwrap_or("none"), SIZES[m.label.size].name)));
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if button(ui, "Back") { m.entry.step = 1; }
                    if primary(ui, &format!("Create {} spool(s)", m.entry.qty)) {
                        let e = m.entry.clone();
                        let mut ids = Vec::new();
                        for _ in 0..e.qty { ids.push(m.new_spool(&e)); }
                        m.log(Sev::Info, "spools", &format!("Created {} spool(s): {} .. {}", ids.len(), code(ids[0]), code(*ids.last().unwrap())));
                        m.st.sel_spool_id = ids[0];
                        if e.print_after { m.label.sel = ids.clone(); m.print_labels(&ids); }
                        m.entry.created = ids;
                        m.entry.step = 3;
                    }
                });
            });
        }
    }
    if m.entry.step == 3 {
        let ids = m.entry.created.clone();
        panel(ui, "Spools created", |ui| {
            ui.label(RichText::new(format!("{} spool(s) added to the shared inventory", ids.len())).color(WHITE));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| { for id in ids.iter().take(12) { ui.label(mono(code(*id))); } if ids.len() > 12 { ui.label(dim(format!("+{} more", ids.len() - 12))); } });
            if !m.label.msg.is_empty() { ui.add_space(4.0); ui.label(dim(m.label.msg.clone())); }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if primary(ui, "Add another") { m.entry.step = 0; m.entry.created.clear(); m.entry.color.clear(); m.entry.lot.clear(); }
                if button(ui, "Print labels") { m.label.sel = ids.clone(); m.st.tab = 2; }
                if button(ui, "View in inventory") { m.st.tab = 0; }
            });
        });
    }
}

fn fill_from(e: &mut SpoolEntry, c: &(&str, &str, &str, &str, f32, f32, f32)) {
    e.brand = c.1.into();
    e.mat = MATS.iter().position(|x| *x == c.2).unwrap_or(0);
    e.color = c.3.into();
    e.net = c.4;
    e.tare = c.5;
    e.cost = c.6;
}

fn labels(ui: &mut Ui, m: &mut Model) {
    let preview_id = m.label.sel.first().copied().or_else(|| m.fspools.first().map(|s| s.id)).unwrap_or(0);
    adaptive(ui, 2, |c| {
        panel(c.at(0), "Label designer", |ui| {
            egui::Grid::new("lbl_cfg").num_columns(2).spacing([14.0, 7.0]).show(ui, |ui| {
                ui.label(dim("Printer"));
                let pn = m.printers.get(m.label.printer).map(|p| format!("{}  -  {}", p.name, p.lang.name())).unwrap_or_default();
                egui::ComboBox::from_id_salt("lbl_printer").width(280.0).selected_text(pn).show_ui(ui, |ui| {
                    for (i, p) in m.printers.iter().enumerate() { ui.selectable_value(&mut m.label.printer, i, format!("{}  -  {}  ({})", p.name, p.lang.name(), p.conn.name())); }
                });
                ui.end_row();
                ui.label(dim("Label size"));
                egui::ComboBox::from_id_salt("lbl_size").width(280.0).selected_text(SIZES[m.label.size].name).show_ui(ui, |ui| { for (i, s) in SIZES.iter().enumerate() { ui.selectable_value(&mut m.label.size, i, s.name); } });
                ui.end_row();
                ui.label(dim("Layout"));
                egui::ComboBox::from_id_salt("lbl_layout").width(280.0).selected_text(LAYOUTS[m.label.layout]).show_ui(ui, |ui| { for (i, s) in LAYOUTS.iter().enumerate() { ui.selectable_value(&mut m.label.layout, i, *s); } });
                ui.end_row();
                ui.label(dim("Copies each")); ui.add(DragValue::new(&mut m.label.copies).range(1..=20)); ui.end_row();
            });
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut m.label.show_brand, "Brand");
                ui.checkbox(&mut m.label.show_dia, "Diameter");
                ui.checkbox(&mut m.label.show_weight, "Net weight");
                ui.checkbox(&mut m.label.show_lot, "Lot");
            });
        });
        panel(c.at(1), "Label preview", |ui| {
            if let Some(si) = m.spool_idx(preview_id) {
                let s = m.fspools[si].clone();
                let size = &SIZES[m.label.size];
                let px = 5.2_f32;
                let (r, _) = ui.allocate_exact_size(vec2(size.w * px, size.h * px), Sense::hover());
                let p = ui.painter_at(r);
                p.rect_filled(r, 4.0, Color32::from_rgb(244, 244, 240));
                let payload = qr_payload(s.id);
                let (n, _) = qr_modules(&payload);
                let g = geometry(size, m.label.layout, n);
                let k = px / 8.0;
                let qr_side = g.mag as f32 * (n as f32 + 2.0) * k;
                draw_qr(&p, Rect::from_min_size(r.min + vec2(g.qr_x as f32 * k, g.qr_y as f32 * k), vec2(qr_side, qr_side)), &payload);
                if g.show_text {
                    let mut y = 14.0;
                    for (i, l) in label_lines(&s, &m.label).iter().enumerate() {
                        let fh = if i == 0 { 32.0 } else { 24.0 };
                        let fit = (g.text_w as f32 * k) / (l.chars().count().max(1) as f32 * 0.62);
                        p.text(r.min + vec2(g.text_x as f32 * k, y * k), Align2::LEFT_TOP, l, FontId::monospace((fh * k * 0.92).min(fit)), Color32::BLACK);
                        y += fh + 10.0;
                    }
                }
                p.rect_stroke(r, 4.0, Stroke::new(1.0_f32, LINE2), StrokeKind::Inside);
                ui.label(dim(format!("{}  at 203 dpi  -  previewing {}", size.name, code(s.id))));
            } else { ui.label(dim("No spool selected.")); }
        });
    });

    panel(ui, "Spools to print", |ui| {
        ui.horizontal_wrapped(|ui| {
            if button(ui, "All without a label") { m.label.sel = m.fspools.iter().filter(|s| !s.printed && s.loc != Loc::Archived).map(|s| s.id).collect(); }
            if button(ui, "Selected spool") { m.label.sel = vec![m.st.sel_spool_id]; }
            if button(ui, "Clear") { m.label.sel.clear(); }
            ui.label(dim(format!("{} selected", m.label.sel.len())));
        });
        egui::ScrollArea::vertical().id_salt("lbl_pick").max_height(180.0).auto_shrink([false, true]).show(ui, |ui| {
            egui::Grid::new("lbl_grid").num_columns(5).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                for s in m.fspools.clone().iter().filter(|s| s.loc != Loc::Archived) {
                    let mut on = m.label.sel.contains(&s.id);
                    if ui.checkbox(&mut on, "").changed() {
                        if on { m.label.sel.push(s.id) } else { m.label.sel.retain(|x| *x != s.id) }
                    }
                    ui.label(mono(code(s.id)));
                    ui.label(format!("{} {} {}", s.brand, s.mat, s.color));
                    ui.label(dim(loc_text(m, s)));
                    ui.label(dim(if s.printed { "label printed" } else { "no label yet" }));
                    ui.end_row();
                }
            });
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            let n = m.label.sel.len();
            ui.add_enabled_ui(n > 0, |ui| {
                if primary(ui, &format!("Print {} label(s)", n * m.label.copies as usize)) { let ids = m.label.sel.clone(); m.print_labels(&ids); }
                if button(ui, "Save label file") {
                    let ids = m.label.sel.clone();
                    m.label.msg = match save_label_file(m, &ids) { Ok(p) => format!("Saved {}", p), Err(e) => e };
                }
                if button(ui, "Copy commands") {
                    if let (Some(p), Some(id)) = (m.printers.get(m.label.printer), m.label.sel.first()) {
                        if let Some(si) = m.spool_idx(*id) { ui.ctx().copy_text(String::from_utf8_lossy(&render(p.lang, &m.fspools[si], &m.label)).to_string()); m.label.msg = "Commands copied".into(); }
                    }
                }
            });
        });
        if !m.label.msg.is_empty() { ui.label(dim(m.label.msg.clone())); }
        if let (Some(p), Some(si)) = (m.printers.get(m.label.printer), m.spool_idx(preview_id)) {
            ui.add_space(6.0);
            ui.label(RichText::new(format!("PRINTER DATA ({})", p.lang.ext())).size(11.0).color(DIM).monospace());
            let bytes = render(p.lang, &m.fspools[si], &m.label);
            let txt: String = if p.lang == Lang::EscPos { bytes.iter().map(|b| format!("{:02X} ", b)).collect::<String>() } else { String::from_utf8_lossy(&bytes).to_string() };
            egui::ScrollArea::vertical().id_salt("lbl_cmd").max_height(150.0).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut txt.as_str()).desired_width(f32::INFINITY).desired_rows(6).font(FontId::monospace(11.5)));
            });
        }
    });

    adaptive(ui, 2, |c| {
        panel(c.at(0), "Printers", |ui| {
            let mut rm = None;
            for (i, p) in m.printers.iter().enumerate() {
                row_card(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(&p.name).strong().size(13.5).color(WHITE)).wrap());
                    ui.add(egui::Label::new(dim(format!("{}  -  {} {}", p.lang.name(), p.conn.name(), p.addr))).wrap());
                    if !p.demo && sbtn(ui, "Remove").clicked() { rm = Some(i); }
                });
            }
            if let Some(i) = rm { m.printers.remove(i); m.label.printer = 0; }
            ui.add_space(6.0);
            ui.label(RichText::new("ADD PRINTER").size(11.0).color(DIM).monospace());
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut m.label.np_name).hint_text("name").desired_width(100.0));
                egui::ComboBox::from_id_salt("np_conn").width(104.0).selected_text(["Network (RAW 9100)", "USB", "Bluetooth"][m.label.np_conn]).show_ui(ui, |ui| { for (i, n) in ["Network (RAW 9100)", "USB", "Bluetooth"].iter().enumerate() { ui.selectable_value(&mut m.label.np_conn, i, *n); } });
                egui::ComboBox::from_id_salt("np_lang").width(84.0).selected_text(["ZPL", "TSPL", "ESC/POS", "CPCL"][m.label.np_lang]).show_ui(ui, |ui| { for (i, n) in ["ZPL", "TSPL", "ESC/POS", "CPCL"].iter().enumerate() { ui.selectable_value(&mut m.label.np_lang, i, *n); } });
                ui.add(egui::TextEdit::singleline(&mut m.label.np_addr).hint_text("host:9100").desired_width(100.0));
                if button(ui, "Add") && !m.label.np_name.trim().is_empty() {
                    m.printers.push(Printer { name: m.label.np_name.trim().into(), conn: [Conn::Network, Conn::Usb, Conn::Bluetooth][m.label.np_conn], lang: [Lang::Zpl, Lang::Tspl, Lang::EscPos, Lang::Cpcl][m.label.np_lang], addr: m.label.np_addr.clone(), demo: false });
                    m.label.np_name.clear();
                }
            });
            ui.label(dim("Network printers receive the label data directly on port 9100. USB and Bluetooth printers get a raw file in ~/MACH5-labels to send with the OS print spooler. Demo printers only simulate."));
        });
        panel(c.at(1), "Print history", |ui| {
            if m.label_jobs.is_empty() { ui.label(dim("No labels printed yet.")); }
            for j in m.label_jobs.iter().rev().take(8) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(mono(j.ts.clone()).color(FAINT));
                    ui.label(format!("{} spool(s) x{}", j.codes.len(), j.copies));
                    ui.label(dim(format!("{}: {}", j.printer, j.status)));
                });
            }
        });
    });
}

fn usage(ui: &mut Ui, m: &mut Model) {
    panel(ui, "Stock by material", |ui| {
        ui.horizontal(|ui| {
            ui.label(dim("Low-stock threshold per spool"));
            ui.add(DragValue::new(&mut m.low_thr).range(20.0..=500.0).suffix(" g"));
        });
        egui::Grid::new("stock").num_columns(7).striped(true).spacing([20.0, 6.0]).show(ui, |ui| {
            for h in ["Material", "Spools", "Loaded", "In stock", "Total", "Value", "Status"] { th(ui, h); }
            ui.end_row();
            for mat in MATS {
                let all: Vec<&FSpool> = m.fspools.iter().filter(|s| s.mat == mat && s.loc != Loc::Archived).collect();
                if all.is_empty() { continue; }
                let loaded: f32 = all.iter().filter(|s| matches!(s.loc, Loc::Machine(_))).map(|s| s.left).sum();
                let stock: f32 = all.iter().filter(|s| !matches!(s.loc, Loc::Machine(_))).map(|s| s.left).sum();
                let lowc = all.iter().filter(|s| s.left < m.low_thr).count();
                let val: f32 = all.iter().map(|s| s.cost * pct(s)).sum();
                ui.label(mono(mat));
                ui.label(mono(all.len().to_string()));
                ui.label(mono(format!("{:.1} kg", loaded.max(0.0) / 1000.0)));
                ui.label(mono(format!("{:.1} kg", stock.max(0.0) / 1000.0)));
                ui.label(mono(format!("{:.1} kg", (loaded + stock).max(0.0) / 1000.0)));
                ui.label(mono(format!("${:.0}", val)));
                if stock < 1500.0 { status(ui, Status::Warn, "reorder soon"); } else if lowc > 0 { status(ui, Status::Warn, &format!("{} spool(s) low", lowc)); } else { status(ui, Status::Ok, "ok"); }
                ui.end_row();
            }
        });
    });
    adaptive(ui, 2, |c| {
        panel(c.at(0), "Needs attention", |ui| {
            let mut open: Option<u32> = None;
            let list: Vec<(u32, Status, String)> = m.fspools.iter().filter_map(|s| { let (st, tx) = m.spool_status(s); if matches!(st, Status::Ok | Status::Off) { None } else { Some((s.id, st, tx)) } }).collect();
            if list.is_empty() { ui.label(dim("Nothing needs attention.")); }
            for (id, st, tx) in &list {
                let s = &m.fspools[m.spool_idx(*id).unwrap()];
                row_card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(mono(code(*id)).strong());
                        ui.label(format!("{} {}", s.mat, s.color));
                        status(ui, *st, tx);
                    });
                    ui.label(dim(loc_text(m, s)).size(11.5));
                    if sbtn(ui, "Open").clicked() { open = Some(*id); }
                });
            }
            if let Some(id) = open { m.st.sel_spool_id = id; m.st.tab = 0; }
        });
        panel(c.at(1), "Consumption, last 14 days (g/day)", |ui| {
            let today: f32 = m.fspools.iter().map(|s| s.used).sum::<f32>() * 0.0 + m.fleet.iter().filter(|f| f.state == MState::Printing).count() as f32 * 180.0;
            let pts: Vec<[f32; 2]> = (0..14).map(|d| [d as f32, if d == 13 { today } else { 1500.0 + ((d as f32 * 1.7).sin() * 450.0) + ((d * 37 % 11) as f32) * 40.0 }]).collect();
            plot(ui, 160.0, &[Series::new("grams", ACCENT, pts)], (0.0, 13.0), (0.0, 3000.0), "g per day");
            ui.label(dim("Rightmost point is today, from machines printing right now."));
        });
    });
    panel(ui, "Dry-box log (moisture-sensitive materials)", |ui| {
        egui::Grid::new("drylog").num_columns(5).striped(true).spacing([16.0, 5.0]).show(ui, |ui| {
            for h in ["Spool", "Material", "Last dried", "Days", "Location"] { th(ui, h); }
            ui.end_row();
            for s in m.fspools.iter().filter(|s| hygroscopic(&s.mat) && s.loc != Loc::Archived).take(14) {
                ui.label(mono(code(s.id)));
                ui.label(s.mat.clone());
                ui.label(mono(if s.dried.is_empty() { "never".to_string() } else { s.dried.clone() }));
                let days = if s.dried.is_empty() { 999 } else { let v: Vec<i32> = s.dried.split('-').filter_map(|x| x.parse().ok()).collect(); (2026 * 372 + 10 * 31 + 28) - (v[0] * 372 + v[1] * 31 + v[2]) };
                ui.label(mono(if days >= 999 { "-".to_string() } else { days.to_string() }).color(if days > 45 { WARN } else { TEXT }));
                ui.label(dim(loc_text(m, s)));
                ui.end_row();
            }
        });
    });
}

fn scan(ui: &mut Ui, m: &mut Model) {
    panel(ui, "Scan a spool label", |ui| {
        ui.label(dim("Point a USB barcode scanner at a label while this field is focused, or type the code. Scanning selects the spool; then load it on a machine in one step."));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut m.st.scan_text).hint_text("M5-SP-00012  or  mach5:spool:M5-SP-00012").desired_width(320.0).font(FontId::monospace(13.0)));
            let go = (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || button(ui, "Find");
            if go && !m.st.scan_text.trim().is_empty() {
                let t = m.st.scan_text.clone();
                m.st.scan_msg = match m.scan(&t) { Some(id) => format!("Found {}", code(id)), None => "No spool with that code".into() };
                m.st.scan_text.clear();
                r.request_focus();
            }
            if button(ui, "Simulate scan") {
                let ids: Vec<u32> = m.fspools.iter().filter(|s| s.loc == Loc::Shelf).map(|s| s.id).collect();
                if !ids.is_empty() { let id = ids[(m.t as usize * 7) % ids.len()]; m.st.sel_spool_id = id; m.st.scan_msg = format!("Scanned {}", code(id)); }
            }
        });
        if !m.st.scan_msg.is_empty() { ui.label(dim(m.st.scan_msg.clone())); }
    });
    if let Some(si) = m.spool_idx(m.st.sel_spool_id) {
        let id = m.fspools[si].id;
        let mut load: Option<(usize, u8)> = None;
        let mut unload = false;
        panel(ui, &format!("Scanned: {}", code(id)), |ui| {
            let s = &m.fspools[si];
            ui.horizontal(|ui| {
                qr_widget(ui, &qr_payload(id), 90.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("{} {} - {}", s.brand, s.mat, s.color)).size(15.0).strong().color(WHITE));
                    ui.label(mono(format!("{:.0} g remaining  (~{:.0} m)   {}", s.left, length_m(&s.mat, s.dia, s.left), loc_text(m, s))));
                    bar(ui, pct(s), 220.0, None);
                });
            });
            ui.add_space(6.0);
            if matches!(s.loc, Loc::Machine(_)) {
                if button(ui, "Unload to shelf") { unload = true; }
            } else {
                ui.label(dim("Load on a machine:"));
                ui.horizontal_wrapped(|ui| {
                    for (mi, f) in m.fleet.iter().enumerate() {
                        ui.menu_button(&f.name, |ui| {
                            for slot in 0..f.slots {
                                let lbl = match m.spool_on(mi, slot) { Some(x) => format!("T{}  (replace {})", slot, code(m.fspools[x].id)), None => format!("T{}  empty", slot) };
                                if ui.button(lbl).clicked() { load = Some((mi, slot)); ui.close_menu(); }
                            }
                        });
                    }
                });
            }
        });
        if unload { m.unload_spool(id); }
        if let Some((mi, slot)) = load { if let Err(e) = m.load_spool(id, mi, slot) { m.st.scan_msg = e; } else { m.st.scan_msg = format!("Loaded {} on {} T{}", code(id), m.mname(mi), slot); } }
    }
    let _ = (Pos2::ZERO,);
}
