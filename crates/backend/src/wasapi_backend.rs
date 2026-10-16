//! WASAPI shared mode for hardware endpoints.
//!
//! Virtual cables are not created here. A shared-mode loopback of a speaker is
//! not a device other applications can select. Selectable cables come from the
//! kernel driver in `driver/`.

use std::sync::Arc;

use audio_program_engine::Shared;
use wasapi::{initialize_sta, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

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

    /// Initialize one hardware endpoint in shared mode at 48 kHz stereo float.
    ///
    /// This checks that the endpoint accepts the format. It does not start a
    /// render or capture thread, and nothing in the session calls it yet.
    /// Streaming on a real Windows device is still unproven.
    pub fn open_shared(&self, device_id: &str, output: bool) -> Result<(), BackendError> {
        let enumerator =
            DeviceEnumerator::new().map_err(|err| BackendError::Device(err.to_string()))?;
        let device = enumerator
            .get_device(device_id)
            .map_err(|err| BackendError::Device(err.to_string()))?;
        let mut client = device
            .get_iaudioclient()
            .map_err(|err| BackendError::Device(err.to_string()))?;
        let format = WaveFormat::new(32, 32, &SampleType::Float, 48_000, 2, None);
        let (period, _) = client
            .get_device_period()
            .map_err(|err| BackendError::Device(err.to_string()))?;
        let direction = if output {
            Direction::Render
        } else {
            Direction::Capture
        };
        let mode = StreamMode::EventsShared {
            autoconvert: true,
            buffer_duration_hns: period,
        };
        client
            .initialize_client(&format, &direction, &mode)
            .map_err(|err| BackendError::Device(err.to_string()))?;
        Ok(())
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
