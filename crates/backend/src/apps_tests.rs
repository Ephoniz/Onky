use super::{add_session, exe_names, supports_app_capture, AppSession};

fn app(pid: u32, playing: bool) -> AppSession {
    AppSession {
        pid,
        exe: "spotify.exe".to_string(),
        name: "Spotify".to_string(),
        playing,
    }
}

#[test]
fn app_capture_starts_at_windows_10_version_2004() {
    assert!(!supports_app_capture(0));
    assert!(!supports_app_capture(18363));
    assert!(supports_app_capture(19041));
    assert!(supports_app_capture(22631));
}

#[test]
fn exe_names_come_from_the_file_name() {
    assert_eq!(
        exe_names(r"C:\Users\me\AppData\Roaming\Spotify\Spotify.exe"),
        Some(("spotify.exe".to_string(), "Spotify".to_string()))
    );
    assert_eq!(
        exe_names("C:/Games/Game.EXE"),
        Some(("game.exe".to_string(), "Game".to_string()))
    );
    assert_eq!(
        exe_names("helper"),
        Some(("helper".to_string(), "helper".to_string()))
    );
    assert_eq!(exe_names(r"C:\Tools\.exe"), None);
    assert_eq!(exe_names(r"C:\folder\"), None);
    assert_eq!(exe_names(""), None);
}

#[test]
fn a_process_with_several_sessions_is_listed_once() {
    let mut apps = Vec::new();
    add_session(&mut apps, app(7, false));
    add_session(&mut apps, app(7, true));
    add_session(&mut apps, app(7, false));
    add_session(&mut apps, app(9, false));
    assert_eq!(apps, vec![app(7, true), app(9, false)]);
}
