use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
#[cfg(any(target_os = "linux", windows))]
use std::time::Duration;

#[cfg(any(test, target_os = "linux"))]
use audio_program_backend::CableSpec;
#[cfg(target_os = "linux")]
use audio_program_backend::VirtualCableBackend;
use audio_program_backend::{HardwareDevice, VirtualCable};
#[cfg(any(target_os = "linux", windows))]
use audio_program_engine::BLOCK_FRAMES;
#[cfg(any(test, target_os = "linux", windows))]
use audio_program_engine::SAMPLE_RATE;
use audio_program_engine::{Engine, Shared};

use crate::strip::Strip;

#[cfg(windows)]
#[path = "session_wasapi.rs"]
mod wasapi;

#[cfg(any(test, target_os = "linux"))]
const CABLES: usize = 2;

pub struct Session {
    pub cables: Vec<VirtualCable>,
    pub hardware: Vec<HardwareDevice>,
    pub error: Option<String>,
    shared: Option<Arc<Shared>>,
    stop: Arc<AtomicBool>,
    clock: Option<JoinHandle<()>>,
    #[cfg(target_os = "linux")]
    backend: Option<audio_program_backend::PipeWireBackend>,
    #[cfg(windows)]
    backend: Option<audio_program_backend::WasapiBackend>,
    #[cfg(windows)]
    devices: wasapi::Devices,
    #[cfg(not(any(target_os = "linux", windows)))]
    backend: Option<()>,
}

impl Session {
    pub fn start() -> Self {
        #[cfg(target_os = "linux")]
        {
            start_pipewire()
        }
        #[cfg(windows)]
        {
            wasapi::start_wasapi()
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            Self::empty("Audio Program runs on Linux (PipeWire) and Windows (WASAPI).")
        }
    }

    pub fn shared(&self) -> Option<Arc<Shared>> {
        self.shared.clone()
    }

    pub fn strips(&self) -> Vec<Strip> {
        #[cfg(windows)]
        {
            self.devices.strips()
        }
        #[cfg(not(windows))]
        {
            crate::strip::cable_strips(&self.cables)
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn empty(message: &str) -> Self {
        Self {
            cables: Vec::new(),
            hardware: Vec::new(),
            error: Some(message.to_string()),
            shared: None,
            stop: Arc::new(AtomicBool::new(true)),
            clock: None,
            backend: None,
            #[cfg(windows)]
            devices: wasapi::Devices::default(),
        }
    }

    fn stop_audio(&mut self) {
        #[cfg(windows)]
        self.devices.stop_streams();
        self.stop.store(true, Ordering::Relaxed);
        if let Some(clock) = self.clock.take() {
            let _ = clock.join();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        #[cfg(windows)]
        self.save_layout();
        self.stop_audio();
        self.backend.take();
    }
}

/// Reopen every device after a stream failed, or after the device list changed
/// while a strip had no device.
#[cfg(any(test, windows))]
fn should_reconnect(stream_failed: bool, devices_changed: bool, strip_unavailable: bool) -> bool {
    stream_failed || (devices_changed && strip_unavailable)
}

#[cfg(target_os = "linux")]
fn start_pipewire() -> Session {
    let mut engine = Engine::new();
    configure_default(&mut engine);
    let shared = engine.shared();
    let stop = Arc::new(AtomicBool::new(false));
    let clock = spawn_clock(engine, Arc::clone(&stop));

    let mut backend = match audio_program_backend::PipeWireBackend::connect() {
        Ok(backend) => backend,
        Err(err) => {
            return Session {
                cables: Vec::new(),
                hardware: Vec::new(),
                error: Some(err.to_string()),
                shared: None,
                stop,
                clock,
                backend: None,
            };
        }
    };
    let hardware = backend.list_hardware().unwrap_or_default();
    let mut cables = Vec::new();
    let mut error = None;
    for index in 0..CABLES {
        match backend.create_virtual_input(cable_spec(index, false), &shared) {
            Ok(cable) => cables.push(cable),
            Err(err) => {
                error = Some(err.to_string());
                break;
            }
        }
        match backend.create_virtual_output(cable_spec(index, true), &shared) {
            Ok(cable) => cables.push(cable),
            Err(err) => {
                error = Some(err.to_string());
                break;
            }
        }
    }
    Session {
        cables,
        hardware,
        error,
        shared: Some(shared),
        stop,
        clock,
        backend: Some(backend),
    }
}

#[cfg(any(test, target_os = "linux"))]
fn configure_default(engine: &mut Engine) {
    for index in 0..CABLES {
        engine.configure_input(index, 2, SAMPLE_RATE);
        engine.configure_output(index, 2, SAMPLE_RATE, false);
        engine.shared().set_gain(index, 1.0);
        engine.shared().set_route(index, index, true);
    }
}

#[cfg(any(test, target_os = "linux"))]
fn cable_spec(index: usize, output: bool) -> CableSpec {
    let (kind, title) = if output {
        ("output", "Output")
    } else {
        ("input", "Input")
    };
    CableSpec {
        node_name: format!("audio-program-virtual-{kind}-{}", index + 1),
        description: format!("Audio Program Virtual {title} {}", index + 1),
        index,
    }
}

/// Run the engine on a steady internal clock, for when no output device is the master.
#[cfg(any(target_os = "linux", windows))]
fn spawn_clock(engine: Engine, stop: Arc<AtomicBool>) -> Option<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("audio-program-clock".to_string())
        .spawn(move || clock_loop(engine, stop))
        .ok()
}

#[cfg(any(target_os = "linux", windows))]
fn clock_loop(mut engine: Engine, stop: Arc<AtomicBool>) {
    let period = Duration::from_nanos(1_000_000_000 * (BLOCK_FRAMES as u64) / (SAMPLE_RATE as u64));
    while !stop.load(Ordering::Relaxed) {
        let started = std::time::Instant::now();
        engine.process();
        let remaining = period.saturating_sub(started.elapsed());
        if !remaining.is_zero() {
            std::thread::sleep(remaining);
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "reconnect_tests.rs"]
mod reconnect_tests;
