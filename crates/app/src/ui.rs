use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::session::Session;

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

pub fn draw(ui: &mut egui::Ui, session: &mut Session) {
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
        });
        ui.add_space(8.0);
    });

    egui::CentralPanel::default().show(ui, |ui| {
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

        ui.label(RichText::new("Strips").color(MUTED));
        ui.add_space(6.0);
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for cable in session.cables.clone() {
                    strip(ui, &shared, &cable.description, cable.index, cable.output);
                }
            });
        });

        ui.add_space(16.0);
        ui.label(RichText::new("Routing").color(MUTED));
        ui.add_space(6.0);
        routing(ui, &shared, session);
        hardware(ui, session);
    });
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

fn strip(
    ui: &mut egui::Ui,
    shared: &audio_program_engine::Shared,
    name: &str,
    index: usize,
    output: bool,
) {
    egui::Frame::new()
        .fill(PANEL)
        .inner_margin(10.0)
        .corner_radius(6.0)
        .show(ui, |ui| {
            ui.set_min_width(148.0);
            ui.set_max_width(180.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(name).size(13.0));
                let kind = if output {
                    "Virtual output"
                } else {
                    "Virtual input"
                };
                ui.label(RichText::new(kind).size(11.0).color(MUTED));
                ui.add_space(8.0);
                if output {
                    meter(ui, shared.peak_output(index));
                    ui.label(RichText::new("fed by the matrix").size(11.0).color(MUTED));
                } else {
                    let mut gain = shared.gain(index);
                    let mut muted = shared.muted(index);
                    ui.add(
                        egui::Slider::new(&mut gain, 0.0..=1.5)
                            .vertical()
                            .show_value(false)
                            .text("gain"),
                    );
                    shared.set_gain(index, gain);
                    meter(ui, shared.peak_input(index));
                    if ui.toggle_value(&mut muted, "Mute").changed() {
                        shared.set_mute(index, muted);
                    }
                }
            });
        });
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

fn routing(ui: &mut egui::Ui, shared: &audio_program_engine::Shared, session: &Session) {
    let inputs: Vec<_> = session
        .cables
        .iter()
        .filter(|cable| !cable.output)
        .cloned()
        .collect();
    let outputs: Vec<_> = session
        .cables
        .iter()
        .filter(|cable| cable.output)
        .cloned()
        .collect();
    if inputs.is_empty() || outputs.is_empty() {
        ui.label(RichText::new("Routing appears when both inputs and outputs exist.").color(MUTED));
        return;
    }
    egui::Grid::new("matrix")
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("");
            for output in &outputs {
                ui.label(
                    RichText::new(short(&output.description))
                        .size(12.0)
                        .color(MUTED),
                );
            }
            ui.end_row();
            for input in &inputs {
                ui.label(short(&input.description));
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

fn hardware(ui: &mut egui::Ui, session: &Session) {
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

fn short(description: &str) -> String {
    description.trim_start_matches("Audio Program ").to_string()
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;
