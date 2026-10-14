//! Real-time mixer.
//!
//! The audio thread calls [`Engine::process`] once per 10 ms block. That call does not
//! allocate, lock, or spin. Device threads only push and pop the SPSC rings in [`Shared`].
//! Gain, mute, and routes are atomics written by the UI thread.

mod graph;
mod resample;
mod ring;

pub use graph::{Engine, Shared};
pub use resample::{Resampler, MAX_RATIO_PPM, SLEW_PPM_PER_BLOCK};

pub const MAX_INPUTS: usize = 16;
pub const MAX_OUTPUTS: usize = 16;
pub const MAX_CHANNELS: usize = 8;
pub const SAMPLE_RATE: u32 = 48_000;
pub const BLOCK_FRAMES: usize = 480;
pub const DRIFT_TARGET_FRAMES: usize = 480;
pub const RING_FRAMES: usize = 4096;

pub type Frame = [f32; MAX_CHANNELS];
