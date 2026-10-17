//! Virtual-cable backend interface.
//!
//! Linux creates the cables with PipeWire. Windows streams existing endpoints with
//! WASAPI shared mode and refuses to invent cables: those come from the driver in `driver/`.

use std::sync::Arc;

use audio_program_engine::Shared;

mod apps;
mod device_io;
mod error;
pub use apps::{add_session, exe_names, supports_app_capture, AppSession, APP_CAPTURE_NEEDS};
pub use device_io::{
    device_error_text, fill_render, mix_ahead, push_capture, stream_rate, DEVICE_CHANNELS,
    MAX_DEVICE_RATE,
};
pub use error::BackendError;

/// Returned when Windows is asked to create a virtual cable.
pub const WINDOWS_VIRTUAL_CABLES: &str =
    "Windows virtual cables are created by the kernel driver in driver/. \
WASAPI shared mode only opens hardware endpoints. A loopback of a playback device is not a \
selectable cable.";

#[cfg(target_os = "linux")]
mod pipewire_backend;
#[cfg(target_os = "linux")]
pub use pipewire_backend::PipeWireBackend;

#[cfg(windows)]
mod wasapi_apps;
#[cfg(windows)]
mod wasapi_backend;
#[cfg(windows)]
pub use wasapi_backend::WasapiBackend;
#[cfg(windows)]
mod wasapi_stream;
#[cfg(windows)]
pub use wasapi_stream::DeviceStream;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareDevice {
    pub id: String,
    pub name: String,
    pub output: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CableSpec {
    pub node_name: String,
    pub description: String,
    /// Engine strip this cable reads or writes.
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualCable {
    pub node_name: String,
    pub description: String,
    pub index: usize,
    pub output: bool,
}

/// What the session needs from a platform backend.
pub trait VirtualCableBackend {
    fn list_hardware(&self) -> Result<Vec<HardwareDevice>, BackendError>;
    fn create_virtual_input(
        &mut self,
        spec: CableSpec,
        shared: &Arc<Shared>,
    ) -> Result<VirtualCable, BackendError>;
    fn create_virtual_output(
        &mut self,
        spec: CableSpec,
        shared: &Arc<Shared>,
    ) -> Result<VirtualCable, BackendError>;
}
