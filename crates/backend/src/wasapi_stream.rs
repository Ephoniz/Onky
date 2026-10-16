//! WASAPI shared-mode streams, one thread per endpoint.
//!
//! A stream thread opens its endpoint, reports the rate, then waits for `start`.
//! The thread of the master output also owns the engine and mixes ahead of its device.
//! Every other device runs through the engine's adaptive resampler.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use audio_program_engine::{Engine, Shared};
use wasapi::{
    initialize_mta, AudioClient, DeviceEnumerator, Direction, Handle, SampleType, StreamMode,
    WasapiError, WaveFormat,
};

use crate::{
    device_error_text, fill_render, mix_ahead, push_capture, stream_rate, BackendError,
    DEVICE_CHANNELS,
};

const FRAME_BYTES: usize = DEVICE_CHANNELS * std::mem::size_of::<f32>();
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);
const WAIT_MS: u32 = 200;

/// An initialized client, before its stream starts.
///
/// Endpoints come from [`open_endpoint`]. A process-loopback client can be opened the same way
/// and handed to the capture loop.
struct Opened {
    client: AudioClient,
    event: Handle,
    rate: u32,
    buffer_frames: usize,
}

type Start = Option<Engine>;

/// A running device thread. Dropping it stops the device and joins the thread.
pub struct DeviceStream {
    pub rate: u32,
    start: Option<SyncSender<Start>>,
    stop: Arc<AtomicBool>,
    failure: Arc<Mutex<Option<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl DeviceStream {
    pub fn start(&mut self) {
        self.send_start(None);
    }

    /// Start a render stream that also owns the engine and paces it.
    pub fn start_as_clock(&mut self, engine: Engine) {
        self.send_start(Some(engine));
    }

    /// Why the stream stopped after it started, if it did.
    pub fn failure(&self) -> Option<String> {
        self.failure.lock().ok().and_then(|slot| slot.clone())
    }

    fn send_start(&mut self, start: Start) {
        if let Some(sender) = self.start.take() {
            let _ = sender.send(start);
        }
    }
}

impl Drop for DeviceStream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.start.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn open_capture(
    device_id: &str,
    input: usize,
    shared: &Arc<Shared>,
) -> Result<DeviceStream, BackendError> {
    let id = device_id.to_string();
    let shared = Arc::clone(shared);
    spawn(
        format!("audio-program-capture-{input}"),
        move || open_endpoint(&id, Direction::Capture),
        move |opened, _engine, stop| capture_loop(opened, &shared, input, stop),
    )
}

pub fn open_render(
    device_id: &str,
    output: usize,
    shared: &Arc<Shared>,
) -> Result<DeviceStream, BackendError> {
    let id = device_id.to_string();
    let shared = Arc::clone(shared);
    spawn(
        format!("audio-program-render-{output}"),
        move || open_endpoint(&id, Direction::Render),
        move |opened, engine, stop| render_loop(opened, &shared, output, engine, stop),
    )
}

fn spawn(
    name: String,
    open: impl FnOnce() -> Result<Opened, String> + Send + 'static,
    run: impl FnOnce(Opened, Start, &AtomicBool) -> Result<(), String> + Send + 'static,
) -> Result<DeviceStream, BackendError> {
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<u32, String>>(1);
    let (start_tx, start_rx) = mpsc::sync_channel::<Start>(1);
    let stop = Arc::new(AtomicBool::new(false));
    let failure = Arc::new(Mutex::new(None));
    let thread_stop = Arc::clone(&stop);
    let thread_failure = Arc::clone(&failure);
    let thread = std::thread::Builder::new()
        .name(name)
        .spawn(move || device_thread(open, run, ready_tx, start_rx, &thread_stop, &thread_failure))
        .map_err(|err| BackendError::Device(err.to_string()))?;
    let mut stream = DeviceStream {
        rate: 0,
        start: Some(start_tx),
        stop,
        failure,
        thread: Some(thread),
    };
    match ready_rx.recv_timeout(OPEN_TIMEOUT) {
        Ok(Ok(rate)) => {
            stream.rate = rate;
            Ok(stream)
        }
        Ok(Err(message)) => Err(BackendError::Device(message)),
        Err(_) => {
            // A device that hangs while opening is left to exit on its own.
            stream.thread.take();
            Err(BackendError::Device(
                "The device did not open in time.".to_string(),
            ))
        }
    }
}

fn device_thread(
    open: impl FnOnce() -> Result<Opened, String>,
    run: impl FnOnce(Opened, Start, &AtomicBool) -> Result<(), String>,
    ready: SyncSender<Result<u32, String>>,
    start: Receiver<Start>,
    stop: &AtomicBool,
    failure: &Mutex<Option<String>>,
) {
    let _ = initialize_mta();
    let opened = match open() {
        Ok(opened) => opened,
        Err(message) => {
            let _ = ready.send(Err(message));
            return;
        }
    };
    let _ = ready.send(Ok(opened.rate));
    let Ok(engine) = start.recv() else {
        return;
    };
    let _priority = ProAudioThread::enter();
    if let Err(message) = run(opened, engine, stop) {
        if let Ok(mut slot) = failure.lock() {
            *slot = Some(message);
        }
    }
}

fn open_endpoint(device_id: &str, direction: Direction) -> Result<Opened, String> {
    let device = DeviceEnumerator::new()
        .and_then(|enumerator| enumerator.get_device(device_id))
        .map_err(describe)?;
    let mut client = device.get_iaudioclient().map_err(describe)?;
    let mix_rate = client
        .get_mixformat()
        .map_err(describe)?
        .get_samplespersec();
    let rate = stream_rate(mix_rate);
    let format = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        rate as usize,
        DEVICE_CHANNELS,
        None,
    );
    let (period, _) = client.get_device_period().map_err(describe)?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: period,
    };
    client
        .initialize_client(&format, &direction, &mode)
        .map_err(describe)?;
    let event = client.set_get_eventhandle().map_err(describe)?;
    let buffer_frames = client.get_buffer_size().map_err(describe)? as usize;
    Ok(Opened {
        client,
        event,
        rate,
        buffer_frames,
    })
}

fn capture_loop(
    opened: Opened,
    shared: &Shared,
    input: usize,
    stop: &AtomicBool,
) -> Result<(), String> {
    let capture = opened.client.get_audiocaptureclient().map_err(describe)?;
    let mut bytes = vec![0u8; opened.buffer_frames * FRAME_BYTES];
    opened.client.start_stream().map_err(describe)?;
    while !stop.load(Ordering::Relaxed) {
        if !wait(&opened)? {
            continue;
        }
        while capture.get_next_packet_size().map_err(describe)? != Some(0) {
            let (frames, info) = capture.read_from_device(&mut bytes).map_err(describe)?;
            if frames == 0 {
                break;
            }
            let packet = &bytes[..frames as usize * FRAME_BYTES];
            push_capture(shared, input, packet, DEVICE_CHANNELS, info.flags.silent);
        }
    }
    let _ = opened.client.stop_stream();
    Ok(())
}

fn render_loop(
    opened: Opened,
    shared: &Shared,
    output: usize,
    mut engine: Option<Engine>,
    stop: &AtomicBool,
) -> Result<(), String> {
    let render = opened.client.get_audiorenderclient().map_err(describe)?;
    let mut bytes = vec![0u8; opened.buffer_frames * FRAME_BYTES];
    let space = opened
        .client
        .get_available_space_in_frames()
        .map_err(describe)? as usize;
    render
        .write_to_device(space, &bytes[..space * FRAME_BYTES], None)
        .map_err(describe)?;
    opened.client.start_stream().map_err(describe)?;
    while !stop.load(Ordering::Relaxed) {
        if !wait(&opened)? {
            continue;
        }
        let space = opened
            .client
            .get_available_space_in_frames()
            .map_err(describe)? as usize;
        if let Some(engine) = engine.as_mut() {
            mix_ahead(engine, shared, output, space);
        }
        let packet = &mut bytes[..space * FRAME_BYTES];
        fill_render(shared, output, packet, DEVICE_CHANNELS);
        render
            .write_to_device(space, packet, None)
            .map_err(describe)?;
    }
    let _ = opened.client.stop_stream();
    Ok(())
}

/// Wait for the device event. A timeout asks the client whether the device is still there.
fn wait(opened: &Opened) -> Result<bool, String> {
    if opened.event.wait_for_event(WAIT_MS).is_ok() {
        return Ok(true);
    }
    opened.client.get_current_padding().map_err(describe)?;
    Ok(false)
}

fn describe(err: WasapiError) -> String {
    if let WasapiError::Windows(windows_err) = &err {
        if let Some(text) = device_error_text(windows_err.code().0) {
            return text.to_string();
        }
    }
    err.to_string()
}

/// Registers the thread with the Windows multimedia scheduler while it lives.
struct ProAudioThread(*mut core::ffi::c_void);

#[link(name = "avrt")]
unsafe extern "system" {
    fn AvSetMmThreadCharacteristicsW(task: *const u16, index: *mut u32) -> *mut core::ffi::c_void;
    fn AvRevertMmThreadCharacteristics(handle: *mut core::ffi::c_void) -> i32;
}

impl ProAudioThread {
    fn enter() -> Self {
        let task: Vec<u16> = "Pro Audio".encode_utf16().chain(Some(0)).collect();
        let mut index = 0u32;
        Self(unsafe { AvSetMmThreadCharacteristicsW(task.as_ptr(), &mut index) })
    }
}

impl Drop for ProAudioThread {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                AvRevertMmThreadCharacteristics(self.0);
            }
        }
    }
}
