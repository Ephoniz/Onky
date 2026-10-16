//! WASAPI shared mode for existing endpoints.
//!
//! Any active endpoint can be a strip, including cables from other drivers such as
//! VB-Cable. Virtual cables are not created here. A shared-mode loopback of a speaker
//! is not a device other applications can select. Selectable cables come from the
//! kernel driver in `driver/`.

use std::sync::Arc;

use audio_program_engine::Shared;
use wasapi::{initialize_sta, DeviceEnumerator, Direction};

use crate::wasapi_stream::{self, DeviceStream};
use crate::{
    BackendError, CableSpec, HardwareDevice, VirtualCable, VirtualCableBackend,
    WINDOWS_VIRTUAL_CABLES,
};

pub struct WasapiBackend;

impl WasapiBackend {
    pub fn connect() -> Result<Self, BackendError> {
        // The mixer window runs on this thread. A multithreaded apartment makes
        // the window's OleInitialize fail, so the process exits before anything is shown.
        let result = initialize_sta();
        if result.is_err() {
            return Err(BackendError::Unavailable(format!(
                "COM initialize failed: {result:?}"
            )));
        }
        Ok(Self)
    }

    /// Open an endpoint that captures into engine input `input`. Virtual cables count too.
    ///
    /// Nothing streams until [`DeviceStream::start`].
    pub fn open_capture(
        &self,
        device_id: &str,
        input: usize,
        shared: &Arc<Shared>,
    ) -> Result<DeviceStream, BackendError> {
        wasapi_stream::open_capture(device_id, input, shared)
    }

    /// Open an endpoint that plays engine output `output`. Virtual cables count too.
    ///
    /// Nothing streams until [`DeviceStream::start`] or [`DeviceStream::start_as_clock`].
    pub fn open_render(
        &self,
        device_id: &str,
        output: usize,
        shared: &Arc<Shared>,
    ) -> Result<DeviceStream, BackendError> {
        wasapi_stream::open_render(device_id, output, shared)
    }
}

impl VirtualCableBackend for WasapiBackend {
    fn list_hardware(&self) -> Result<Vec<HardwareDevice>, BackendError> {
        let enumerator =
            DeviceEnumerator::new().map_err(|err| BackendError::Unavailable(err.to_string()))?;
        let mut devices = Vec::new();
        for (direction, output) in [(Direction::Render, true), (Direction::Capture, false)] {
            let collection = enumerator
                .get_device_collection(&direction)
                .map_err(|err| BackendError::Device(err.to_string()))?;
            let count = collection
                .get_nbr_devices()
                .map_err(|err| BackendError::Device(err.to_string()))?;
            for index in 0..count {
                let device = collection
                    .get_device_at_index(index)
                    .map_err(|err| BackendError::Device(err.to_string()))?;
                let id = device
                    .get_id()
                    .map_err(|err| BackendError::Device(err.to_string()))?;
                let name = device.get_friendlyname().unwrap_or_else(|_| id.clone());
                devices.push(HardwareDevice { id, name, output });
            }
        }
        Ok(devices)
    }

    fn create_virtual_input(
        &mut self,
        _spec: CableSpec,
        _shared: &Arc<Shared>,
    ) -> Result<VirtualCable, BackendError> {
        Err(BackendError::VirtualCableUnavailable(
            WINDOWS_VIRTUAL_CABLES.to_string(),
        ))
    }

    fn create_virtual_output(
        &mut self,
        _spec: CableSpec,
        _shared: &Arc<Shared>,
    ) -> Result<VirtualCable, BackendError> {
        Err(BackendError::VirtualCableUnavailable(
            WINDOWS_VIRTUAL_CABLES.to_string(),
        ))
    }
}

#[cfg(test)]
#[path = "wasapi_tests.rs"]
mod tests;
