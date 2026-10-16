use audio_program_engine::Engine;

use super::{open_capture, open_render};
use crate::BackendError;

const MISSING: &str = "{0.0.0.00000000}.{00000000-0000-0000-0000-000000000000}";

#[test]
fn a_missing_endpoint_fails_to_open_without_hanging() {
    let shared = Engine::new().shared();
    for result in [
        open_capture(MISSING, 0, &shared),
        open_render(MISSING, 0, &shared),
    ] {
        match result {
            Err(BackendError::Device(message)) => assert!(!message.is_empty()),
            Err(other) => panic!("unexpected error kind: {other}"),
            Ok(_) => panic!("a missing endpoint opened"),
        }
    }
}
