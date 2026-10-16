//! Record a failure that would otherwise vanish with the console window.

use std::path::PathBuf;

pub fn startup_log_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("audio-program-startup.log"))
}

pub fn write_startup_log(message: &str) -> Option<PathBuf> {
    let path = startup_log_path()?;
    std::fs::write(&path, format!("audio-program: {message}\n")).ok()?;
    Some(path)
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = format!("panic: {info}");
        write_startup_log(&message);
        show_startup_error(&message);
        previous(info);
    }));
}

pub fn report_startup_failure(message: &str) -> ! {
    eprintln!("audio-program: {message}");
    write_startup_log(message);
    show_startup_error(message);
    std::process::exit(1);
}

pub fn attach_parent_console() {
    #[cfg(windows)]
    attach_parent_console_windows();
}

fn show_startup_error(message: &str) {
    #[cfg(windows)]
    show_message_box(message);
    #[cfg(not(windows))]
    let _ = message;
}

#[cfg(windows)]
fn attach_parent_console_windows() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(windows)]
fn show_message_box(message: &str) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            hwnd: *mut core::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            flags: u32,
        ) -> i32;
    }
    let text = wide(message);
    let caption = wide("Audio Program could not open");
    // MB_OK | MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST
    const FLAGS: u32 = 0x0000_0010 | 0x0001_0000 | 0x0004_0000;
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), FLAGS);
    }
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::{startup_log_path, write_startup_log};

    #[test]
    fn startup_log_is_named_next_to_the_executable() {
        let path = startup_log_path().expect("current exe");
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("audio-program-startup.log")
        );
        let exe = std::env::current_exe().expect("current exe");
        assert_eq!(path.parent(), exe.parent());
    }

    #[test]
    fn startup_log_records_the_failure() {
        let path = write_startup_log("window failed").expect("write");
        let text = std::fs::read_to_string(&path).expect("read");
        let _ = std::fs::remove_file(&path);
        assert!(text.starts_with("audio-program: "));
        assert!(text.contains("window failed"));
    }
}
