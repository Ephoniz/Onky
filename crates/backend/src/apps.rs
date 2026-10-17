//! Apps that play audio, as Windows lists them in its audio sessions.

/// A process that owns an audio session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppSession {
    pub pid: u32,
    /// Lowercase executable file name, such as `spotify.exe`. Saved layouts match on this.
    pub exe: String,
    /// The executable name as it is on disk, without `.exe`.
    pub name: String,
    /// Whether a stream in one of its sessions is running now.
    pub playing: bool,
}

/// Shown instead of the app list when Windows cannot capture one app.
pub const APP_CAPTURE_NEEDS: &str =
    "Capturing an app needs Windows 10 version 2004 or later. Use a cable for apps instead.";

/// Process loopback came with Windows 10 version 2004, build 19041.
pub fn supports_app_capture(build: u32) -> bool {
    build >= 19041
}

/// The lowercase file name and display name of an executable path.
pub fn exe_names(path: &str) -> Option<(String, String)> {
    let file = path.rsplit(['\\', '/']).next()?;
    let name = match file.len().checked_sub(4) {
        Some(stem) if file[stem..].eq_ignore_ascii_case(".exe") => &file[..stem],
        _ => file,
    };
    if name.is_empty() {
        return None;
    }
    Some((file.to_lowercase(), name.to_string()))
}

/// Add one session's process. A process with sessions on several devices is listed once.
pub fn add_session(apps: &mut Vec<AppSession>, app: AppSession) {
    match apps.iter_mut().find(|known| known.pid == app.pid) {
        Some(known) => known.playing |= app.playing,
        None => apps.push(app),
    }
}

#[cfg(test)]
#[path = "apps_tests.rs"]
mod tests;
