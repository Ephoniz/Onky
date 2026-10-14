//! Virtual-cable backend interface.
//!
//! Linux creates the cables with PipeWire. Windows opens hardware with WASAPI
//! shared mode and refuses to invent cables: those come from the driver in `driver/`.

use std::sync::Arc;

use audio_program_engine::Shared;

mod error;
pub use error::BackendError;

#[cfg(target_os = "linux")]
mod pipewire_backend;
#[cfg(target_os = "linux")]
pub use pipewire_backend::PipeWireBackend;

#[cfg(windows)]
mod wasapi_backend;
#[cfg(windows)]
pub use wasapi_backend::WasapiBackend;

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
