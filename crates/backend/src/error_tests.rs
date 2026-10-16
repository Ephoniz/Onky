use std::error::Error;

use super::BackendError;
use crate::WINDOWS_VIRTUAL_CABLES;

#[test]
fn errors_display_their_message() {
    let unavailable = BackendError::Unavailable("PipeWire is not running".to_string());
    let cables = BackendError::VirtualCableUnavailable(WINDOWS_VIRTUAL_CABLES.to_string());
    let device = BackendError::Device("device missing".to_string());
    assert_eq!(unavailable.to_string(), "PipeWire is not running");
    assert!(cables.to_string().contains("driver/"));
    assert_eq!(device.to_string(), "device missing");
    assert!(unavailable.source().is_none());
}
