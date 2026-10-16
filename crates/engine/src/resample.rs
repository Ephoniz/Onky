//! Cubic resampler with a slew-limited ratio.
//!
//! `ratio` is slave frames consumed per master frame for an input. Both devices
//! may be nominally 48 kHz; the ratio still moves when the ring fill leaves the
//! 10 ms target. The step is capped so a full ring cannot jump the pitch.

use crate::{Frame, BLOCK_FRAMES, DRIFT_TARGET_FRAMES, MAX_CHANNELS};

pub const SLEW_PPM_PER_BLOCK: f64 = 50.0;
pub const MAX_RATIO_PPM: f64 = 1000.0;
const SRC_CAP: usize = 1024;

#[derive(Clone)]
pub struct Resampler {
    base: f64,
    ppm: f64,
    phase: f64,
    channels: usize,
    src_len: usize,
    src: Vec<Frame>,
}

impl Resampler {
    pub fn new() -> Self {
        Self {
            base: 1.0,
            ppm: 0.0,
            phase: 0.0,
            channels: 2,
            src_len: 0,
            src: vec![[0.0; MAX_CHANNELS]; SRC_CAP],
        }
    }

    pub fn set_rates(&mut self, slave_rate: u32, master_rate: u32) {
        let slave = slave_rate.max(1) as f64;
        let master = master_rate.max(1) as f64;
        self.base = slave / master;
    }

    pub fn set_channels(&mut self, channels: usize) {
        self.channels = channels.clamp(1, MAX_CHANNELS);
    }

    pub fn ratio_ppm(&self) -> f64 {
        self.ppm
    }

    pub fn ratio(&self) -> f64 {
        self.base * (1.0 + self.ppm * 1e-6)
    }

    /// Positive error means the ring is above the target, so consume faster.
    pub fn slew_input_fill(&mut self, fill: usize) {
        self.slew(fill as f64 - DRIFT_TARGET_FRAMES as f64);
    }

    /// Positive error means the output ring is below the target, so produce more.
    pub fn slew_output_fill(&mut self, fill: usize) {
        self.slew(DRIFT_TARGET_FRAMES as f64 - fill as f64);
    }

    fn slew(&mut self, error_frames: f64) {
        let desired = (error_frames / DRIFT_TARGET_FRAMES as f64) * 500.0;
        let desired = desired.clamp(-MAX_RATIO_PPM, MAX_RATIO_PPM);
        let delta = (desired - self.ppm).clamp(-SLEW_PPM_PER_BLOCK, SLEW_PPM_PER_BLOCK);
        self.ppm = (self.ppm + delta).clamp(-MAX_RATIO_PPM, MAX_RATIO_PPM);
    }

    /// Produce exactly `BLOCK_FRAMES` master frames from `new_frames` of slave audio.
    pub fn render_input(&mut self, new_frames: &[Frame], out: &mut [Frame]) {
        let channels = self.channels;
        let src_len = self.append(new_frames);
        let step = self.ratio().max(1e-4);
        let mut pos = self.phase;
        for frame in out.iter_mut().take(BLOCK_FRAMES) {
            if src_len < 2 || pos + 1.0 >= src_len as f64 {
                *frame = [0.0; MAX_CHANNELS];
                continue;
            }
            *frame = sample_cubic(&self.src, src_len, pos, channels);
            pos += step;
        }
        self.phase = pos;
        self.discard_consumed();
    }

    /// Turn one master block into slave frames. Returns how many were written.
    pub fn render_output(&mut self, master: &[Frame], out: &mut [Frame]) -> usize {
        let channels = self.channels;
        let src_len = self.append(master);
        let step = 1.0 / self.ratio().max(1e-4);
        let mut pos = self.phase;
        let mut produced = 0;
        while produced < out.len() && pos + 1.0 < src_len as f64 {
            out[produced] = sample_cubic(&self.src, src_len, pos, channels);
            produced += 1;
            pos += step;
        }
        self.phase = pos;
        self.discard_consumed();
        produced
    }

    fn append(&mut self, new_frames: &[Frame]) -> usize {
        self.discard_consumed();
        let room = SRC_CAP.saturating_sub(self.src_len);
        let take = new_frames.len().min(room);
        for (index, frame) in new_frames.iter().take(take).enumerate() {
            self.src[self.src_len + index] = *frame;
        }
        self.src_len += take;
        self.src_len
    }

    fn discard_consumed(&mut self) {
        if self.src_len == 0 {
            self.phase = 0.0;
            return;
        }
        let behind = self.phase.floor() as usize;
        let drop = behind.saturating_sub(2).min(self.src_len);
        if drop == 0 {
            return;
        }
        self.src.copy_within(drop..self.src_len, 0);
        self.src_len -= drop;
        self.phase -= drop as f64;
    }
}

impl Default for Resampler {
    fn default() -> Self {
        Self::new()
    }
}

fn sample_cubic(src: &[Frame], len: usize, pos: f64, channels: usize) -> Frame {
    let i1 = pos.floor() as usize;
    let t = (pos - i1 as f64) as f32;
    let mut out = [0.0; MAX_CHANNELS];
    for (channel, sample) in out.iter_mut().take(channels).enumerate() {
        let y0 = at(src, len, i1 as isize - 1, channel);
        let y1 = at(src, len, i1 as isize, channel);
        let y2 = at(src, len, i1 as isize + 1, channel);
        let y3 = at(src, len, i1 as isize + 2, channel);
        *sample = cubic(y0, y1, y2, y3, t);
    }
    out
}

fn at(src: &[Frame], len: usize, index: isize, channel: usize) -> f32 {
    if len == 0 {
        return 0.0;
    }
    let index = index.clamp(0, len as isize - 1) as usize;
    src[index][channel]
}

fn cubic(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let a = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
    let b = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c = -0.5 * y0 + 0.5 * y2;
    let d = y1;
    ((a * t + b) * t + c) * t + d
}

#[cfg(test)]
#[path = "resample_tests.rs"]
mod tests;
