use crate::data::{Model, Page};
use eframe::egui::Ui;

mod builder;
mod control;
mod dashboard;
mod farm;
mod hardware;
mod materials;
mod system;
mod tuning;

pub fn show(ui: &mut Ui, page: Page, m: &mut Model) {
    match page {
        Page::Dashboard => dashboard::dashboard(ui, m),
        Page::Control => control::control(ui, m),
        Page::Print => control::print(ui, m),
        Page::Thermal => control::thermal(ui, m),
        Page::Motion => tuning::motion(ui, m),
        Page::Calibration => tuning::calibration(ui, m),
        Page::Probe => tuning::probe(ui, m),
        Page::Drives => tuning::drives(ui, m),
        Page::Materials => materials::filament(ui, m),
        Page::Camera => materials::camera(ui, m),
        Page::CanBus => hardware::can_bus(ui, m),
        Page::Nodes => hardware::nodes(ui, m),
        Page::Firmware => hardware::firmware(ui, m),
        Page::Builder => builder::builder(ui, m),
        Page::Config => system::config(ui, m),
        Page::Rules => system::rules(ui, m),
        Page::Api => system::api(ui, m),
        Page::Fleet => farm::fleet(ui, m),
        Page::Simulator => system::simulator(ui, m),
        Page::Spec => system::spec(ui, m),
        Page::Logs => system::logs(ui, m),
    }
}
