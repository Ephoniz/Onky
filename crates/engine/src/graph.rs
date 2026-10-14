use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::resample::Resampler;
use crate::ring::SpscRing;
use crate::{Frame, BLOCK_FRAMES, MAX_CHANNELS, MAX_INPUTS, MAX_OUTPUTS, SAMPLE_RATE};

struct Endpoint {
    resampler: Resampler,
    active: bool,
    channels: usize,
}

/// Gain, mute, routes, meters, and the device rings.
///
/// Device callbacks and the UI share this. The audio thread is the only
/// consumer of input rings and the only producer of output rings.
pub struct Shared {
    gain: [AtomicU32; MAX_INPUTS],
    mute: [AtomicBool; MAX_INPUTS],
    routes: [AtomicU32; MAX_INPUTS],
    peak_in: [AtomicU32; MAX_INPUTS],
    peak_out: [AtomicU32; MAX_OUTPUTS],
    input_count: AtomicUsize,
    output_count: AtomicUsize,
    inputs: [SpscRing; MAX_INPUTS],
    outputs: [SpscRing; MAX_OUTPUTS],
}

impl Shared {
    fn new() -> Self {
        Self {
            gain: std::array::from_fn(|_| AtomicU32::new(1.0f32.to_bits())),
            mute: std::array::from_fn(|_| AtomicBool::new(false)),
            routes: std::array::from_fn(|_| AtomicU32::new(0)),
            peak_in: std::array::from_fn(|_| AtomicU32::new(0)),
            peak_out: std::array::from_fn(|_| AtomicU32::new(0)),
            input_count: AtomicUsize::new(0),
            output_count: AtomicUsize::new(0),
            inputs: std::array::from_fn(|_| SpscRing::new()),
            outputs: std::array::from_fn(|_| SpscRing::new()),
        }
    }

    pub fn set_gain(&self, input: usize, gain: f32) {
        if input < MAX_INPUTS {
            self.gain[input].store(gain.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn gain(&self, input: usize) -> f32 {
        f32::from_bits(self.gain[input].load(Ordering::Relaxed))
    }

    pub fn set_mute(&self, input: usize, mute: bool) {
        if input < MAX_INPUTS {
            self.mute[input].store(mute, Ordering::Relaxed);
        }
    }

    pub fn muted(&self, input: usize) -> bool {
        self.mute[input].load(Ordering::Relaxed)
    }

    pub fn set_route(&self, input: usize, output: usize, on: bool) {
        if input >= MAX_INPUTS || output >= MAX_OUTPUTS {
            return;
        }
        let bit = 1u32 << output;
        if on {
            self.routes[input].fetch_or(bit, Ordering::Relaxed);
        } else {
            self.routes[input].fetch_and(!bit, Ordering::Relaxed);
        }
    }

    pub fn routed(&self, input: usize, output: usize) -> bool {
        let bits = self.routes[input].load(Ordering::Relaxed);
        bits & (1u32 << output) != 0
    }

    pub fn push_input(&self, input: usize, frames: &[Frame]) -> usize {
        self.inputs[input].push(frames)
    }

    pub fn push_input_frame(&self, input: usize, frame: &Frame) -> bool {
        self.inputs[input].push_frame(frame)
    }

    pub fn pop_output(&self, output: usize, out: &mut [Frame]) -> usize {
        self.outputs[output].pop(out)
    }

    pub fn pop_output_frame(&self, output: usize) -> Option<Frame> {
        let mut frame = [Frame::default()];
        if self.outputs[output].pop(&mut frame) == 1 {
            Some(frame[0])
        } else {
            None
        }
    }

    pub fn input_available(&self, input: usize) -> usize {
        self.inputs[input].available()
    }

    pub fn output_available(&self, output: usize) -> usize {
        self.outputs[output].available()
    }

    pub fn peak_input(&self, input: usize) -> f32 {
        f32::from_bits(self.peak_in[input].load(Ordering::Relaxed))
    }

    pub fn peak_output(&self, output: usize) -> f32 {
        f32::from_bits(self.peak_out[output].load(Ordering::Relaxed))
    }

    pub fn input_count(&self) -> usize {
        self.input_count.load(Ordering::Relaxed)
    }

    pub fn output_count(&self) -> usize {
        self.output_count.load(Ordering::Relaxed)
    }
}

/// Mixer owned by the audio thread.
pub struct Engine {
    shared: Arc<Shared>,
    inputs: [Endpoint; MAX_INPUTS],
    outputs: [Endpoint; MAX_OUTPUTS],
    input_count: usize,
    output_count: usize,
    master: Option<usize>,
    in_bus: Vec<Frame>,
    out_bus: Vec<Frame>,
    pulled: Vec<Frame>,
    slave: Vec<Frame>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared::new()),
            inputs: std::array::from_fn(|_| Endpoint {
                resampler: Resampler::new(),
                active: false,
                channels: 2,
            }),
            outputs: std::array::from_fn(|_| Endpoint {
                resampler: Resampler::new(),
                active: false,
                channels: 2,
            }),
            input_count: 0,
            output_count: 0,
            master: None,
            in_bus: vec![[0.0; MAX_CHANNELS]; MAX_INPUTS * BLOCK_FRAMES],
            out_bus: vec![[0.0; MAX_CHANNELS]; MAX_OUTPUTS * BLOCK_FRAMES],
            pulled: vec![[0.0; MAX_CHANNELS]; 1024],
            slave: vec![[0.0; MAX_CHANNELS]; 1024],
        }
    }

    pub fn shared(&self) -> Arc<Shared> {
        Arc::clone(&self.shared)
    }

    pub fn configure_input(&mut self, index: usize, channels: usize, nominal_rate: u32) {
        if index >= MAX_INPUTS {
            return;
        }
        let channels = channels.clamp(1, MAX_CHANNELS);
        let endpoint = &mut self.inputs[index];
        endpoint.active = true;
        endpoint.channels = channels;
        endpoint.resampler.set_channels(channels);
        endpoint.resampler.set_rates(nominal_rate, SAMPLE_RATE);
        self.input_count = self.input_count.max(index + 1);
        self.shared
            .input_count
            .store(self.input_count, Ordering::Relaxed);
    }

    pub fn configure_output(
        &mut self,
        index: usize,
        channels: usize,
        nominal_rate: u32,
        master: bool,
    ) {
        if index >= MAX_OUTPUTS {
            return;
        }
        let channels = channels.clamp(1, MAX_CHANNELS);
        let endpoint = &mut self.outputs[index];
        endpoint.active = true;
        endpoint.channels = channels;
        endpoint.resampler.set_channels(channels);
        endpoint.resampler.set_rates(nominal_rate, SAMPLE_RATE);
        self.output_count = self.output_count.max(index + 1);
        self.shared
            .output_count
            .store(self.output_count, Ordering::Relaxed);
        if master {
            self.master = Some(index);
        } else if self.master == Some(index) {
            self.master = None;
        }
    }

    pub fn input_ratio_ppm(&self, index: usize) -> f64 {
        self.inputs[index].resampler.ratio_ppm()
    }

    pub fn output_ratio_ppm(&self, index: usize) -> f64 {
        self.outputs[index].resampler.ratio_ppm()
    }

    /// One 10 ms mix block. No allocation, no locks, no spinning.
    pub fn process(&mut self) {
        let input_count = self.input_count;
        let output_count = self.output_count;

        for input in 0..input_count {
            if !self.inputs[input].active {
                continue;
            }
            let fill = self.shared.inputs[input].available();
            self.inputs[input].resampler.slew_input_fill(fill);
            let ratio = self.inputs[input].resampler.ratio();
            let need = (((BLOCK_FRAMES as f64) * ratio).ceil() as usize + 4).min(self.pulled.len());
            let got = self.shared.inputs[input].pop(&mut self.pulled[..need]);
            let start = input * BLOCK_FRAMES;
            self.inputs[input].resampler.render_input(
                &self.pulled[..got],
                &mut self.in_bus[start..start + BLOCK_FRAMES],
            );
        }

        for output in 0..output_count {
            let start = output * BLOCK_FRAMES;
            for frame in &mut self.out_bus[start..start + BLOCK_FRAMES] {
                *frame = [0.0; MAX_CHANNELS];
            }
        }

        for input in 0..input_count {
            if !self.inputs[input].active {
                continue;
            }
            let in_channels = self.inputs[input].channels;
            let start = input * BLOCK_FRAMES;
            let mut peak = 0.0f32;
            for frame in &self.in_bus[start..start + BLOCK_FRAMES] {
                for sample in frame.iter().take(in_channels) {
                    peak = peak.max(sample.abs());
                }
            }
            if self.shared.mute[input].load(Ordering::Relaxed) {
                store_peak(&self.shared.peak_in[input], 0.0);
                continue;
            }
            let gain = f32::from_bits(self.shared.gain[input].load(Ordering::Relaxed));
            store_peak(&self.shared.peak_in[input], peak * gain.abs());
            if gain == 0.0 {
                continue;
            }
            let routes = self.shared.routes[input].load(Ordering::Relaxed);
            if routes == 0 {
                continue;
            }
            for output in 0..output_count {
                if routes & (1u32 << output) == 0 || !self.outputs[output].active {
                    continue;
                }
                let shared_channels = in_channels.min(self.outputs[output].channels);
                let out_start = output * BLOCK_FRAMES;
                for frame in 0..BLOCK_FRAMES {
                    for channel in 0..shared_channels {
                        let sample = self.in_bus[start + frame][channel] * gain;
                        self.out_bus[out_start + frame][channel] += sample;
                    }
                }
            }
        }

        for output in 0..output_count {
            if !self.outputs[output].active {
                continue;
            }
            let out_channels = self.outputs[output].channels;
            let start = output * BLOCK_FRAMES;
            let mut peak = 0.0f32;
            for frame in &self.out_bus[start..start + BLOCK_FRAMES] {
                for sample in frame.iter().take(out_channels) {
                    peak = peak.max(sample.abs());
                }
            }
            store_peak(&self.shared.peak_out[output], peak);
            if self.master == Some(output) {
                self.shared.outputs[output].push(&self.out_bus[start..start + BLOCK_FRAMES]);
                continue;
            }
            let fill = self.shared.outputs[output].available();
            self.outputs[output].resampler.slew_output_fill(fill);
            self.pulled[..BLOCK_FRAMES].copy_from_slice(&self.out_bus[start..start + BLOCK_FRAMES]);
            let produced = self.outputs[output]
                .resampler
                .render_output(&self.pulled[..BLOCK_FRAMES], &mut self.slave);
            self.shared.outputs[output].push(&self.slave[..produced]);
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

fn store_peak(slot: &AtomicU32, peak: f32) {
    let previous = f32::from_bits(slot.load(Ordering::Relaxed));
    let shown = if peak >= previous {
        peak
    } else {
        previous * 0.85
    };
    slot.store(shown.to_bits(), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stereo(value0: f32, value1: f32) -> Frame {
        let mut frame = [0.0; MAX_CHANNELS];
        frame[0] = value0;
        frame[1] = value1;
        frame
    }

    fn feed(engine: &Engine, input: usize, frame: Frame, count: usize) {
        let frames = vec![frame; count];
        assert_eq!(engine.shared.push_input(input, &frames), count);
    }

    fn drain_master(engine: &Engine) {
        let mut buf = vec![[0.0; MAX_CHANNELS]; 1024];
        while engine.shared.pop_output(0, &mut buf) > 0 {}
    }

    fn prime(inputs: &[(usize, f32, f32)]) -> Engine {
        let mut engine = Engine::new();
        for (index, _, _) in inputs {
            engine.configure_input(*index, 2, SAMPLE_RATE);
            engine.shared().set_gain(*index, 1.0);
            engine.shared().set_route(*index, 0, true);
        }
        engine.configure_output(0, 2, SAMPLE_RATE, true);
        for (index, left, right) in inputs {
            feed(&engine, *index, stereo(*left, *right), 2_000);
        }
        for _ in 0..3 {
            engine.process();
        }
        drain_master(&engine);
        engine
    }

    fn master_block(engine: &mut Engine, inputs: &[(usize, f32, f32)]) -> Vec<Frame> {
        for (index, left, right) in inputs {
            feed(engine, *index, stereo(*left, *right), 800);
        }
        engine.process();
        let mut out = vec![[0.0; MAX_CHANNELS]; BLOCK_FRAMES];
        let got = engine.shared.pop_output(0, &mut out);
        assert_eq!(got, BLOCK_FRAMES, "master block was short");
        out
    }

    #[test]
    fn routes_sum_and_honor_gain() {
        let inputs = [(0usize, 0.25f32, 0.0f32), (1, 0.25, -0.5)];
        let mut engine = prime(&inputs);
        engine.shared.set_gain(1, 0.5);
        let block = master_block(&mut engine, &inputs);
        for frame in &block {
            assert!((frame[0] - 0.375).abs() < 1e-3, "left {}", frame[0]);
            assert!((frame[1] - (-0.25)).abs() < 1e-3, "right {}", frame[1]);
        }
    }

    #[test]
    fn mute_zero_gain_and_open_route_are_silent() {
        let inputs = [(0usize, 0.8f32, 0.8f32)];
        let mut engine = prime(&inputs);
        engine.shared.set_mute(0, true);
        for frame in master_block(&mut engine, &inputs) {
            assert_eq!(frame[0], 0.0);
            assert_eq!(frame[1], 0.0);
        }

        engine.shared.set_mute(0, false);
        engine.shared.set_gain(0, 0.0);
        for frame in master_block(&mut engine, &inputs) {
            assert_eq!(frame[0], 0.0);
        }

        engine.shared.set_gain(0, 1.0);
        engine.shared.set_route(0, 0, false);
        for frame in master_block(&mut engine, &inputs) {
            assert_eq!(frame[0], 0.0);
        }
    }

    #[test]
    fn channel_count_does_not_invent_channels() {
        let mut engine = Engine::new();
        engine.configure_input(0, 1, SAMPLE_RATE);
        engine.configure_output(0, 2, SAMPLE_RATE, true);
        engine.shared.set_route(0, 0, true);
        let mut mono = [0.0; MAX_CHANNELS];
        mono[0] = 0.4;
        mono[1] = 0.9;
        feed(&engine, 0, mono, 2_000);
        for _ in 0..3 {
            engine.process();
        }
        drain_master(&engine);
        feed(&engine, 0, mono, 800);
        engine.process();
        let mut out = vec![[0.0; MAX_CHANNELS]; BLOCK_FRAMES];
        assert_eq!(engine.shared.pop_output(0, &mut out), BLOCK_FRAMES);
        for frame in &out {
            assert!((frame[0] - 0.4).abs() < 1e-3, "left {}", frame[0]);
            assert_eq!(frame[1], 0.0);
        }

        let mut engine = Engine::new();
        engine.configure_input(0, 2, SAMPLE_RATE);
        engine.configure_output(0, 1, SAMPLE_RATE, true);
        engine.shared.set_route(0, 0, true);
        feed(&engine, 0, stereo(0.3, 0.7), 2_000);
        for _ in 0..3 {
            engine.process();
        }
        drain_master(&engine);
        feed(&engine, 0, stereo(0.3, 0.7), 800);
        engine.process();
        let mut out = vec![[0.0; MAX_CHANNELS]; BLOCK_FRAMES];
        assert_eq!(engine.shared.pop_output(0, &mut out), BLOCK_FRAMES);
        for frame in &out {
            assert!((frame[0] - 0.3).abs() < 1e-4);
        }
    }

    #[test]
    fn input_ratio_slews_when_the_ring_is_high() {
        let mut engine = Engine::new();
        engine.configure_input(0, 2, SAMPLE_RATE);
        engine.configure_output(0, 2, SAMPLE_RATE, true);
        feed(&engine, 0, stereo(0.1, 0.1), 3_000);
        engine.process();
        let ppm = engine.input_ratio_ppm(0);
        assert!(ppm > 0.0 && ppm <= crate::SLEW_PPM_PER_BLOCK + 1e-6);
        let mut previous = ppm;
        for _ in 0..10 {
            engine.process();
            let next = engine.input_ratio_ppm(0);
            assert!(next - previous <= crate::SLEW_PPM_PER_BLOCK + 1e-6);
            previous = next;
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn reference_session_cpu_budget() {
        const BLOCKS: u64 = 2_000;
        let mut engine = Engine::new();
        for input in 0..4 {
            engine.configure_input(input, 2, SAMPLE_RATE);
            engine.shared.set_gain(input, 0.25);
            for output in 0..4 {
                engine.shared.set_route(input, output, true);
            }
            feed(&engine, input, stereo(0.2, -0.2), 2_000);
        }
        for output in 0..4 {
            engine.configure_output(output, 2, SAMPLE_RATE, output == 0);
        }
        let frame = stereo(0.2, -0.15);
        let scratch = vec![frame; BLOCK_FRAMES + 8];
        for _ in 0..8 {
            drive(&engine, 0, &scratch);
        }

        let mut sink = vec![[0.0; MAX_CHANNELS]; BLOCK_FRAMES + 8];
        let mut process_ns = 0u64;
        let total_start = thread_cpu_ns();
        for block in 0..BLOCKS {
            drive(&engine, block, &scratch);
            let start = thread_cpu_ns();
            engine.process();
            process_ns += thread_cpu_ns().saturating_sub(start);
            for output in 0..4 {
                let drift = if output == 0 { 0 } else { 1 + (output % 2) };
                let _ = engine
                    .shared
                    .pop_output(output, &mut sink[..BLOCK_FRAMES + drift]);
            }
        }
        let total_ns = thread_cpu_ns().saturating_sub(total_start);
        let audio_ns = BLOCKS * (BLOCK_FRAMES as u64) * 1_000_000_000 / (SAMPLE_RATE as u64);
        let callback = (process_ns as f64) / (audio_ns as f64) * 100.0;
        let thread = (total_ns as f64) / (audio_ns as f64) * 100.0;
        println!(
            "cpu budget: mix callback {callback:.3}% of one core (limit 2%), audio thread {thread:.3}% (limit 5%)"
        );
        // Debug builds are not the budget measurement. CI and the recorded
        // numbers use `cargo test --release`.
        if cfg!(debug_assertions) {
            return;
        }
        assert!(
            callback < 2.0,
            "mix callback {callback:.3}% exceeds the 2% budget"
        );
        assert!(
            thread < 5.0,
            "audio thread {thread:.3}% exceeds the 5% budget"
        );
    }

    fn drive(engine: &Engine, block: u64, scratch: &[Frame]) {
        let input_ppm: [f64; 4] = [180.0, -220.0, 350.0, -140.0];
        for (input, ppm) in input_ppm.iter().enumerate() {
            let mut extra = ((*ppm).abs() / 200.0) as usize;
            if block % 7 == input as u64 {
                extra += 1;
            }
            let count = BLOCK_FRAMES + extra;
            assert!(count <= scratch.len());
            let _ = engine.shared.push_input(input, &scratch[..count]);
        }
    }

    #[cfg(target_os = "linux")]
    fn thread_cpu_ns() -> u64 {
        let mut time = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe {
            libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time);
        }
        (time.tv_sec as u64) * 1_000_000_000 + time.tv_nsec as u64
    }
}
