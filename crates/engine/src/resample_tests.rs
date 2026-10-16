use super::*;

#[test]
fn slew_moves_without_jumping_and_stops_at_the_cap() {
    let mut resampler = Resampler::new();
    resampler.set_rates(48_000, 48_000);
    assert_eq!(resampler.ratio_ppm(), 0.0);
    resampler.slew_input_fill(DRIFT_TARGET_FRAMES + 8_000);
    assert!(resampler.ratio_ppm() > 0.0);
    assert!(resampler.ratio_ppm() <= SLEW_PPM_PER_BLOCK + 1e-9);
    let mut previous = resampler.ratio_ppm();
    for _ in 0..40 {
        resampler.slew_input_fill(DRIFT_TARGET_FRAMES + 8_000);
        let delta = resampler.ratio_ppm() - previous;
        assert!(delta <= SLEW_PPM_PER_BLOCK + 1e-6);
        assert!(delta >= -1e-9);
        previous = resampler.ratio_ppm();
    }
    assert!((resampler.ratio_ppm() - MAX_RATIO_PPM).abs() < 1e-6);
}

#[test]
fn constant_signal_survives_ratio_one() {
    let mut resampler = Resampler::new();
    resampler.set_rates(48_000, 48_000);
    resampler.set_channels(2);
    let input = vec![[0.5, -0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; BLOCK_FRAMES + 8];
    let mut output = vec![[0.0; MAX_CHANNELS]; BLOCK_FRAMES];
    resampler.render_input(&input, &mut output);
    resampler.render_input(&input, &mut output);
    for frame in &output {
        assert!((frame[0] - 0.5).abs() < 1e-5);
        assert!((frame[1] + 0.25).abs() < 1e-5);
    }
}

#[test]
fn nominal_rate_sets_the_base_ratio() {
    let mut resampler = Resampler::new();
    resampler.set_rates(96_000, 48_000);
    assert!((resampler.ratio() - 2.0).abs() < 1e-9);
    resampler.set_rates(0, 48_000);
    assert!((resampler.ratio() - (1.0 / 48_000.0)).abs() < 1e-12);
}

#[test]
fn output_fill_slews_the_opposite_way_from_input_fill() {
    let mut high = Resampler::new();
    high.slew_output_fill(DRIFT_TARGET_FRAMES + 8_000);
    assert!(high.ratio_ppm() < 0.0);
    assert!(high.ratio_ppm() >= -SLEW_PPM_PER_BLOCK - 1e-9);

    let mut low = Resampler::new();
    low.slew_output_fill(0);
    assert!(low.ratio_ppm() > 0.0);
    assert!(low.ratio_ppm() <= SLEW_PPM_PER_BLOCK + 1e-9);
}

#[test]
fn underrun_writes_silence_and_output_render_stays_near_one_block() {
    let mut resampler = Resampler::new();
    resampler.set_rates(48_000, 48_000);
    let mut output = vec![[1.0; MAX_CHANNELS]; BLOCK_FRAMES];
    resampler.render_input(&[], &mut output);
    assert!(output.iter().all(|frame| frame[0] == 0.0));

    let input = vec![[0.2; MAX_CHANNELS]; BLOCK_FRAMES];
    let mut slave = vec![[0.0; MAX_CHANNELS]; 1024];
    let produced = resampler.render_output(&input, &mut slave);
    assert!(
        produced > 400 && produced <= BLOCK_FRAMES,
        "produced {produced}"
    );
    assert!((slave[produced / 2][0] - 0.2).abs() < 1e-4);
}
