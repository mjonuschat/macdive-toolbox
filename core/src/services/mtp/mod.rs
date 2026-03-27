//! MTP (Media Transfer Protocol) device service.
//!
//! Provides detection, connection, file-tree browsing, and activity-file
//! synchronisation for MTP devices (e.g. dive computers, Garmin watches).

mod device;
mod files;

pub use device::{Device, DeviceSelector};
pub use files::{ActivityFolder, detect, filetree, is_activity_file, read_existing_activities};
