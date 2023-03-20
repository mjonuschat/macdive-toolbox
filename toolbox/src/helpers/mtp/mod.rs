pub(crate) mod types;
mod utils;

use std::ops::Deref;
use std::path::Path;

use libmtp_rs::{
    device::{
        raw::{detect_raw_devices, RawDevice},
        MtpDevice,
    },
    error::{Error as MtpError, MtpErrorKind},
    object::{filetypes::Filetype, Object},
    storage::{files::File, Parent, Storage},
};

pub use utils::{detect, filetree};

use crate::errors::{MtpDeviceError, MtpStorageError};
use crate::helpers::fs;
use types::DeviceSelector;

pub(in crate::helpers::mtp) fn get_raw_devices() -> Result<Vec<RawDevice>, MtpDeviceError> {
    detect_raw_devices().map_err(|e| match e {
        MtpError::Unknown => MtpDeviceError::LibMtpError(e),
        MtpError::Utf8Error { .. } => MtpDeviceError::LibMtpError(e),
        MtpError::MtpError { kind, .. } => match kind {
            MtpErrorKind::NoDeviceAttached => MtpDeviceError::NoDeviceAttached,
            _ => MtpDeviceError::LibMtpError(e),
        },
    })
}

#[derive(Debug)]
pub struct Device {
    pub name: String,
    pub serial: String,
    inner: MtpDevice,
}

#[derive(Debug)]
pub struct ActivityFolder {
    pub storage_id: u32,
    pub parent: Parent,
}

impl Device {
    pub fn new(device: MtpDevice) -> Self {
        Self {
            name: Self::friendly_name(&device),
            serial: Self::serial_number(&device),
            inner: device,
        }
    }

    pub fn get(selector: &DeviceSelector) -> Result<Device, MtpDeviceError> {
        let raw_devices = get_raw_devices()?;

        if raw_devices.len() > 1 && matches!(selector, DeviceSelector::First) {
            println!(
                "Found {} MTP devices, defaulting to first one found.",
                raw_devices.len()
            );
            println!("Please select a specific device using manufacturer/model/serial number");
        }

        for raw_device in raw_devices {
            if let Some(device) = raw_device.open_uncached() {
                match selector {
                    DeviceSelector::First => return Ok(Self::new(device)),
                    DeviceSelector::ManufacturerName(ref pattern) => {
                        if let Ok(name) = device.manufacturer_name() {
                            if name.contains(pattern) {
                                return Ok(Self::new(device));
                            }
                        }
                    }
                    DeviceSelector::ModelName(ref pattern) => {
                        if let Ok(name) = device.model_name() {
                            if name.contains(pattern) {
                                return Ok(Self::new(device));
                            }
                        }
                    }
                    DeviceSelector::SerialNumber(ref pattern) => {
                        if let Ok(serial) = device.serial_number() {
                            if serial == *pattern {
                                return Ok(Self::new(device));
                            }
                        }
                    }
                }
            } else {
                let device = raw_device.device_entry();
                println!(
                    "Could not open device (Vendor {:04x}, Product {:04x}), skipping...",
                    device.vendor_id, device.product_id
                )
            }
        }

        Err(MtpDeviceError::DeviceNotFound)
    }

    fn friendly_name(device: &MtpDevice) -> String {
        match device.get_friendly_name() {
            Ok(fname) => fname,
            Err(_) => format!(
                "{} {}",
                device
                    .manufacturer_name()
                    .unwrap_or_else(|_| "Unknown".to_string()),
                device
                    .model_name()
                    .unwrap_or_else(|_| "Unknown".to_string())
            ),
        }
    }

    fn serial_number(device: &MtpDevice) -> String {
        device
            .serial_number()
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    fn find_folder_recursive<'a>(
        path: &Path,
        storage: &'a Storage,
        folder: Option<File<'a>>,
    ) -> Result<Option<File<'a>>, MtpStorageError> {
        let parent = folder
            .as_ref()
            .map_or(Parent::Root, |f| Parent::Folder(f.id()));
        let mut components = path.components();

        match components.next() {
            Some(component) => {
                let mut targets = storage
                    .files_and_folders(parent)
                    .into_iter()
                    .filter(|entry| {
                        matches!(entry.ftype(), Filetype::Folder)
                            && entry.name() == component.as_os_str()
                    });

                match targets.next() {
                    Some(target) => {
                        Self::find_folder_recursive(components.as_path(), storage, Some(target))
                    }
                    None => Err(MtpStorageError::FolderNotFound(
                        component.as_os_str().to_string_lossy().to_string(),
                    )),
                }
            }
            None => Ok(folder),
        }
    }

    // TODO: Handle multiple storages with identical folders
    fn activity_folder(&self, path: &Path) -> Result<ActivityFolder, MtpStorageError> {
        let storage_pool = self.storage_pool();

        for (i, (_id, storage)) in storage_pool.iter().enumerate() {
            // Find activity folder
            if let Some(folder) = Self::find_folder_recursive(path, storage, None)? {
                println!(
                    "Found {} folder on Storage {}:",
                    path.to_string_lossy(),
                    i + 1
                );
                println!(
                    "  Description: {}",
                    storage.description().unwrap_or("Unknown")
                );
                println!(
                    "  Max. capacity: {}",
                    bytefmt::format(storage.maximum_capacity())
                );
                println!(
                    "  Free space: {}",
                    bytefmt::format(storage.free_space_in_bytes())
                );
                return Ok(ActivityFolder {
                    storage_id: storage.id(),
                    parent: Parent::Folder(folder.id()),
                });
            }
        }

        Err(MtpStorageError::FolderNotFound(
            "Activity folder not found".to_string(),
        ))
    }

    pub fn activity_files(&self, path: &Path) -> Result<Vec<File>, MtpStorageError> {
        let folder = self.activity_folder(path)?;
        let files: Vec<File> = self
            .storage_pool()
            .by_id(folder.storage_id)
            .map(|v| v.files_and_folders(folder.parent))
            .unwrap_or_else(Vec::new);

        Ok(files
            .into_iter()
            .filter(|item| !matches!(item.ftype(), Filetype::Folder))
            .filter(|item| fs::is_activity_file(item.name()))
            .collect())
    }
}

impl Deref for Device {
    type Target = MtpDevice;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
