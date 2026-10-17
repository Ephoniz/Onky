//! The strips the user picked from the device list, and their settings.
//!
//! Saved as JSON so the next launch restores the same mixer. Routes name output
//! devices by id, so they survive strips being added or removed. An app strip is saved by
//! its executable name, so it comes back when the app runs again.

use std::path::{Path, PathBuf};

use audio_program_backend::{AppSession, HardwareDevice};
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
    /// App strips only: also capture the processes the app started.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_children: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputStrip {
    pub id: String,
    pub name: String,
}

/// An app the picker offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppChoice {
    pub exe: String,
    pub name: String,
    pub used: bool,
    pub include_children: bool,
}

const APP_PREFIX: &str = "app:";

/// The executable an app strip captures. `None` for a device strip.
pub fn app_exe(id: &str) -> Option<&str> {
    id.strip_prefix(APP_PREFIX)
}

/// The process to capture for `exe`: one playing now if there is one.
pub fn app_process(apps: &[AppSession], exe: &str) -> Option<u32> {
    apps.iter()
        .filter(|app| app.exe == exe)
        .max_by_key(|app| app.playing)
        .map(|app| app.pid)
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
            self.inputs.push(input_strip(&device.id, &device.name));
        }
        true
    }

    pub fn uses_app(&self, exe: &str) -> bool {
        self.app_strip(exe).is_some()
    }

    /// Add a strip that captures `exe`. Returns false when it is already a strip or the engine is full.
    pub fn add_app(&mut self, exe: &str, name: &str) -> bool {
        if self.uses_app(exe) || self.inputs.len() >= MAX_INPUTS {
            return false;
        }
        self.inputs
            .push(input_strip(&format!("{APP_PREFIX}{exe}"), name));
        true
    }

    pub fn remove_app(&mut self, exe: &str) {
        self.inputs.retain(|strip| app_exe(&strip.id) != Some(exe));
    }

    pub fn set_include_children(&mut self, exe: &str, include: bool) {
        if let Some(strip) = self.app_strip_mut(exe) {
            strip.include_children = include;
        }
    }

    /// Apps to offer: those playing now, then app strips whose app is quiet or not running.
    pub fn app_choices(&self, apps: &[AppSession]) -> Vec<AppChoice> {
        let mut choices: Vec<AppChoice> = Vec::new();
        for app in apps.iter().filter(|app| app.playing) {
            if !choices.iter().any(|choice| choice.exe == app.exe) {
                choices.push(self.app_choice(&app.exe, app.name.clone()));
            }
        }
        for strip in &self.inputs {
            let Some(exe) = app_exe(&strip.id) else {
                continue;
            };
            if choices.iter().any(|choice| choice.exe == exe) {
                continue;
            }
            let name = if app_process(apps, exe).is_some() {
                strip.name.clone()
            } else {
                format!("{} (not running)", strip.name)
            };
            choices.push(self.app_choice(exe, name));
        }
        choices
    }

    /// The process each input strip captures: `Some` for an app strip whose app is running.
    pub fn app_pids(&self, apps: &[AppSession]) -> Vec<Option<u32>> {
        self.inputs
            .iter()
            .map(|strip| app_exe(&strip.id).and_then(|exe| app_process(apps, exe)))
            .collect()
    }

    /// Whether app strips must reopen: an app started, or a process they captured is gone.
    ///
    /// `opened` is what [`Layout::app_pids`] returned when the strips were opened.
    pub fn apps_moved(&self, opened: &[Option<u32>], apps: &[AppSession]) -> bool {
        self.inputs.iter().zip(opened).any(|(strip, opened)| {
            let Some(exe) = app_exe(&strip.id) else {
                return false;
            };
            match opened {
                Some(pid) => !apps.iter().any(|app| app.pid == *pid),
                None => app_process(apps, exe).is_some(),
            }
        })
    }

    fn app_choice(&self, exe: &str, name: String) -> AppChoice {
        let strip = self.app_strip(exe);
        AppChoice {
            exe: exe.to_string(),
            name,
            used: strip.is_some(),
            include_children: strip.is_some_and(|strip| strip.include_children),
        }
    }

    fn app_strip(&self, exe: &str) -> Option<&InputStrip> {
        self.inputs
            .iter()
            .find(|strip| app_exe(&strip.id) == Some(exe))
    }

    fn app_strip_mut(&mut self, exe: &str) -> Option<&mut InputStrip> {
        self.inputs
            .iter_mut()
            .find(|strip| app_exe(&strip.id) == Some(exe))
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
            kind: if app_exe(&strip.id).is_some() {
                "App"
            } else {
                "Input device"
            },
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
            .filter(|strip| app_exe(&strip.id).is_none())
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

fn input_strip(id: &str, name: &str) -> InputStrip {
    InputStrip {
        id: id.to_string(),
        name: name.to_string(),
        gain: 1.0,
        muted: false,
        routes: Vec::new(),
        include_children: false,
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

#[cfg(test)]
#[path = "layout_app_tests.rs"]
mod app_tests;
