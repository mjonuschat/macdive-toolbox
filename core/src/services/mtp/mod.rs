//! MTP (Media Transfer Protocol) device service.
//!
//! Provides detection, connection, file-tree browsing, and activity-file
//! synchronisation for MTP devices (e.g. dive computers, Garmin watches).

mod device;
mod files;

pub use device::{Device, DeviceSelector};
pub use files::{ActivityFolder, detect, filetree, is_activity_file, read_existing_activities};

use crate::error::Error;

/// Maps an [`mtp_rs::Error`] into our crate-level [`Error::Mtp`].
///
/// Checks [`mtp_rs::Error::is_exclusive_access`] to surface a macOS-specific
/// diagnostic message when `ptpcamerad` claims the USB interface.
pub(super) fn map_mtp_error(e: mtp_rs::Error) -> Error {
    if e.is_exclusive_access() {
        Error::Mtp(
            "device claimed by another process. On macOS, try: \
             sudo launchctl unload /System/Library/LaunchDaemons/com.apple.ptpcamerad.plist"
                .to_string(),
        )
    } else {
        Error::Mtp(e.to_string())
    }
}
