//! The Windows session: chosen WASAPI endpoints and apps as strips, saved between launches.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use audio_program_backend::{
    AppSession, DeviceStream, HardwareDevice, VirtualCableBackend, WasapiBackend,
    APP_CAPTURE_NEEDS, DEVICE_CHANNELS,
};
use audio_program_engine::{Engine, Shared};

use super::{should_reconnect, spawn_clock, Session};
use crate::layout::{app_exe, layout_path, AppChoice, InputStrip, Layout};
use crate::strip::Strip;

const REFRESH: Duration = Duration::from_secs(2);

#[derive(Default)]
pub(super) struct Devices {
    layout: Layout,
    saved: Layout,
    path: Option<PathBuf>,
    inputs: Vec<Slot>,
    outputs: Vec<Slot>,
    present: Vec<String>,
    next_refresh: Option<Instant>,
    app_capture: bool,
    apps: Vec<AppSession>,
    /// The process each input strip was opened for, from [`Layout::app_pids`].
    app_pids: Vec<Option<u32>>,
}

enum Slot {
    Running(DeviceStream),
    Unavailable(String),
    NotRunning,
}

impl Slot {
    fn status(&self) -> Option<String> {
        match self {
            Slot::Running(stream) => stream
                .failure()
                .map(|message| format!("Disconnected. {message}")),
            Slot::Unavailable(message) => Some(format!("Unavailable. {message}")),
            Slot::NotRunning => Some("Not running.".to_string()),
        }
    }

    fn failed(&self) -> bool {
        matches!(self, Slot::Running(stream) if stream.failure().is_some())
    }
}

impl Devices {
    pub(super) fn strips(&self) -> Vec<Strip> {
        let inputs: Vec<_> = self.inputs.iter().map(Slot::status).collect();
        let outputs: Vec<_> = self.outputs.iter().map(Slot::status).collect();
        self.layout.strips(&inputs, &outputs)
    }

    pub(super) fn stop_streams(&mut self) {
        self.inputs.clear();
        self.outputs.clear();
    }

    fn slots(&self) -> impl Iterator<Item = &Slot> {
        self.inputs.iter().chain(&self.outputs)
    }
}

pub(super) fn start_wasapi() -> Session {
    let backend = match WasapiBackend::connect() {
        Ok(backend) => backend,
        Err(err) => return Session::empty(&err.to_string()),
    };
    let app_capture = backend.app_capture_supported();
    let path = layout_path();
    let layout = path.as_deref().map(Layout::load).unwrap_or_default();
    let mut session = Session {
        cables: Vec::new(),
        hardware: Vec::new(),
        error: None,
        shared: None,
        stop: Arc::new(AtomicBool::new(true)),
        clock: None,
        backend: Some(backend),
        devices: Devices {
            saved: layout.clone(),
            layout,
            path,
            app_capture,
            ..Devices::default()
        },
    };
    session.refresh_devices();
    session.rebuild();
    session
}

impl Session {
    /// Devices the picker offers, including strips whose device is unplugged.
    pub fn device_choices(&self) -> Vec<HardwareDevice> {
        self.devices.layout.choices(&self.hardware)
    }

    pub fn uses(&self, device: &HardwareDevice) -> bool {
        self.devices.layout.uses(device)
    }

    /// Make `device` a strip, or remove its strip.
    pub fn toggle_device(&mut self, device: &HardwareDevice) {
        self.capture_layout();
        if self.devices.layout.uses(device) {
            self.devices.layout.remove(device);
        } else if !self.devices.layout.add(device) {
            return;
        }
        self.rebuild();
        self.save_layout();
    }

    /// Whether this Windows can capture an app. The picker shows why not otherwise.
    pub fn app_capture_supported(&self) -> bool {
        self.devices.app_capture
    }

    /// Apps the picker offers: those playing now, then app strips.
    pub fn app_choices(&self) -> Vec<AppChoice> {
        self.devices.layout.app_choices(&self.devices.apps)
    }

    /// Make an app a strip, or remove its strip.
    pub fn toggle_app(&mut self, app: &AppChoice) {
        self.capture_layout();
        if app.used {
            self.devices.layout.remove_app(&app.exe);
        } else if !self.devices.layout.add_app(&app.exe, &app.name) {
            return;
        }
        self.rebuild();
        self.save_layout();
    }

    pub fn set_include_children(&mut self, exe: &str, include: bool) {
        self.capture_layout();
        self.devices.layout.set_include_children(exe, include);
        self.rebuild();
        self.save_layout();
    }

    pub fn set_master(&mut self, output: usize) {
        self.capture_layout();
        self.devices.layout.set_master(output);
        self.rebuild();
        self.save_layout();
    }

    pub fn reconnect(&mut self) {
        self.capture_layout();
        self.refresh_devices();
        self.rebuild();
    }

    /// Called every frame. Saves changes and reconnects devices every couple of seconds.
    pub fn poll(&mut self) {
        let now = Instant::now();
        if self.devices.next_refresh.is_some_and(|next| now < next) {
            return;
        }
        self.devices.next_refresh = Some(now + REFRESH);
        self.capture_layout();
        self.save_layout();
        let changed = self.refresh_devices();
        let failed = self.devices.slots().any(Slot::failed);
        let unavailable = self
            .devices
            .slots()
            .any(|slot| matches!(slot, Slot::Unavailable(_)));
        let apps_moved = self
            .devices
            .layout
            .apps_moved(&self.devices.app_pids, &self.devices.apps);
        if should_reconnect(failed, changed, unavailable) || apps_moved {
            self.rebuild();
        }
    }

    pub(super) fn save_layout(&mut self) {
        self.capture_layout();
        if self.devices.layout == self.devices.saved {
            return;
        }
        let Some(path) = &self.devices.path else {
            return;
        };
        if self.devices.layout.save(path).is_ok() {
            self.devices.saved = self.devices.layout.clone();
        }
    }

    fn capture_layout(&mut self) {
        if let Some(shared) = &self.shared {
            self.devices.layout.capture(shared);
        }
    }

    /// Re-list the endpoints and apps. Returns whether the set of device ids changed.
    fn refresh_devices(&mut self) -> bool {
        let Some(backend) = &self.backend else {
            return false;
        };
        if self.devices.app_capture {
            self.devices.apps = backend.list_apps().unwrap_or_default();
        }
        match backend.list_hardware() {
            Ok(hardware) => {
                self.error = None;
                let ids: Vec<String> = hardware.iter().map(|device| device.id.clone()).collect();
                self.hardware = hardware;
                let changed = ids != self.devices.present;
                self.devices.present = ids;
                changed
            }
            Err(err) => {
                self.error = Some(err.to_string());
                false
            }
        }
    }

    /// Stop every stream, then open the layout's devices into a new engine.
    fn rebuild(&mut self) {
        self.capture_layout();
        self.stop_audio();
        self.shared = None;
        let Some(backend) = &self.backend else {
            return;
        };
        let layout = &self.devices.layout;
        let mut engine = Engine::new();
        let shared = engine.shared();
        let master = layout.master_index();

        let mut outputs: Vec<Slot> = Vec::new();
        for (index, strip) in layout.outputs.iter().enumerate() {
            outputs.push(match backend.open_render(&strip.id, index, &shared) {
                Ok(stream) => {
                    let is_master = master == Some(index);
                    engine.configure_output(index, DEVICE_CHANNELS, stream.rate, is_master);
                    Slot::Running(stream)
                }
                Err(err) => Slot::Unavailable(err.to_string()),
            });
        }
        let app_pids = layout.app_pids(&self.devices.apps);
        let mut inputs: Vec<Slot> = Vec::new();
        for (index, (strip, pid)) in layout.inputs.iter().zip(&app_pids).enumerate() {
            let app_capture = self.devices.app_capture;
            let opened = open_input(backend, strip, *pid, app_capture, index, &shared);
            inputs.push(match opened {
                Some(Ok(stream)) => {
                    engine.configure_input(index, DEVICE_CHANNELS, stream.rate);
                    Slot::Running(stream)
                }
                Some(Err(message)) => Slot::Unavailable(message),
                None => Slot::NotRunning,
            });
        }
        layout.apply(&shared);

        let mut engine = Some(engine);
        if let Some(Slot::Running(stream)) = master.and_then(|index| outputs.get_mut(index)) {
            if let Some(engine) = engine.take() {
                stream.start_as_clock(engine);
            }
        }
        for slot in outputs.iter_mut().chain(inputs.iter_mut()) {
            if let Slot::Running(stream) = slot {
                stream.start();
            }
        }
        if let Some(engine) = engine {
            self.stop = Arc::new(AtomicBool::new(false));
            self.clock = spawn_clock(engine, Arc::clone(&self.stop));
        }
        self.devices.inputs = inputs;
        self.devices.outputs = outputs;
        self.devices.app_pids = app_pids;
        self.shared = Some(shared);
    }
}

/// Open a device strip, or an app strip on its process `pid`. `None` when the app is not running.
fn open_input(
    backend: &WasapiBackend,
    strip: &InputStrip,
    pid: Option<u32>,
    app_capture: bool,
    index: usize,
    shared: &Arc<Shared>,
) -> Option<Result<DeviceStream, String>> {
    let opened = if app_exe(&strip.id).is_none() {
        backend.open_capture(&strip.id, index, shared)
    } else if !app_capture {
        return Some(Err(APP_CAPTURE_NEEDS.to_string()));
    } else {
        backend.open_app_capture(pid?, strip.include_children, index, shared)
    };
    Some(opened.map_err(|err| err.to_string()))
}
