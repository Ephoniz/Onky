use audio_program_backend::{AppSession, HardwareDevice};
use audio_program_engine::{Engine, MAX_INPUTS};

use super::{app_exe, app_process, AppChoice, Layout};

fn app(pid: u32, exe: &str, playing: bool) -> AppSession {
    AppSession {
        pid,
        exe: exe.to_string(),
        name: exe.trim_end_matches(".exe").to_string(),
        playing,
    }
}

fn choice(exe: &str, name: &str, used: bool, include_children: bool) -> AppChoice {
    AppChoice {
        exe: exe.to_string(),
        name: name.to_string(),
        used,
        include_children,
    }
}

fn mic() -> HardwareDevice {
    HardwareDevice {
        id: "mic".to_string(),
        name: "Mic".to_string(),
        output: false,
    }
}

fn headphones() -> HardwareDevice {
    HardwareDevice {
        id: "headphones".to_string(),
        name: "Headphones".to_string(),
        output: true,
    }
}

#[test]
fn an_app_becomes_an_input_strip_once() {
    let mut layout = Layout::default();
    assert!(layout.add(&mic()));
    assert!(layout.add_app("spotify.exe", "Spotify"));
    assert!(!layout.add_app("spotify.exe", "Spotify"));
    assert!(layout.uses_app("spotify.exe"));
    assert!(!layout.uses_app("discord.exe"));
    assert_eq!(layout.inputs.len(), 2);
    assert_eq!(app_exe(&layout.inputs[1].id), Some("spotify.exe"));
    assert_eq!(app_exe(&layout.inputs[0].id), None);
    assert_eq!(layout.inputs[1].name, "Spotify");
    assert_eq!(layout.inputs[1].gain, 1.0);
    assert!(!layout.inputs[1].include_children);
}

#[test]
fn apps_share_the_engine_input_bound() {
    let mut layout = Layout::default();
    for index in 0..MAX_INPUTS {
        assert!(layout.add_app(&format!("app{index}.exe"), "App"));
    }
    assert!(!layout.add_app("one-more.exe", "App"));
    assert_eq!(layout.inputs.len(), MAX_INPUTS);
}

#[test]
fn removing_an_app_keeps_device_strips() {
    let mut layout = Layout::default();
    layout.add(&mic());
    layout.add_app("spotify.exe", "Spotify");
    layout.remove_app("spotify.exe");
    assert_eq!(layout.inputs.len(), 1);
    assert_eq!(layout.inputs[0].id, "mic");
}

#[test]
fn app_strips_are_labelled_and_routed_like_inputs() {
    let mut layout = Layout::default();
    layout.add(&mic());
    layout.add_app("spotify.exe", "Spotify");
    layout.add(&headphones());
    layout.inputs[1].routes = vec!["headphones".to_string()];
    let strips = layout.strips(&[None, Some("Not running.".to_string())], &[None]);
    assert_eq!(strips[0].kind, "Input device");
    assert_eq!(strips[1].kind, "App");
    assert_eq!(strips[1].name, "Spotify");
    assert_eq!(strips[1].status.as_deref(), Some("Not running."));
    let shared = Engine::new().shared();
    layout.apply(&shared);
    assert!(shared.routed(1, 0));
    assert!(!shared.routed(0, 0));
}

#[test]
fn app_strips_are_not_offered_as_missing_devices() {
    let mut layout = Layout::default();
    layout.add_app("spotify.exe", "Spotify");
    assert!(layout.choices(&[]).is_empty());
}

#[test]
fn child_processes_are_a_saved_per_app_choice() {
    let mut layout = Layout::default();
    layout.add_app("chrome.exe", "chrome");
    layout.add_app("spotify.exe", "Spotify");
    layout.set_include_children("chrome.exe", true);
    layout.set_include_children("missing.exe", true);
    assert!(layout.inputs[0].include_children);
    assert!(!layout.inputs[1].include_children);
    assert_eq!(Layout::from_json(&layout.to_json()), Some(layout));
}

#[test]
fn a_layout_saved_before_app_strips_still_loads() {
    let text = r#"{"inputs":[{"id":"mic","name":"Mic","gain":0.5,"muted":false}]}"#;
    let layout = Layout::from_json(text).unwrap();
    assert!(!layout.inputs[0].include_children);
    assert!(!layout.to_json().contains("include_children"));
}

#[test]
fn the_process_to_capture_prefers_one_playing_now() {
    let apps = [
        app(4, "discord.exe", false),
        app(5, "discord.exe", true),
        app(6, "discord.exe", false),
        app(7, "spotify.exe", false),
    ];
    assert_eq!(app_process(&apps, "discord.exe"), Some(5));
    assert_eq!(app_process(&apps, "spotify.exe"), Some(7));
    assert_eq!(app_process(&apps, "game.exe"), None);
}

#[test]
fn choices_list_playing_apps_then_quiet_or_closed_app_strips() {
    let mut layout = Layout::default();
    layout.add_app("spotify.exe", "Spotify");
    layout.add_app("game.exe", "Game");
    layout.add_app("discord.exe", "Discord");
    layout.set_include_children("discord.exe", true);
    let apps = [
        app(3, "chrome.exe", true),
        app(4, "chrome.exe", true),
        app(5, "spotify.exe", false),
        app(6, "discord.exe", true),
        app(7, "paused.exe", false),
    ];
    assert_eq!(
        layout.app_choices(&apps),
        vec![
            choice("chrome.exe", "chrome", false, false),
            choice("discord.exe", "discord", true, true),
            choice("spotify.exe", "Spotify", true, false),
            choice("game.exe", "Game (not running)", true, false),
        ]
    );
}

#[test]
fn app_pids_follow_the_input_strips() {
    let mut layout = Layout::default();
    layout.add(&mic());
    layout.add_app("spotify.exe", "Spotify");
    layout.add_app("game.exe", "Game");
    let apps = [app(9, "spotify.exe", true)];
    assert_eq!(layout.app_pids(&apps), vec![None, Some(9), None]);
}

#[test]
fn app_strips_reopen_when_their_app_starts_or_exits() {
    let mut layout = Layout::default();
    layout.add(&mic());
    layout.add_app("spotify.exe", "Spotify");
    let running = [app(9, "spotify.exe", true)];
    let restarted = [app(12, "spotify.exe", true)];
    let opened_running = layout.app_pids(&running);
    let opened_closed = layout.app_pids(&[]);

    assert!(!layout.apps_moved(&opened_running, &running));
    assert!(!layout.apps_moved(&opened_closed, &[]));
    assert!(layout.apps_moved(&opened_closed, &running), "app started");
    assert!(layout.apps_moved(&opened_running, &[]), "app exited");
    assert!(
        layout.apps_moved(&opened_running, &restarted),
        "app restarted"
    );
}

#[test]
fn a_second_process_of_the_same_app_does_not_reopen_the_strip() {
    let mut layout = Layout::default();
    layout.add_app("discord.exe", "Discord");
    let opened = layout.app_pids(&[app(5, "discord.exe", true)]);
    let now = [app(4, "discord.exe", true), app(5, "discord.exe", false)];
    assert!(!layout.apps_moved(&opened, &now));
}
