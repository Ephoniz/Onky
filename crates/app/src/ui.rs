use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::session::Session;
use crate::strip::Strip;

const INK: Color32 = Color32::from_rgb(243, 239, 230);
const MUTED: Color32 = Color32::from_rgb(176, 166, 150);
const PANEL: Color32 = Color32::from_rgb(38, 34, 28);
const AMBER: Color32 = Color32::from_rgb(224, 164, 90);
const ALERT: Color32 = Color32::from_rgb(196, 92, 74);

pub fn install_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(22, 20, 17);
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(16, 14, 12);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(54, 48, 40);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(72, 62, 48);
    visuals.widgets.active.bg_fill = AMBER;
    visuals.selection.bg_fill = AMBER;
    visuals.override_text_color = Some(INK);
    ctx.set_visuals(visuals);
}

pub fn draw(ui: &mut egui::Ui, session: &mut Session, updates: Option<&mut bool>) {
    egui::Panel::top("bar").show(ui, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Audio Program")
                    .size(22.0)
                    .strong()
                    .color(INK),
            );
            ui.label(RichText::new("48 kHz · 10 ms").color(MUTED));
            if let Some(enabled) = updates {
                ui.checkbox(enabled, "Check for updates");
            }
        });
        ui.add_space(8.0);
    });

    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| mixer(ui, session));
    });
}

fn mixer(ui: &mut egui::Ui, session: &mut Session) {
    if let Some(error) = &session.error {
        alert(ui, error);
        ui.add_space(12.0);
    }
    let Some(shared) = session.shared() else {
        if session.error.is_none() {
            ui.label("No virtual cables are open.");
        }
        hardware(ui, session);
        return;
    };
    let strips = session.strips();

    ui.label(RichText::new("Strips").color(MUTED));
    ui.add_space(6.0);
    if strips.is_empty() {
        ui.label(RichText::new("Tick input and output devices below to add strips.").color(MUTED));
    }
    let mut new_master = None;
    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.horizontal(|ui| {
            for item in &strips {
                if strip(ui, &shared, item) {
                    new_master = Some(item.index);
                }
            }
        });
    });
    if let Some(output) = new_master {
        set_master(session, output);
    }

    ui.add_space(16.0);
    ui.label(RichText::new("Routing").color(MUTED));
    ui.add_space(6.0);
    routing(ui, &shared, &strips);
    hardware(ui, session);
}

fn alert(ui: &mut egui::Ui, message: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(48, 28, 24))
        .stroke(Stroke::new(1.0, ALERT))
        .inner_margin(12.0)
        .corner_radius(6.0)
        .show(ui, |ui| {
            ui.label(RichText::new("Audio backend").color(ALERT).strong());
            ui.label(message);
        });
}

/// Draw one strip. Returns true when the user picks this output as the master clock.
fn strip(ui: &mut egui::Ui, shared: &audio_program_engine::Shared, item: &Strip) -> bool {
    let mut picked_master = false;
    egui::Frame::new()
        .fill(PANEL)
        .inner_margin(10.0)
        .corner_radius(6.0)
        .show(ui, |ui| {
            ui.set_min_width(148.0);
            ui.set_max_width(180.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&item.name).size(13.0));
                ui.label(RichText::new(item.kind).size(11.0).color(MUTED));
                if let Some(status) = &item.status {
                    ui.label(RichText::new(status).size(11.0).color(ALERT));
                }
                ui.add_space(8.0);
                if item.output {
                    meter(ui, shared.peak_output(item.index));
                    ui.label(RichText::new("fed by the matrix").size(11.0).color(MUTED));
                    if let Some(master) = item.master {
                        picked_master = ui.radio(master, "Master clock").clicked() && !master;
                    }
                } else {
                    let mut gain = shared.gain(item.index);
                    let mut muted = shared.muted(item.index);
                    ui.add(
                        egui::Slider::new(&mut gain, 0.0..=1.5)
                            .vertical()
                            .show_value(false)
                            .text("gain"),
                    );
                    shared.set_gain(item.index, gain);
                    meter(ui, shared.peak_input(item.index));
                    if ui.toggle_value(&mut muted, "Mute").changed() {
                        shared.set_mute(item.index, muted);
                    }
                }
            });
        });
    picked_master
}

fn meter(ui: &mut egui::Ui, peak: f32) {
    let fraction = peak.clamp(0.0, 1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 120.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::from_rgb(16, 14, 12));
    let mut filled = rect;
    filled.set_top(rect.bottom() - rect.height() * fraction);
    painter.rect_filled(filled, 2.0, AMBER);
}

fn routing(ui: &mut egui::Ui, shared: &audio_program_engine::Shared, strips: &[Strip]) {
    let inputs: Vec<_> = strips.iter().filter(|item| !item.output).collect();
    let outputs: Vec<_> = strips.iter().filter(|item| item.output).collect();
    if inputs.is_empty() || outputs.is_empty() {
        ui.label(RichText::new("Routing appears when both inputs and outputs exist.").color(MUTED));
        return;
    }
    egui::Grid::new("matrix")
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("");
            for output in &outputs {
                ui.label(RichText::new(short(&output.name)).size(12.0).color(MUTED));
            }
            ui.end_row();
            for input in &inputs {
                ui.label(short(&input.name));
                for output in &outputs {
                    let mut on = shared.routed(input.index, output.index);
                    if ui.checkbox(&mut on, "").changed() {
                        shared.set_route(input.index, output.index, on);
                    }
                }
                ui.end_row();
            }
        });
}

#[cfg(not(windows))]
fn set_master(_session: &mut Session, _output: usize) {}

#[cfg(windows)]
fn set_master(session: &mut Session, output: usize) {
    session.set_master(output);
}

#[cfg(not(windows))]
fn hardware(ui: &mut egui::Ui, session: &mut Session) {
    if session.hardware.is_empty() {
        return;
    }
    ui.add_space(16.0);
    ui.label(RichText::new("Hardware seen by the backend").color(MUTED));
    for device in &session.hardware {
        let kind = if device.output { "output" } else { "input" };
        ui.label(format!("{} · {kind}", device.name));
    }
}

/// The device picker: each ticked device is a strip.
#[cfg(windows)]
fn hardware(ui: &mut egui::Ui, session: &mut Session) {
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Devices").color(MUTED));
        if ui.button("Reconnect").clicked() {
            session.reconnect();
        }
    });
    ui.label(
        RichText::new("Tick a device to make it a strip. Cables such as VB-Cable are devices too.")
            .size(11.0)
            .color(MUTED),
    );
    ui.add_space(6.0);
    let choices = session.device_choices();
    let mut toggled = None;
    ui.columns(2, |columns| {
        for (column, output, title) in [
            (0, false, "Inputs (record from)"),
            (1, true, "Outputs (play to)"),
        ] {
            let ui = &mut columns[column];
            ui.label(RichText::new(title).strong());
            for device in choices.iter().filter(|device| device.output == output) {
                let mut on = session.uses(device);
                if ui.checkbox(&mut on, &device.name).changed() {
                    toggled = Some(device.clone());
                }
            }
        }
    });
    if let Some(device) = toggled {
        session.toggle_device(&device);
    }
}

fn short(description: &str) -> String {
    description.trim_start_matches("Audio Program ").to_string()
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;
