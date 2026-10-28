use crate::builder::*;
use crate::data::*;
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, collapsing_header::CollapsingState, pos2, vec2, Align, DragValue, FontId, Layout, RichText, Ui};
use std::collections::HashSet;

enum Act {
    Add(usize),
    Remove(u32),
    Dup(u32),
    Reset(u32),
}

pub fn builder(ui: &mut Ui, m: &mut Model) {
    title(ui, "Config Builder", "Block-based machine config. Elect the modules you need in each system and parameterise them - no long text file to hand-edit");
    let enabled = m.bld.blocks.iter().filter(|b| b.on).count();
    let issues: usize = m.bld.blocks.iter().filter(|b| b.on).map(|b| m.bld.issues(b)).sum();
    let changed: usize = m.bld.blocks.iter().map(|b| m.bld.modified(b)).sum();
    ui.horizontal(|ui| {
        ui.label(mono(format!("{} modules", enabled)));
        ui.label(dim(format!("{} parameters changed from defaults", changed)));
        if issues > 0 { ui.label(RichText::new(format!("{} issues", issues)).color(ERR)); } else { status(ui, Status::Ok, "valid"); }
        ui.add_space(12.0);
        ui.add_enabled_ui(issues == 0, |ui| {
            if primary(ui, "Validate and apply") {
                m.log(Sev::Info, "builder", &format!("{} modules validated against the schema and written with ack and readback", enabled));
                m.bld.import_msg = vec![format!("Applied: {} modules, 0 issues.", enabled)];
            }
        });
    });
    if let Some(l) = m.bld.import_msg.first() {
        if l.starts_with("Applied") { ui.label(dim(l.clone())); }
    }
    ui.add_space(6.0);
    tabs(ui, &mut m.bld.tab, &["Modules", "Generated config", "Import Klipper cfg"]);
    match m.bld.tab {
        0 => modules(ui, m),
        1 => generated(ui, m),
        _ => import(ui, m),
    }
}

fn modules(ui: &mut Ui, m: &mut Model) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut m.bld.search).hint_text("search modules or Klipper sections").desired_width(280.0));
        if button(ui, "Expand all") { m.bld.force_open = Some(true); }
        if button(ui, "Collapse all") { m.bld.force_open = Some(false); }
    });
    ui.add_space(8.0);
    let force = m.bld.force_open.take();
    let q = m.bld.search.to_lowercase();
    let mut acts: Vec<Act> = Vec::new();
    let present: HashSet<&'static str> = {
        let b = &m.bld;
        b.blocks.iter().filter(|x| x.on).map(|x| b.defs[x.def].id).collect()
    };
    let bld = &mut m.bld;
    let defs = &bld.defs;
    let blocks = &mut bld.blocks;

    for (g, (gname, gdesc)) in GROUPS.iter().enumerate() {
        let n_g = blocks.iter().filter(|b| defs[b.def].group == g).count();
        let visible: Vec<usize> = (0..blocks.len()).filter(|&i| {
            let b = &blocks[i];
            let d = &defs[b.def];
            d.group == g && (q.is_empty() || d.title.to_lowercase().contains(&q) || d.fmt.to_lowercase().contains(&q) || b.name.to_lowercase().contains(&q))
        }).collect();
        if !q.is_empty() && visible.is_empty() { continue; }

        let id = ui.make_persistent_id(("bgrp", g));
        let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, g == 0);
        if let Some(f) = force { state.set_open(f); }
        if !q.is_empty() { state.set_open(true); }
        let avail = defs.iter().filter(|d| d.group == g).count();
        let head = state.show_header(ui, |ui| {
            ui.label(RichText::new(*gname).strong().size(15.0).color(WHITE));
            ui.label(dim(format!("{} elected of {} available", n_g, avail)));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.menu_button("+ Add module", |ui| {
                    ui.label(dim(*gdesc));
                    ui.separator();
                    for (di, d) in defs.iter().enumerate().filter(|(_, d)| d.group == g) {
                        let exists = !d.multi && blocks.iter().any(|b| b.def == di);
                        let r = ui.add_enabled(!exists, egui::Button::new(format!("{}    [{}]", d.title, d.fmt)));
                        if r.on_hover_text(d.desc).clicked() {
                            acts.push(Act::Add(di));
                            ui.close_menu();
                        }
                    }
                });
            });
            if g == 0 {
                let r = ui.min_rect();
                set_anchor(ui.ctx(), "bgroup0", egui::Rect::from_min_max(r.min - vec2(26.0, 6.0), pos2(ui.max_rect().right(), r.bottom() + 6.0)));
            }
        });
        head.body(|ui| {
            ui.label(dim(*gdesc));
            ui.add_space(4.0);
            if visible.is_empty() {
                ui.label(dim("No modules elected in this group. Use Add module."));
            }
            for i in visible {
                let d = &defs[blocks[i].def];
                let missing: Vec<&str> = d.requires.iter().copied().filter(|r| !present.contains(r)).collect();
                let bad = d.params.iter().zip(&blocks[i].vals).filter(|(p, v)| invalid(&p.kind, v).is_some()).count() + missing.len();
                let changed = d.params.iter().zip(&blocks[i].vals).filter(|(p, v)| p.def != **v).count();
                let uid = blocks[i].uid;
                let section = if d.fmt.contains("{}") { d.fmt.replace("{}", &blocks[i].name) } else { d.fmt.to_string() };
                let bid = ui.make_persistent_id(("blk", uid));
                let mut bs = CollapsingState::load_with_default_open(ui.ctx(), bid, false);
                if let Some(f) = force { bs.set_open(f); }
                if !q.is_empty() { bs.set_open(true); }
                bs.show_header(ui, |ui| {
                    ui.checkbox(&mut blocks[i].on, "");
                    ui.label(RichText::new(d.title).strong().color(if blocks[i].on { TEXT } else { FAINT }));
                    if d.multi && !blocks[i].name.is_empty() { ui.label(mono(blocks[i].name.clone())); }
                    ui.label(mono(format!("[{}]", section)).color(DIM).size(11.5));
                    if bad > 0 { ui.label(RichText::new(format!("{} issue(s)", bad)).color(ERR)); }
                    else if changed > 0 { ui.label(RichText::new(format!("{} changed", changed)).color(ACCENT).size(12.0)); }
                })
                .body(|ui| {
                    ui.label(dim(d.desc));
                    for r in &missing {
                        ui.horizontal(|ui| { dot(ui, Status::Warn); ui.label(RichText::new(format!("Requires the '{}' module to be elected", r)).color(WARN)); });
                    }
                    if d.multi {
                        ui.horizontal(|ui| {
                            ui.label(dim("Instance name"));
                            ui.add(egui::TextEdit::singleline(&mut blocks[i].name).desired_width(170.0));
                        });
                    }
                    ui.add_space(2.0);
                    egui::Grid::new(("pg", uid)).num_columns(4).striped(true).spacing([16.0, 7.0]).show(ui, |ui| {
                        for (p, v) in d.params.iter().zip(blocks[i].vals.iter_mut()) {
                            let modified = p.def != *v;
                            ui.label(RichText::new(p.label).color(if modified { ACCENT } else { TEXT })).on_hover_text(format!("{}: {}", p.key, p.desc));
                            match (&p.kind, &mut *v) {
                                (PKind::F { min, max, .. }, PVal::F(x)) => { ui.add(DragValue::new(x).speed(((max - min) / 400.0).clamp(0.0005, 50.0)).max_decimals(6)); }
                                (PKind::I { min, max, .. }, PVal::I(x)) => { ui.add(DragValue::new(x).speed(((max - min) as f64 / 400.0).clamp(1.0, 5000.0))); }
                                (PKind::B, PVal::B(x)) => { ui.checkbox(x, ""); }
                                (PKind::S, PVal::S(x)) => {
                                    if x.contains('\n') { ui.add(egui::TextEdit::multiline(x).desired_rows(3).desired_width(200.0).font(FontId::monospace(12.0))); }
                                    else { ui.add(egui::TextEdit::singleline(x).desired_width(200.0)); }
                                }
                                (PKind::C(o), PVal::C(c)) => {
                                    egui::ComboBox::from_id_salt((uid, p.key)).width(200.0).selected_text(o.get(*c).copied().unwrap_or("")).show_ui(ui, |ui| {
                                        for (k, n) in o.iter().enumerate() { ui.selectable_value(c, k, *n); }
                                    });
                                }
                                _ => { ui.label("?"); }
                            }
                            let unit = match &p.kind { PKind::F { unit, .. } | PKind::I { unit, .. } => *unit, _ => "" };
                            ui.label(dim(unit));
                            match invalid(&p.kind, &*v) {
                                Some(msg) => wrap(ui, 420.0, RichText::new(msg).color(ERR)),
                                None => wrap(ui, 420.0, dim(p.desc)),
                            }
                            ui.end_row();
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if button(ui, "Reset to defaults") { acts.push(Act::Reset(uid)); }
                        if d.multi && button(ui, "Duplicate") { acts.push(Act::Dup(uid)); }
                        if button(ui, "Remove module") { acts.push(Act::Remove(uid)); }
                    });
                    ui.add_space(6.0);
                });
            }
        });
        ui.add_space(4.0);
        ui.separator();
    }

    for a in acts {
        match a {
            Act::Add(d) => {
                let name = if m.bld.defs[d].multi { "new".to_string() } else { String::new() };
                m.bld.add(d, &name);
            }
            Act::Remove(u) => m.bld.blocks.retain(|b| b.uid != u),
            Act::Dup(u) => m.bld.dup(u),
            Act::Reset(u) => m.bld.reset(u),
        }
    }
}

fn generated(ui: &mut Ui, m: &mut Model) {
    let text = m.bld.generate();
    panel(ui, "Generated config (read-only, Klipper-style)", |ui| {
        ui.horizontal(|ui| {
            if primary(ui, "Copy to clipboard") { ui.ctx().copy_text(text.clone()); }
            ui.label(dim(format!("{} lines. This is what the blocks compile to; the blocks stay the source of truth.", text.lines().count())));
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical().max_height(560.0).auto_shrink([false, true]).show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut text.as_str()).desired_width(f32::INFINITY).desired_rows(30).font(FontId::monospace(12.5)));
        });
    });
}

fn import(ui: &mut Ui, m: &mut Model) {
    panel(ui, "Import a Klipper printer.cfg", |ui| {
        ui.label(dim("Paste a config. Each [section] becomes a block; keys that match a module parameter are filled in. Pin options (step_pin, heater_pin ...) are replaced by node bindings; unknown sections and keys are listed below."));
        ui.add_space(4.0);
        ui.add(egui::TextEdit::multiline(&mut m.bld.import_text).desired_width(f32::INFINITY).desired_rows(16).font(FontId::monospace(12.5)));
        ui.horizontal(|ui| {
            ui.checkbox(&mut m.bld.replace, "Replace current blocks");
            if primary(ui, "Import") {
                let t = m.bld.import_text.clone();
                m.bld.import(&t);
                m.bld.tab = 0;
                let n = m.bld.blocks.len();
                m.log(Sev::Info, "builder", &format!("Imported Klipper config: {} blocks", n));
            }
            if button(ui, "Load sample") { m.bld.import_text = SAMPLE_CFG.into(); }
            if button(ui, "Clear") { m.bld.import_text.clear(); }
        });
        for l in &m.bld.import_msg { ui.label(mono(l.clone()).color(DIM)); }
    });
}
