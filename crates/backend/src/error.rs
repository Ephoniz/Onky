use std::fmt;

#[derive(Debug)]
pub enum BackendError {
    /// The audio server or device API could not be used.
    Unavailable(String),
    /// Virtual cables are not provided by this backend.
    VirtualCableUnavailable(String),
    Device(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::Unavailable(message)
            | BackendError::VirtualCableUnavailable(message)
            | BackendError::Device(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for BackendError {}
