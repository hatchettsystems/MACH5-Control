use crate::data::{Model, Page};
use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, Ui};

mod builder;
mod control;
mod dashboard;
mod farm;
mod hardware;
mod materials;
mod print;

pub fn print_file_color(n: &str) -> &'static str { print::file_color(n) }
mod spools;
mod system;
mod tuning;

/// Draw a merged page: its title, then each section stacked on one scrolling page.
pub fn show(ui: &mut Ui, page: Page, m: &mut Model) {
    let secs = page.tabs();
    if secs.is_empty() {
        leaf(ui, page, m);
        return;
    }
    ui.label(egui::RichText::new(page.nav_title()).size(20.0).strong().color(WHITE));
    ui.label(dim(page.hub_blurb()));
    ui.add_space(4.0);
    for (i, (sub, name)) in secs.iter().enumerate() {
        ui.add_space(if i == 0 { 4.0 } else { 14.0 });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(*name).size(16.0).strong().color(WHITE));
        });
        let y = ui.cursor().top() + 1.0;
        ui.painter().line_segment([egui::pos2(ui.min_rect().left(), y), egui::pos2(ui.max_rect().right(), y)], egui::Stroke::new(1.0_f32, LINE2));
        ui.add_space(8.0);
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("in_hub"), true));
        leaf(ui, *sub, m);
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("in_hub"), false));
    }
}

fn leaf(ui: &mut Ui, page: Page, m: &mut Model) {
    match page {
        Page::Dashboard => dashboard::dashboard(ui, m),
        Page::Print => print::print(ui, m),
        Page::Thermal => control::thermal(ui, m),
        Page::Motion => tuning::motion(ui, m),
        Page::Calibration => tuning::calibration(ui, m),
        Page::Probe => tuning::probe(ui, m),
        Page::Drives => tuning::drives(ui, m),
        Page::Materials => materials::filament(ui, m),
        Page::Spools => spools::spools(ui, m),
        Page::Camera => materials::camera(ui, m),
        Page::CanBus => hardware::can_bus(ui, m),
        Page::Nodes => hardware::nodes(ui, m),
        Page::Firmware => hardware::firmware(ui, m),
        Page::Config => system::config(ui, m),
        Page::Rules => system::rules(ui, m),
        Page::Builder => builder::builder(ui, m),
        Page::Api => system::api(ui, m),
        Page::Fleet => farm::fleet(ui, m),
        Page::Simulator => system::simulator(ui, m),
        Page::Spec => system::spec(ui, m),
        Page::Logs => system::logs(ui, m),
    }
}
