use super::{hardware_output, playback_frame_count};

#[test]
fn hardware_list_skips_our_cables_and_virtual_sources() {
    assert_eq!(hardware_output("speakers", "Audio/Sink"), Some(true));
    assert_eq!(hardware_output("mic", "Audio/Source"), Some(false));
    assert_eq!(
        hardware_output("audio-program-virtual-input-1", "Audio/Sink"),
        None
    );
    assert_eq!(
        hardware_output("audio-program-virtual-output-1", "Audio/Source/Virtual"),
        None
    );
    assert_eq!(hardware_output("monitor", "Audio/Source/Virtual"), None);
}

#[test]
fn playback_does_not_pad_past_the_frames_the_mixer_has() {
    let stride = 8;
    assert_eq!(playback_frame_count(131_072, 131_072, stride, 480), 480);
    assert_eq!(playback_frame_count(0, 131_072, stride, 480), 480);
    assert_eq!(playback_frame_count(131_072, 131_072, stride, 0), 0);
    assert_eq!(playback_frame_count(64, 131_072, stride, 480), 8);
}
