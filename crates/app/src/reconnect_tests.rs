use super::should_reconnect;

#[test]
fn a_failed_stream_always_reconnects() {
    assert!(should_reconnect(true, false, false));
    assert!(should_reconnect(true, true, true));
}

#[test]
fn a_missing_device_reconnects_only_when_the_device_list_changes() {
    assert!(!should_reconnect(false, false, true));
    assert!(should_reconnect(false, true, true));
}

#[test]
fn a_healthy_mixer_does_not_reconnect() {
    assert!(!should_reconnect(false, false, false));
    assert!(!should_reconnect(false, true, false));
}
