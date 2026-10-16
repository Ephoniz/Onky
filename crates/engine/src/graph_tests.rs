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

#[test]
fn slave_output_passes_a_constant_input() {
    let mut engine = Engine::new();
    engine.configure_input(0, 2, SAMPLE_RATE);
    engine.configure_output(0, 2, SAMPLE_RATE, false);
    engine.shared().set_gain(0, 1.0);
    engine.shared().set_route(0, 0, true);
    let tone = stereo(0.5, -0.25);
    feed(&engine, 0, tone, 2_000);
    for _ in 0..4 {
        engine.process();
    }
    let mut discard = vec![[0.0; MAX_CHANNELS]; 1024];
    while engine.shared.pop_output(0, &mut discard) > 0 {}
    feed(&engine, 0, tone, 800);
    engine.process();
    let mut out = vec![[0.0; MAX_CHANNELS]; 1024];
    let got = engine.shared.pop_output(0, &mut out);
    assert!(got > 400, "slave output wrote {got} frames");
    for frame in &out[..got] {
        assert!((frame[0] - 0.5).abs() < 1e-3, "left {}", frame[0]);
        assert!((frame[1] + 0.25).abs() < 1e-3, "right {}", frame[1]);
    }
}

#[test]
fn meter_falls_after_the_input_goes_quiet() {
    let mut engine = Engine::new();
    engine.configure_input(0, 2, SAMPLE_RATE);
    engine.configure_output(0, 2, SAMPLE_RATE, true);
    engine.shared().set_route(0, 0, true);
    feed(&engine, 0, stereo(0.8, 0.0), BLOCK_FRAMES + 8);
    engine.process();
    let loud = engine.shared().peak_input(0);
    assert!(loud > 0.5, "peak {loud}");
    while engine.shared().input_available(0) > 0 {
        engine.process();
    }
    for _ in 0..8 {
        feed(&engine, 0, stereo(0.0, 0.0), BLOCK_FRAMES);
        engine.process();
    }
    let quiet = engine.shared().peak_input(0);
    assert!(quiet < loud * 0.5, "quiet {quiet} loud {loud}");
}

#[test]
fn one_input_reaches_each_routed_output() {
    let mut engine = Engine::new();
    engine.configure_input(0, 2, SAMPLE_RATE);
    engine.configure_output(0, 2, SAMPLE_RATE, true);
    engine.configure_output(1, 2, SAMPLE_RATE, false);
    engine.shared().set_gain(0, 1.0);
    engine.shared().set_route(0, 0, true);
    engine.shared().set_route(0, 1, true);
    let tone = stereo(0.4, -0.2);
    feed(&engine, 0, tone, 2_000);
    for _ in 0..4 {
        engine.process();
    }
    drain_master(&engine);
    let mut discard = vec![[0.0; MAX_CHANNELS]; 1024];
    while engine.shared.pop_output(1, &mut discard) > 0 {}

    feed(&engine, 0, tone, 800);
    engine.process();
    let first = engine.shared().pop_output_frame(0).expect("master frame");
    assert!((first[0] - 0.4).abs() < 1e-3);
    assert!((first[1] + 0.2).abs() < 1e-3);
    let mut master = vec![[0.0; MAX_CHANNELS]; BLOCK_FRAMES];
    assert_eq!(engine.shared.pop_output(0, &mut master), BLOCK_FRAMES - 1);
    assert!(engine.shared().peak_output(0) > 0.3);

    let mut slave = vec![[0.0; MAX_CHANNELS]; 1024];
    let got = engine.shared.pop_output(1, &mut slave);
    assert!(got > 400, "second output wrote {got} frames");
    for frame in &slave[..got] {
        assert!((frame[0] - 0.4).abs() < 1e-3, "left {}", frame[0]);
        assert!((frame[1] + 0.2).abs() < 1e-3, "right {}", frame[1]);
    }

    engine.shared().set_route(0, 1, false);
    assert!(!engine.shared().routed(0, 1));
    assert!(engine.shared().routed(0, 0));
    feed(&engine, 0, tone, 800);
    engine.process();
    let again = engine
        .shared()
        .pop_output_frame(0)
        .expect("routed master frame");
    assert!((again[0] - 0.4).abs() < 1e-3);
}

#[test]
fn indexes_past_the_limit_are_ignored() {
    let mut engine = Engine::new();
    engine.configure_input(MAX_INPUTS, 2, SAMPLE_RATE);
    engine.configure_output(MAX_OUTPUTS, 2, SAMPLE_RATE, true);
    engine.shared().set_gain(MAX_INPUTS, 0.2);
    engine.shared().set_mute(MAX_INPUTS, true);
    engine.shared().set_route(MAX_INPUTS, 0, true);
    assert_eq!(engine.shared().input_count(), 0);
    assert_eq!(engine.shared().output_count(), 0);
    engine.configure_input(0, 2, SAMPLE_RATE);
    assert!(!engine.shared().routed(0, 0));
    assert!(!engine.shared().muted(0));
}
