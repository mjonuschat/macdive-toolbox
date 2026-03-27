use std::path::Path;

use indicatif::{ProgressBar, ProgressStyle};
use mtp_rs::mtp::Storage;
use mtp_rs::ptp::{ObjectHandle, ObjectInfo, StorageId};
use ptree::item::StringItem;
use walkdir::{DirEntry, WalkDir};

use super::device::{Device, DeviceSelector, list_devices};
use crate::error::{Error, Result};

/// File extensions that are considered activity/fitness files.
const ACTIVITY_FILE_TYPES: [&str; 3] = ["fit", "gpx", "tcx"];

/// Identifies an activity folder on a specific MTP storage unit.
#[derive(Debug)]
pub struct ActivityFolder {
    /// The storage ID within the device's storage pool.
    pub storage_id: StorageId,
    /// The object handle of the folder (used as parent when listing contents).
    pub handle: ObjectHandle,
}

// ---------------------------------------------------------------------------
// Activity file helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the file name has an activity file extension (fit/gpx/tcx),
/// case-insensitively.
///
/// # Examples
///
/// ```
/// use macdive_toolbox_core::services::mtp::is_activity_file;
///
/// assert!(is_activity_file("track.FIT"));
/// assert!(!is_activity_file("photo.jpg"));
/// ```
pub fn is_activity_file(file: &str) -> bool {
    let extension = Path::new(&file.to_lowercase())
        .extension()
        .and_then(|v| v.to_str().map(|s| s.to_owned()));

    match extension {
        Some(ext) => ACTIVITY_FILE_TYPES.contains(&ext.as_str()),
        None => false,
    }
}

/// Walks `path` recursively and returns the file names of all activity files found.
///
/// Only files with activity extensions (fit/gpx/tcx) are included; directories
/// are traversed but not returned.
pub fn read_existing_activities(path: &Path) -> Vec<String> {
    WalkDir::new(path)
        .into_iter()
        .filter_entry(is_dir_or_activity)
        .filter_map(|entry| entry.ok())
        .filter(|entry| !entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect()
}

/// Helper for [`read_existing_activities`]: keeps directories and activity files.
fn is_dir_or_activity(entry: &DirEntry) -> bool {
    if entry.path().is_dir() {
        return true;
    }
    entry
        .file_name()
        .to_str()
        .map(is_activity_file)
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// MTP folder / file traversal on a Device
// ---------------------------------------------------------------------------

impl Device {
    /// Recursively searches `storage` for the folder described by `path`.
    ///
    /// Returns `Ok(Some(handle))` when the folder is found, `Ok(None)` when the
    /// path is empty (root), or `Err(MtpFolderNotFound)` when a path component
    /// does not exist.
    ///
    /// Uses `Box::pin` because recursive async functions require indirection
    /// to produce a finite-sized future.
    fn find_folder_recursive<'a>(
        path: &'a Path,
        storage: &'a Storage,
        parent: Option<ObjectHandle>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<ObjectHandle>>> + Send + 'a>,
    > {
        Box::pin(async move {
            let mut components = path.components();

            match components.next() {
                Some(component) => {
                    let entries = storage
                        .list_objects(parent)
                        .await
                        .map_err(|e| Error::Mtp(e.to_string()))?;

                    let target = entries.into_iter().find(|entry| {
                        entry.is_folder()
                            && entry.filename == component.as_os_str().to_string_lossy()
                    });

                    match target {
                        Some(folder) => {
                            Self::find_folder_recursive(
                                components.as_path(),
                                storage,
                                Some(folder.handle),
                            )
                            .await
                        }
                        None => Err(Error::MtpFolderNotFound(
                            component.as_os_str().to_string_lossy().to_string(),
                        )),
                    }
                }
                None => Ok(parent),
            }
        })
    }

    /// Locates the folder at `path` across all storage units on this device.
    ///
    /// Logs storage information when the folder is found.
    ///
    /// # Errors
    ///
    /// Returns `Error::MtpFolderNotFound` when no storage contains the folder.
    ///
    /// # Note
    ///
    /// Currently returns the first matching storage; devices with multiple
    /// identical folder paths on different storages are not fully handled.
    pub(super) async fn activity_folder(&self, path: &Path) -> Result<ActivityFolder> {
        let storages = self
            .inner()
            .storages()
            .await
            .map_err(|e| Error::Mtp(e.to_string()))?;

        for (i, storage) in storages.iter().enumerate() {
            let result = Self::find_folder_recursive(path, storage, None).await?;
            if let Some(handle) = result {
                let info = storage.info();
                tracing::info!(
                    path = %path.to_string_lossy(),
                    storage_index = i + 1,
                    description = %info.description,
                    max_capacity = %bytefmt::format(info.max_capacity),
                    free_space = %bytefmt::format(info.free_space_bytes),
                    "Found activity folder on storage"
                );
                return Ok(ActivityFolder {
                    storage_id: storage.id(),
                    handle,
                });
            }
        }

        Err(Error::MtpFolderNotFound(
            "activity folder not found".to_string(),
        ))
    }

    /// Returns all non-folder activity files located at `path` on the device.
    ///
    /// Only files whose names pass [`is_activity_file`] are included.
    /// Returns a vector of `(ObjectInfo, StorageId)` tuples so callers can
    /// download files from the correct storage.
    ///
    /// # Errors
    ///
    /// Propagates any error from [`Device::activity_folder`] or storage listing.
    pub async fn activity_files(&self, path: &Path) -> Result<Vec<(ObjectInfo, StorageId)>> {
        let folder = self.activity_folder(path).await?;
        let storages = self
            .inner()
            .storages()
            .await
            .map_err(|e| Error::Mtp(e.to_string()))?;

        let storage = storages
            .into_iter()
            .find(|s| s.id() == folder.storage_id)
            .ok_or_else(|| Error::Mtp("storage disappeared after folder lookup".to_string()))?;

        let objects = storage
            .list_objects(Some(folder.handle))
            .await
            .map_err(|e| Error::Mtp(e.to_string()))?;

        let storage_id = storage.id();
        Ok(objects
            .into_iter()
            .filter(|obj| obj.is_file())
            .filter(|obj| is_activity_file(&obj.filename))
            .map(|obj| (obj, storage_id))
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Top-level utility functions
// ---------------------------------------------------------------------------

/// Detects all attached MTP devices and prints their information.
///
/// `verbose`:
/// - `0` -- basic info (manufacturer, model, serial, storage list)
/// - `1` -- additional storage details
/// - `2+` -- all device info fields
///
/// # Errors
///
/// Returns an error if the device list cannot be retrieved or a device cannot
/// be opened.
/// Information about a detected MTP device and its storages.
#[derive(Debug)]
pub struct DetectedDevice {
    /// Manufacturer name.
    pub manufacturer: String,
    /// Model/product name.
    pub model: String,
    /// Device serial number.
    pub serial_number: String,
    /// Firmware/device version string.
    pub device_version: String,
    /// Storage units on the device.
    pub storages: Vec<DetectedStorage>,
}

/// Information about a single storage unit on a detected device.
#[derive(Debug)]
pub struct DetectedStorage {
    /// Human-readable storage description.
    pub description: String,
    /// Maximum capacity in bytes.
    pub max_capacity: u64,
    /// Free space in bytes.
    pub free_space_bytes: u64,
    /// Storage type (e.g. "FixedRam", "RemovableRam").
    pub storage_type: String,
    /// Filesystem type (e.g. "GenericHierarchical").
    pub filesystem_type: String,
    /// Volume identifier.
    pub volume_identifier: String,
}

/// Result of attempting to open a single detected USB device.
#[derive(Debug)]
pub enum DeviceDetectionResult {
    /// Successfully opened and queried the device.
    Connected(DetectedDevice),
    /// Could not open the device.
    Failed {
        /// USB vendor ID.
        vendor_id: u16,
        /// USB product ID.
        product_id: u16,
        /// Error message.
        error: String,
    },
}

/// Detect all connected MTP devices and return their information.
///
/// Enumerates USB devices, attempts to open each as an MTP device,
/// and returns device/storage metadata for display by the caller.
pub async fn detect_devices() -> Result<Vec<DeviceDetectionResult>> {
    let device_infos = list_devices()?;
    let mut results = Vec::new();

    for info in &device_infos {
        match mtp_rs::mtp::MtpDevice::open_by_location(info.location_id).await {
            Ok(device) => {
                let dev_info = device.device_info();
                let storages = device.storages().await.map_err(super::map_mtp_error)?;

                let storage_infos = storages
                    .iter()
                    .map(|s| {
                        let si = s.info();
                        DetectedStorage {
                            description: si.description.clone(),
                            max_capacity: si.max_capacity,
                            free_space_bytes: si.free_space_bytes,
                            storage_type: format!("{:?}", si.storage_type),
                            filesystem_type: format!("{:?}", si.filesystem_type),
                            volume_identifier: si.volume_identifier.clone(),
                        }
                    })
                    .collect();

                results.push(DeviceDetectionResult::Connected(DetectedDevice {
                    manufacturer: dev_info.manufacturer.clone(),
                    model: dev_info.model.clone(),
                    serial_number: dev_info.serial_number.clone(),
                    device_version: dev_info.device_version.clone(),
                    storages: storage_infos,
                }));
            }
            Err(e) => {
                let err = super::map_mtp_error(e);
                results.push(DeviceDetectionResult::Failed {
                    vendor_id: info.vendor_id,
                    product_id: info.product_id,
                    error: err.to_string(),
                });
            }
        }
    }

    Ok(results)
}

/// Opens the device matching `selector` and displays its full file tree.
///
/// When `verbose` is `false`, only folders that contain (directly or
/// transitively) at least one activity file are shown.
///
/// # Errors
///
/// Returns an error if the device cannot be opened or the tree cannot be
/// printed.
pub async fn filetree(selector: DeviceSelector, verbose: bool) -> Result<()> {
    let device = Device::get(&selector).await?;

    let storages = device
        .inner()
        .storages()
        .await
        .map_err(|e| Error::Mtp(e.to_string()))?;

    for storage in &storages {
        let si = storage.info();
        let name = if si.description.is_empty() {
            format!("{:?}", storage.id())
        } else {
            si.description.clone()
        };

        let spinner = create_spinner(&format!("Scanning {}", &name));

        let result = recursive_file_tree(
            storage,
            None,
            format!("Storage: {}", &name),
            verbose,
            &spinner,
        )
        .await;

        spinner.finish_and_clear();

        match result {
            Ok(Some(tree)) => ptree::print_tree(&tree).map_err(Error::Io)?,
            Ok(None) => {
                tracing::info!(
                    storage = %name,
                    "Storage: {} - no activity files found",
                    &name
                );
            }
            Err(e) => {
                tracing::warn!(
                    storage = %name,
                    error = %e,
                    "Error scanning storage: {}",
                    &name
                );
            }
        }
    }

    Ok(())
}

/// Creates a spinner progress bar with the given message.
fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ])
            .template("{msg} {spinner:.blue}")
            // ProgressStyle::template only fails on invalid template syntax;
            // the template above is validated and safe.
            .expect("spinner template is valid"),
    );
    pb.set_message(msg.to_owned());
    pb
}

/// Recursively builds a ptree `StringItem` for a folder's contents.
///
/// Returns `Ok(None)` when not in verbose mode and the subtree contains no
/// activity files (used to prune empty branches from the display).
///
/// Uses `Box::pin` because recursive async functions require indirection
/// to produce a finite-sized future.
fn recursive_file_tree<'a>(
    storage: &'a Storage,
    parent: Option<ObjectHandle>,
    text: String,
    verbose: bool,
    spinner: &'a ProgressBar,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<StringItem>>> + Send + 'a>> {
    Box::pin(async move {
        let objects = storage
            .list_objects(parent)
            .await
            .map_err(|e| Error::Mtp(e.to_string()))?;

        let mut children: Vec<StringItem> = Vec::new();

        for obj in objects {
            spinner.tick();
            if obj.is_folder() {
                let result = recursive_file_tree(
                    storage,
                    Some(obj.handle),
                    obj.filename.clone(),
                    verbose,
                    spinner,
                )
                .await?;
                if let Some(item) = result {
                    children.push(item);
                }
            } else if verbose || is_activity_file(&obj.filename) {
                children.push(StringItem {
                    text: obj.filename.clone(),
                    children: Vec::new(),
                });
            }
        }

        if verbose || !children.is_empty() {
            return Ok(Some(StringItem { text, children }));
        }

        Ok(None)
    })
}
