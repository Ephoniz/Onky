//! The strips the user picked from the device list, and their settings.
//!
//! Saved as JSON so the next launch restores the same mixer. Routes name output
//! devices by id, so they survive strips being added or removed.

use std::path::{Path, PathBuf};

use audio_program_backend::HardwareDevice;
use audio_program_engine::{Shared, MAX_INPUTS, MAX_OUTPUTS};
use serde::{Deserialize, Serialize};

use crate::strip::Strip;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    #[serde(default)]
    pub inputs: Vec<InputStrip>,
    #[serde(default)]
    pub outputs: Vec<OutputStrip>,
    /// Output device id that clocks the mixer. The first output when unset.
    #[serde(default)]
    pub master: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputStrip {
    pub id: String,
    pub name: String,
    pub gain: f32,
    pub muted: bool,
    #[serde(default)]
    pub routes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputStrip {
    pub id: String,
    pub name: String,
}

impl Layout {
    pub fn uses(&self, device: &HardwareDevice) -> bool {
        if device.output {
            self.outputs.iter().any(|strip| strip.id == device.id)
        } else {
            self.inputs.iter().any(|strip| strip.id == device.id)
        }
    }

    /// Add a strip for `device`. Returns false when it is already a strip or the engine is full.
    pub fn add(&mut self, device: &HardwareDevice) -> bool {
        if self.uses(device) {
            return false;
        }
        if device.output {
            if self.outputs.len() >= MAX_OUTPUTS {
                return false;
            }
            self.outputs.push(OutputStrip {
                id: device.id.clone(),
                name: device.name.clone(),
            });
        } else {
            if self.inputs.len() >= MAX_INPUTS {
                return false;
            }
            self.inputs.push(InputStrip {
                id: device.id.clone(),
                name: device.name.clone(),
                gain: 1.0,
                muted: false,
                routes: Vec::new(),
            });
        }
        true
    }

    pub fn remove(&mut self, device: &HardwareDevice) {
        if !device.output {
            self.inputs.retain(|strip| strip.id != device.id);
            return;
        }
        self.outputs.retain(|strip| strip.id != device.id);
        for input in &mut self.inputs {
            input.routes.retain(|id| *id != device.id);
        }
        if self.master.as_deref() == Some(device.id.as_str()) {
            self.master = None;
        }
    }

    pub fn master_index(&self) -> Option<usize> {
        let chosen = self
            .master
            .as_ref()
            .and_then(|id| self.outputs.iter().position(|strip| strip.id == *id));
        chosen.or(if self.outputs.is_empty() {
            None
        } else {
            Some(0)
        })
    }

    pub fn set_master(&mut self, output: usize) {
        if let Some(strip) = self.outputs.get(output) {
            self.master = Some(strip.id.clone());
        }
    }

    /// Write gain, mute, and routes into the engine. Strip `n` is engine input or output `n`.
    pub fn apply(&self, shared: &Shared) {
        for (input, strip) in self.inputs.iter().enumerate() {
            shared.set_gain(input, strip.gain);
            shared.set_mute(input, strip.muted);
            for (output, target) in self.outputs.iter().enumerate() {
                shared.set_route(input, output, strip.routes.contains(&target.id));
            }
        }
    }

    /// Read back what the window changed in the engine.
    pub fn capture(&mut self, shared: &Shared) {
        for (input, strip) in self.inputs.iter_mut().enumerate() {
            strip.gain = shared.gain(input);
            strip.muted = shared.muted(input);
            strip.routes = self
                .outputs
                .iter()
                .enumerate()
                .filter(|(output, _)| shared.routed(input, *output))
                .map(|(_, target)| target.id.clone())
                .collect();
        }
    }

    /// Strips for the window. `status` holds a problem to show for each input, then each output.
    pub fn strips(
        &self,
        input_status: &[Option<String>],
        output_status: &[Option<String>],
    ) -> Vec<Strip> {
        let master = self.master_index();
        let inputs = self.inputs.iter().enumerate().map(|(index, strip)| Strip {
            name: strip.name.clone(),
            kind: "Input device",
            index,
            output: false,
            status: input_status.get(index).cloned().flatten(),
            master: None,
        });
        let outputs = self.outputs.iter().enumerate().map(|(index, strip)| Strip {
            name: strip.name.clone(),
            kind: "Output device",
            index,
            output: true,
            status: output_status.get(index).cloned().flatten(),
            master: Some(master == Some(index)),
        });
        inputs.chain(outputs).collect()
    }

    /// Devices to offer in the picker: what Windows lists now, then strips whose device is gone.
    pub fn choices(&self, present: &[HardwareDevice]) -> Vec<HardwareDevice> {
        let saved = self
            .inputs
            .iter()
            .map(|strip| (&strip.id, &strip.name, false))
            .chain(
                self.outputs
                    .iter()
                    .map(|strip| (&strip.id, &strip.name, true)),
            );
        let missing: Vec<HardwareDevice> = saved
            .filter(|(id, _, output)| {
                !present
                    .iter()
                    .any(|device| device.id == **id && device.output == *output)
            })
            .map(|(id, name, output)| HardwareDevice {
                id: id.clone(),
                name: format!("{name} (not connected)"),
                output,
            })
            .collect();
        present.iter().cloned().chain(missing).collect()
    }

    pub fn from_json(text: &str) -> Option<Self> {
        let mut layout: Self = serde_json::from_str(text).ok()?;
        layout.inputs.truncate(MAX_INPUTS);
        layout.outputs.truncate(MAX_OUTPUTS);
        for strip in &mut layout.inputs {
            if !strip.gain.is_finite() {
                strip.gain = 1.0;
            }
        }
        Some(layout)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// The saved layout, or an empty one when there is none or it cannot be read.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| Self::from_json(&text))
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_json())
    }
}

/// `%APPDATA%\Audio Program\session.json`.
pub fn layout_path() -> Option<PathBuf> {
    let root = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(root)
            .join("Audio Program")
            .join("session.json"),
    )
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
