#[cfg(any(test, not(windows)))]
use audio_program_backend::VirtualCable;

/// One strip as the window draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct Strip {
    pub name: String,
    pub kind: &'static str,
    /// Engine input or output index.
    pub index: usize,
    pub output: bool,
    /// A problem to show on the strip, such as an unplugged device.
    pub status: Option<String>,
    /// Whether this output clocks the mixer. `None` when the user cannot choose.
    pub master: Option<bool>,
}

#[cfg(any(test, not(windows)))]
pub fn cable_strips(cables: &[VirtualCable]) -> Vec<Strip> {
    cables
        .iter()
        .map(|cable| Strip {
            name: cable.description.clone(),
            kind: if cable.output {
                "Virtual output"
            } else {
                "Virtual input"
            },
            index: cable.index,
            output: cable.output,
            status: None,
            master: None,
        })
        .collect()
}

#[cfg(test)]
#[path = "strip_tests.rs"]
mod tests;
