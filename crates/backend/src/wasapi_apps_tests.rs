use super::{list_apps, windows_build};

#[test]
fn the_windows_build_is_read_without_the_compatibility_shim() {
    // Windows 8 is build 9200, which is what the shimmed GetVersionEx would report.
    assert!(windows_build() >= 10240, "build {}", windows_build());
}

#[test]
fn the_app_list_leaves_out_this_process_and_system_sounds() {
    let _ = wasapi::initialize_mta();
    let Ok(apps) = list_apps() else {
        return;
    };
    for app in apps {
        assert_ne!(app.pid, 0);
        assert_ne!(app.pid, std::process::id());
        assert!(!app.exe.is_empty() && !app.name.is_empty());
    }
}
