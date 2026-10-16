use super::{cable_spec, configure_default};
use audio_program_engine::Engine;

#[test]
fn default_session_names_cables_and_routes_input_to_matching_output() {
    let mut engine = Engine::new();
    configure_default(&mut engine);
    let shared = engine.shared();
    assert!(shared.routed(0, 0));
    assert!(shared.routed(1, 1));
    assert!(!shared.routed(0, 1));
    assert!(!shared.routed(1, 0));
    assert!((shared.gain(0) - 1.0).abs() < f32::EPSILON);
    assert!((shared.gain(1) - 1.0).abs() < f32::EPSILON);

    let input = cable_spec(0, false);
    assert_eq!(input.node_name, "audio-program-virtual-input-1");
    assert_eq!(input.description, "Audio Program Virtual Input 1");
    assert_eq!(input.index, 0);
    let output = cable_spec(1, true);
    assert_eq!(output.node_name, "audio-program-virtual-output-2");
    assert_eq!(output.description, "Audio Program Virtual Output 2");
    assert_eq!(output.index, 1);
}
