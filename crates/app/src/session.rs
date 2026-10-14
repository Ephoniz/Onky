use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

#[cfg(windows)]
use audio_program_backend::BackendError;
use audio_program_backend::{CableSpec, HardwareDevice, VirtualCable, VirtualCableBackend};
use audio_program_engine::{Engine, Shared, BLOCK_FRAMES, SAMPLE_RATE};

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
            start_wasapi()
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            Self::empty("Audio Program runs on Linux (PipeWire) and Windows (WASAPI).")
        }
    }

    pub fn shared(&self) -> Option<Arc<Shared>> {
        self.shared.clone()
    }

    #[allow(dead_code)]
    fn empty(message: &str) -> Self {
        Self {
            cables: Vec::new(),
            hardware: Vec::new(),
            error: Some(message.to_string()),
            shared: None,
            stop: Arc::new(AtomicBool::new(true)),
            clock: None,
            backend: None,
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(clock) = self.clock.take() {
            let _ = clock.join();
        }
        self.backend.take();
    }
}

#[cfg(target_os = "linux")]
fn start_pipewire() -> Session {
    let mut engine = Engine::new();
    for index in 0..CABLES {
        engine.configure_input(index, 2, SAMPLE_RATE);
        engine.configure_output(index, 2, SAMPLE_RATE, false);
        engine.shared().set_gain(index, 1.0);
        engine.shared().set_route(index, index, true);
    }
    let shared = engine.shared();
    let stop = Arc::new(AtomicBool::new(false));
    let clock_stop = Arc::clone(&stop);
    let clock = std::thread::Builder::new()
        .name("audio-program-clock".to_string())
        .spawn(move || clock_loop(engine, clock_stop))
        .ok();

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
        let input = CableSpec {
            node_name: format!("audio-program-virtual-input-{}", index + 1),
            description: format!("Audio Program Virtual Input {}", index + 1),
            index,
        };
        let output = CableSpec {
            node_name: format!("audio-program-virtual-output-{}", index + 1),
            description: format!("Audio Program Virtual Output {}", index + 1),
            index,
        };
        match backend.create_virtual_input(input, &shared) {
            Ok(cable) => cables.push(cable),
            Err(err) => {
                error = Some(err.to_string());
                break;
            }
        }
        match backend.create_virtual_output(output, &shared) {
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

#[cfg(windows)]
fn start_wasapi() -> Session {
    let backend = match audio_program_backend::WasapiBackend::connect() {
        Ok(backend) => backend,
        Err(err) => return Session::empty(&err.to_string()),
    };
    let hardware = backend.list_hardware().unwrap_or_default();
    let mut backend = backend;
    let shared = Engine::new().shared();
    let probe = CableSpec {
        node_name: "audio-program-virtual-input-1".to_string(),
        description: "Audio Program Virtual Input 1".to_string(),
        index: 0,
    };
    let error = match backend.create_virtual_input(probe, &shared) {
        Ok(_) => None,
        Err(BackendError::VirtualCableUnavailable(message)) => Some(message),
        Err(err) => Some(err.to_string()),
    };
    Session {
        cables: Vec::new(),
        hardware,
        error,
        shared: None,
        stop: Arc::new(AtomicBool::new(true)),
        clock: None,
        backend: Some(backend),
    }
}

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
