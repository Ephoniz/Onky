//! The apps in the Windows audio sessions, and whether this Windows can capture one.

use std::ffi::c_void;

use wasapi::{AudioSessionControl, Device, DeviceEnumerator, Direction, SessionState, WasapiError};

use crate::{add_session, exe_names, AppSession, BackendError};

/// Processes with an audio session on any playback device, except this one.
pub fn list_apps() -> Result<Vec<AppSession>, BackendError> {
    let collection = DeviceEnumerator::new()
        .and_then(|enumerator| enumerator.get_device_collection(&Direction::Render))
        .map_err(|err| BackendError::Unavailable(err.to_string()))?;
    let count = collection
        .get_nbr_devices()
        .map_err(|err| BackendError::Device(err.to_string()))?;
    let mut apps = Vec::new();
    for index in 0..count {
        if let Ok(device) = collection.get_device_at_index(index) {
            let _ = add_device_sessions(&device, &mut apps);
        }
    }
    Ok(apps)
}

fn add_device_sessions(device: &Device, apps: &mut Vec<AppSession>) -> Result<(), WasapiError> {
    let sessions = device
        .get_iaudiosessionmanager()?
        .get_audiosessionenumerator()?;
    for index in 0..sessions.get_count()? {
        if let Some(app) = sessions
            .get_session(index)
            .ok()
            .and_then(|s| session_app(&s))
        {
            add_session(apps, app);
        }
    }
    Ok(())
}

fn session_app(session: &AudioSessionControl) -> Option<AppSession> {
    let playing = match session.get_state().ok()? {
        SessionState::Active => true,
        SessionState::Inactive => false,
        SessionState::Expired => return None,
    };
    // Process 0 is the system sounds session.
    let pid = session.get_process_id().ok()?;
    if pid == 0 || pid == std::process::id() {
        return None;
    }
    let (exe, name) = exe_names(&process_path(pid)?)?;
    Some(AppSession {
        pid,
        exe,
        name,
        playing,
    })
}

const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
    fn QueryFullProcessImageNameW(
        process: *mut c_void,
        flags: u32,
        name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

fn process_path(pid: u32) -> Option<String> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return None;
    }
    let mut path = [0u16; 1024];
    let mut size = path.len() as u32;
    let read = unsafe { QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut size) };
    unsafe {
        CloseHandle(process);
    }
    (read != 0).then(|| String::from_utf16_lossy(&path[..size as usize]))
}

#[repr(C)]
struct OsVersionInfo {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    service_pack: [u16; 128],
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
}

/// The Windows build number, or 0 when it cannot be read.
///
/// `GetVersionEx` reports Windows 8 to programs without a manifest, so this asks ntdll.
pub fn windows_build() -> u32 {
    let mut info = OsVersionInfo {
        size: std::mem::size_of::<OsVersionInfo>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        platform: 0,
        service_pack: [0; 128],
    };
    if unsafe { RtlGetVersion(&mut info) } == 0 {
        info.build
    } else {
        0
    }
}

#[cfg(test)]
#[path = "wasapi_apps_tests.rs"]
mod tests;
