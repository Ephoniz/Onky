//! Session wiring for the Audio Program mixer.

#[cfg(any(test, windows))]
mod layout;
mod session;
mod startup;
mod strip;
mod ui;
mod update;

pub use session::Session;
pub use startup::{attach_parent_console, install_panic_hook, report_startup_failure};
pub use ui::{draw, install_style};
pub use update::{
    apply_pending_update, checksum_matches, current_version, is_newer, newest_offer, parse_version,
    set_updates_enabled, updates_enabled, updates_enabled_at, updates_setting_path,
};
