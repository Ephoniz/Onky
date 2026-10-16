use super::*;
use audio_program_engine::SAMPLE_RATE;

fn packet(samples: &[f32]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

fn samples(bytes: &[u8]) -> Vec<f32> {
    let (raw_samples, _) = bytes.as_chunks::<4>();
    raw_samples
        .iter()
        .map(|raw| f32::from_le_bytes(*raw))
        .collect()
}

#[test]
fn capture_packet_lands_in_its_own_input_ring() {
    let engine = Engine::new();
    let shared = engine.shared();
    let bytes = packet(&[0.5, -0.5, 0.25, -0.25]);
    assert_eq!(push_capture(&shared, 3, &bytes, 2, false), 2);
    assert_eq!(shared.input_available(3), 2);
    assert_eq!(shared.input_available(0), 0);
}

#[test]
fn silent_capture_keeps_time_with_zeros() {
    let mut engine = Engine::new();
    let shared = engine.shared();
    engine.configure_input(0, 2, SAMPLE_RATE);
    engine.configure_output(0, 2, SAMPLE_RATE, true);
    shared.set_route(0, 0, true);
    let bytes = packet(&[0.8; 2 * 960]);
    assert_eq!(push_capture(&shared, 0, &bytes, 2, true), 960);
    engine.process();
    let mut out = [[1.0; MAX_CHANNELS]; BLOCK_FRAMES];
    assert_eq!(shared.pop_output(0, &mut out), BLOCK_FRAMES);
    assert!(out.iter().all(|frame| frame[0] == 0.0 && frame[1] == 0.0));
}

#[test]
fn capture_reaches_a_routed_master_output() {
    let mut engine = Engine::new();
    let shared = engine.shared();
    engine.configure_input(0, 2, SAMPLE_RATE);
    engine.configure_output(0, 2, SAMPLE_RATE, true);
    shared.set_route(0, 0, true);
    let mut interleaved = Vec::new();
    for _ in 0..2_000 {
        interleaved.extend_from_slice(&[0.25, -0.5]);
    }
    push_capture(&shared, 0, &packet(&interleaved), 2, false);
    for _ in 0..3 {
        engine.process();
    }
    let mut out = vec![0u8; BLOCK_FRAMES * 2 * 4];
    let mut drained = vec![[0.0; MAX_CHANNELS]; BLOCK_FRAMES];
    shared.pop_output(0, &mut drained);
    shared.pop_output(0, &mut drained);
    assert_eq!(fill_render(&shared, 0, &mut out, 2), BLOCK_FRAMES);
    for frame in samples(&out).as_chunks::<2>().0 {
        assert!((frame[0] - 0.25).abs() < 1e-3, "left {}", frame[0]);
        assert!((frame[1] + 0.5).abs() < 1e-3, "right {}", frame[1]);
    }
}

#[test]
fn capture_stops_when_the_ring_is_full() {
    let engine = Engine::new();
    let shared = engine.shared();
    let bytes = packet(&vec![0.1; 2 * (RING_FRAMES + 10)]);
    assert_eq!(push_capture(&shared, 0, &bytes, 2, false), RING_FRAMES - 1);
}

#[test]
fn capture_ignores_a_trailing_partial_frame_and_zero_channels() {
    let engine = Engine::new();
    let shared = engine.shared();
    let mut bytes = packet(&[0.1, 0.2, 0.3]);
    bytes.truncate(10);
    assert_eq!(push_capture(&shared, 0, &bytes, 2, false), 1);
    assert_eq!(push_capture(&shared, 0, &bytes, 0, false), 0);
}

#[test]
fn render_writes_silence_after_the_ring_runs_out() {
    let mut engine = Engine::new();
    let shared = engine.shared();
    engine.configure_input(0, 2, SAMPLE_RATE);
    engine.configure_output(0, 2, SAMPLE_RATE, true);
    engine.process();
    let mut out = packet(&[7.0; 2 * (BLOCK_FRAMES + 20)]);
    assert_eq!(fill_render(&shared, 0, &mut out, 2), BLOCK_FRAMES);
    assert!(samples(&out).iter().all(|sample| *sample == 0.0));
    assert_eq!(fill_render(&shared, 0, &mut out, 0), 0);
}

#[test]
fn render_zeroes_device_channels_past_the_engine() {
    let engine = Engine::new();
    let shared = engine.shared();
    let channels = MAX_CHANNELS + 2;
    let mut out = packet(&vec![3.0; channels * 4]);
    assert_eq!(fill_render(&shared, 0, &mut out, channels), 0);
    assert!(samples(&out).iter().all(|sample| *sample == 0.0));
}

#[test]
fn mix_ahead_fills_the_master_ring_to_the_request() {
    let mut engine = Engine::new();
    let shared = engine.shared();
    engine.configure_output(0, 2, SAMPLE_RATE, true);
    mix_ahead(&mut engine, &shared, 0, 1_000);
    assert_eq!(shared.output_available(0), 3 * BLOCK_FRAMES);
    mix_ahead(&mut engine, &shared, 0, 1_000);
    assert_eq!(shared.output_available(0), 3 * BLOCK_FRAMES);
}

#[test]
fn mix_ahead_gives_up_when_the_master_never_fills() {
    let mut engine = Engine::new();
    let shared = engine.shared();
    mix_ahead(&mut engine, &shared, 0, 1_000);
    assert_eq!(shared.output_available(0), 0);
}

#[test]
fn stream_rate_keeps_rates_the_resampler_can_take() {
    assert_eq!(stream_rate(44_100), 44_100);
    assert_eq!(stream_rate(48_000), 48_000);
    assert_eq!(stream_rate(96_000), 96_000);
    assert_eq!(stream_rate(192_000), SAMPLE_RATE);
    assert_eq!(stream_rate(0), SAMPLE_RATE);
}

#[test]
fn device_errors_have_plain_text() {
    let unplugged = 0x8889_0004u32 as i32;
    assert_eq!(
        device_error_text(unplugged),
        Some("The device was unplugged or its format changed.")
    );
    assert!(device_error_text(0x8889_000Au32 as i32).is_some());
    assert!(device_error_text(0x8007_0490u32 as i32).is_some());
    assert_eq!(device_error_text(0), None);
}
