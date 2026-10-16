use audio_program_backend::HardwareDevice;
use audio_program_engine::{Engine, MAX_INPUTS, MAX_OUTPUTS};

use super::{layout_path, Layout};

fn device(id: &str, output: bool) -> HardwareDevice {
    HardwareDevice {
        id: id.to_string(),
        name: format!("{id} name"),
        output,
    }
}

fn mic_cable_and_two_outputs() -> Layout {
    let mut layout = Layout::default();
    assert!(layout.add(&device("mic", false)));
    assert!(layout.add(&device("cable-output", false)));
    assert!(layout.add(&device("headphones", true)));
    assert!(layout.add(&device("cable-input", true)));
    layout
}

#[test]
fn added_devices_become_strips_once() {
    let mut layout = mic_cable_and_two_outputs();
    assert!(!layout.add(&device("mic", false)));
    assert!(layout.uses(&device("mic", false)));
    assert!(!layout.uses(&device("mic", true)));
    assert_eq!(layout.inputs.len(), 2);
    assert_eq!(layout.outputs.len(), 2);
    assert_eq!(layout.inputs[0].name, "mic name");
    assert_eq!(layout.inputs[0].gain, 1.0);
    assert!(!layout.inputs[0].muted);
    assert!(layout.inputs[0].routes.is_empty());
}

#[test]
fn strips_stop_at_the_engine_bound() {
    let mut layout = Layout::default();
    for index in 0..MAX_INPUTS + 2 {
        layout.add(&device(&format!("in{index}"), false));
    }
    for index in 0..MAX_OUTPUTS + 2 {
        layout.add(&device(&format!("out{index}"), true));
    }
    assert_eq!(layout.inputs.len(), MAX_INPUTS);
    assert_eq!(layout.outputs.len(), MAX_OUTPUTS);
}

#[test]
fn apply_and_capture_round_trip_through_the_engine() {
    let mut layout = mic_cable_and_two_outputs();
    layout.inputs[0].gain = 0.5;
    layout.inputs[0].routes = vec!["headphones".to_string()];
    layout.inputs[1].muted = true;
    layout.inputs[1].routes = vec!["headphones".to_string(), "cable-input".to_string()];
    let shared = Engine::new().shared();
    layout.apply(&shared);
    assert_eq!(shared.gain(0), 0.5);
    assert!(shared.routed(0, 0));
    assert!(!shared.routed(0, 1));
    assert!(shared.muted(1));
    assert!(shared.routed(1, 0) && shared.routed(1, 1));

    shared.set_gain(1, 1.25);
    shared.set_mute(1, false);
    shared.set_route(1, 0, false);
    layout.capture(&shared);
    assert_eq!(layout.inputs[1].gain, 1.25);
    assert!(!layout.inputs[1].muted);
    assert_eq!(layout.inputs[1].routes, vec!["cable-input".to_string()]);
    assert_eq!(layout.inputs[0].routes, vec!["headphones".to_string()]);
}

#[test]
fn apply_clears_routes_left_from_before() {
    let layout = mic_cable_and_two_outputs();
    let shared = Engine::new().shared();
    shared.set_route(0, 1, true);
    layout.apply(&shared);
    assert!(!shared.routed(0, 1));
}

#[test]
fn routes_follow_an_output_when_another_is_removed() {
    let mut layout = mic_cable_and_two_outputs();
    layout.inputs[0].routes = vec!["headphones".to_string(), "cable-input".to_string()];
    layout.remove(&device("headphones", true));
    assert_eq!(layout.inputs[0].routes, vec!["cable-input".to_string()]);
    let shared = Engine::new().shared();
    layout.apply(&shared);
    assert!(shared.routed(0, 0), "cable-input is now output 0");
}

#[test]
fn removing_an_input_keeps_the_others() {
    let mut layout = mic_cable_and_two_outputs();
    layout.remove(&device("mic", false));
    assert_eq!(layout.inputs.len(), 1);
    assert_eq!(layout.inputs[0].id, "cable-output");
    assert_eq!(layout.outputs.len(), 2);
}

#[test]
fn master_is_the_chosen_output_or_the_first() {
    let mut layout = Layout::default();
    assert_eq!(layout.master_index(), None);
    layout = mic_cable_and_two_outputs();
    assert_eq!(layout.master_index(), Some(0));
    layout.set_master(1);
    assert_eq!(layout.master.as_deref(), Some("cable-input"));
    assert_eq!(layout.master_index(), Some(1));
    layout.set_master(9);
    assert_eq!(layout.master_index(), Some(1));
    layout.remove(&device("cable-input", true));
    assert_eq!(layout.master, None);
    assert_eq!(layout.master_index(), Some(0));
}

#[test]
fn strips_carry_names_status_and_the_master() {
    let layout = mic_cable_and_two_outputs();
    let strips = layout.strips(
        &[None, Some("Disconnected".to_string())],
        &[None, Some("Unavailable".to_string())],
    );
    assert_eq!(strips.len(), 4);
    assert_eq!(strips[0].name, "mic name");
    assert_eq!(strips[0].kind, "Input device");
    assert!(!strips[0].output);
    assert_eq!(strips[0].master, None);
    assert_eq!(strips[1].status.as_deref(), Some("Disconnected"));
    assert_eq!(strips[2].index, 0);
    assert!(strips[2].output);
    assert_eq!(strips[2].master, Some(true));
    assert_eq!(strips[3].master, Some(false));
    assert_eq!(strips[3].status.as_deref(), Some("Unavailable"));
    assert_eq!(layout.strips(&[], &[])[1].status, None);
}

#[test]
fn choices_keep_strips_whose_device_is_gone() {
    let layout = mic_cable_and_two_outputs();
    let present = vec![
        device("mic", false),
        device("speakers", true),
        device("headphones", true),
    ];
    let choices = layout.choices(&present);
    assert_eq!(&choices[..3], &present[..]);
    assert_eq!(choices.len(), 5);
    assert_eq!(choices[3].id, "cable-output");
    assert_eq!(choices[3].name, "cable-output name (not connected)");
    assert!(!choices[3].output);
    assert_eq!(choices[4].id, "cable-input");
    assert!(choices[4].output);
    assert!(
        layout.uses(&choices[4]),
        "a missing strip can still be unticked"
    );
}

#[test]
fn json_round_trips_and_bad_text_is_rejected() {
    let mut layout = mic_cable_and_two_outputs();
    layout.inputs[0].gain = 0.75;
    layout.inputs[0].routes = vec!["cable-input".to_string()];
    layout.set_master(1);
    assert_eq!(Layout::from_json(&layout.to_json()), Some(layout));
    assert_eq!(Layout::from_json("not json"), None);
    assert_eq!(Layout::from_json("{}"), Some(Layout::default()));
}

#[test]
fn json_from_a_newer_or_edited_file_is_kept_in_bounds() {
    let mut layout = Layout::default();
    for index in 0..MAX_INPUTS {
        layout.add(&device(&format!("in{index}"), false));
    }
    let mut value: serde_json::Value = serde_json::from_str(&layout.to_json()).unwrap();
    let inputs = value["inputs"].as_array_mut().unwrap();
    inputs.push(inputs[0].clone());
    value["future"] = serde_json::Value::Bool(true);
    let loaded = Layout::from_json(&value.to_string()).unwrap();
    assert_eq!(loaded.inputs.len(), MAX_INPUTS);
}

#[test]
fn save_then_load_restores_the_layout() {
    let dir = std::env::temp_dir().join(format!("audio-program-layout-{}", std::process::id()));
    let path = dir.join("nested").join("session.json");
    let mut layout = mic_cable_and_two_outputs();
    layout.inputs[1].muted = true;
    layout.save(&path).expect("save");
    assert_eq!(Layout::load(&path), layout);
    std::fs::write(&path, "{ broken").unwrap();
    assert_eq!(Layout::load(&path), Layout::default());
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(Layout::load(&path), Layout::default());
}

#[test]
fn layout_is_saved_under_appdata() {
    let Some(path) = layout_path() else {
        return;
    };
    assert!(
        path.ends_with("Audio Program/session.json")
            || path.ends_with("Audio Program\\session.json")
    );
}
