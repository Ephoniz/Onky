//! Per-packet work a device thread does between the device buffer and the engine.
//!
//! These run on device threads. They do not allocate, lock, or wait.

use audio_program_engine::{Engine, Frame, Shared, BLOCK_FRAMES, MAX_CHANNELS, RING_FRAMES};

/// Channels of every device stream. Windows converts other layouts to and from this.
pub const DEVICE_CHANNELS: usize = 2;

/// Highest device rate the engine resampler is sized for.
pub const MAX_DEVICE_RATE: u32 = 96_000;

const SAMPLE_BYTES: usize = std::mem::size_of::<f32>();

/// Rate to open a device at: its own mix rate when the resampler can take it.
pub fn stream_rate(mix_rate: u32) -> u32 {
    if (8_000..=MAX_DEVICE_RATE).contains(&mix_rate) {
        mix_rate
    } else {
        audio_program_engine::SAMPLE_RATE
    }
}

/// Push one interleaved little-endian f32 capture packet into an input ring.
///
/// A packet flagged silent still pushes its length in zeros, so the ring keeps time.
/// Returns how many frames the ring accepted.
pub fn push_capture(
    shared: &Shared,
    input: usize,
    bytes: &[u8],
    channels: usize,
    silent: bool,
) -> usize {
    let stride = channels * SAMPLE_BYTES;
    if stride == 0 {
        return 0;
    }
    let used = channels.min(MAX_CHANNELS);
    let mut pushed = 0;
    for packet_frame in bytes.chunks_exact(stride) {
        let mut frame: Frame = [0.0; MAX_CHANNELS];
        if !silent {
            let (raw_samples, _) = packet_frame.as_chunks::<SAMPLE_BYTES>();
            for (sample, raw) in frame.iter_mut().zip(raw_samples).take(used) {
                *sample = f32::from_le_bytes(*raw);
            }
        }
        if !shared.push_input_frame(input, &frame) {
            break;
        }
        pushed += 1;
    }
    pushed
}

/// Fill one interleaved little-endian f32 render packet from an output ring.
///
/// Frames the ring does not have are written as silence. Returns how many came from the ring.
pub fn fill_render(shared: &Shared, output: usize, bytes: &mut [u8], channels: usize) -> usize {
    let stride = channels * SAMPLE_BYTES;
    if stride == 0 {
        return 0;
    }
    let mut popped = 0;
    for packet_frame in bytes.chunks_exact_mut(stride) {
        let frame = match shared.pop_output_frame(output) {
            Some(frame) => {
                popped += 1;
                frame
            }
            None => [0.0; MAX_CHANNELS],
        };
        let (raw_samples, _) = packet_frame.as_chunks_mut::<SAMPLE_BYTES>();
        for (channel, raw) in raw_samples.iter_mut().enumerate() {
            let sample = if channel < MAX_CHANNELS {
                frame[channel]
            } else {
                0.0
            };
            raw.copy_from_slice(&sample.to_le_bytes());
        }
    }
    popped
}

/// Mix blocks until the master output ring holds at least `frames`.
///
/// The master device thread calls this before each packet, so that device paces the engine.
pub fn mix_ahead(engine: &mut Engine, shared: &Shared, master: usize, frames: usize) {
    for _ in 0..RING_FRAMES / BLOCK_FRAMES {
        if shared.output_available(master) >= frames {
            return;
        }
        engine.process();
    }
}

/// Plain text for the WASAPI failures a user can act on.
pub fn device_error_text(hresult: i32) -> Option<&'static str> {
    match hresult as u32 {
        0x8889_0004 => Some("The device was unplugged or its format changed."),
        0x8889_000A => Some("Another app is using this device in exclusive mode."),
        0x8889_0008 => Some("The device does not accept a shared float stream."),
        0x8889_0010 => Some("The Windows audio service is not running."),
        0x8007_0490 => Some("The device is not connected."),
        _ => None,
    }
}

#[cfg(test)]
#[path = "device_io_tests.rs"]
mod tests;
