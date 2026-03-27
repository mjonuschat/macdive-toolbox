use std::path::Path;

use indicatif::{ProgressBar, ProgressStyle};
use libmtp_rs::{
    device::{MtpDevice, StorageSort},
    object::{Object, filetypes::Filetype},
    storage::{Parent, Storage, files::File},
};
use ptree::item::StringItem;
use walkdir::{DirEntry, WalkDir};

use super::device::{Device, DeviceSelector, get_raw_devices};
use crate::error::{Error, Result};

/// File extensions that are considered activity/fitness files.
const ACTIVITY_FILE_TYPES: [&str; 3] = ["fit", "gpx", "tcx"];

/// Identifies an activity folder on a specific MTP storage unit.
#[derive(Debug)]
pub struct ActivityFolder {
    /// The numeric storage ID within the device's storage pool.
    pub storage_id: u32,
    /// The MTP parent reference pointing to this folder.
    pub parent: Parent,
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
    /// Returns `Ok(Some(file))` when the folder is found, `Ok(None)` when the
    /// path is empty (root), or `Err(MtpFolderNotFound)` when a component
    /// does not exist.
    fn find_folder_recursive<'a>(
        path: &Path,
        storage: &'a Storage,
        folder: Option<File<'a>>,
    ) -> Result<Option<File<'a>>> {
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
                    None => Err(Error::MtpFolderNotFound(
                        component.as_os_str().to_string_lossy().to_string(),
                    )),
                }
            }
            None => Ok(folder),
        }
    }

    /// Locates the folder at `path` across all storage units on this device.
    ///
    /// Prints storage information when the folder is found.
    ///
    /// # Errors
    ///
    /// Returns `Error::MtpFolderNotFound` when no storage contains the folder.
    ///
    /// # Note
    ///
    /// Currently returns the first matching storage; devices with multiple
    /// identical folder paths on different storages are not fully handled.
    pub(super) fn activity_folder(&self, path: &Path) -> Result<ActivityFolder> {
        let storage_pool = self.storage_pool();

        for (i, (_id, storage)) in storage_pool.iter().enumerate() {
            let result = Self::find_folder_recursive(path, storage, None)?;
            if let Some(folder) = result {
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

        Err(Error::MtpFolderNotFound(
            "activity folder not found".to_string(),
        ))
    }

    /// Returns all non-folder activity files located at `path` on the device.
    ///
    /// Only files whose names pass [`is_activity_file`] are included.
    ///
    /// # Errors
    ///
    /// Propagates any error from [`Device::activity_folder`].
    pub fn activity_files(&self, path: &Path) -> Result<Vec<File<'_>>> {
        let folder = self.activity_folder(path)?;
        let files: Vec<File<'_>> = self
            .storage_pool()
            .by_id(folder.storage_id)
            .map(|v| v.files_and_folders(folder.parent))
            .unwrap_or_default();

        Ok(files
            .into_iter()
            .filter(|item| !matches!(item.ftype(), Filetype::Folder))
            .filter(|item| is_activity_file(item.name()))
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Top-level utility functions (formerly utils/detect.rs and utils/filetree.rs)
// ---------------------------------------------------------------------------

/// Detects all attached MTP devices and prints their information.
///
/// `verbose`:
/// - `0` — basic info (manufacturer, model, serial, storage list)
/// - `1` — additional storage details
/// - `2+` — full raw device dump via `dump_device_info`
///
/// # Errors
///
/// Returns an error if the device list cannot be retrieved.
pub fn detect(verbose: u8) -> Result<()> {
    println!("Listing raw device(s)");
    let raw_devices = get_raw_devices()?;

    println!("   Found {} device(s):", raw_devices.len());
    for raw_device in raw_devices.iter() {
        let entry = raw_device.device_entry();
        println!(
            "   {}: {} ({:04x}:{:04x}) @ bus {}, dev {}",
            entry.vendor,
            entry.product,
            entry.vendor_id,
            entry.product_id,
            raw_device.bus_number(),
            raw_device.dev_number(),
        );
    }

    println!("Attempting to connect to device(s)");
    for (i, raw_device) in raw_devices.iter().enumerate() {
        match raw_device.open_uncached() {
            Some(device) => match verbose {
                0 => device_info(device, false)?,
                1 => device_info(device, true)?,
                _ => device.dump_device_info(),
            },
            None => {
                println!("Unable to open raw device {}", i);
            }
        }
    }
    Ok(())
}

fn device_info(mut device: MtpDevice, verbose: bool) -> Result<()> {
    println!("Device info:");
    println!(
        "   Manufacturer: {}",
        device
            .manufacturer_name()
            .map_err(|e| Error::Mtp(e.to_string()))?
    );
    println!(
        "   Model: {}",
        device.model_name().map_err(|e| Error::Mtp(e.to_string()))?
    );
    println!(
        "   Serial number: {}",
        device
            .serial_number()
            .map_err(|e| Error::Mtp(e.to_string()))?
    );

    device
        .update_storage(StorageSort::NotSorted)
        .map_err(|e| Error::Mtp(e.to_string()))?;
    println!("\n   Storage Devices:");
    for (_id, storage) in device.storage_pool().iter() {
        println!("      StorageID: 0x{:08x}", storage.id());
        println!(
            "         StorageDescription: {}",
            storage.description().unwrap_or("(null)")
        );
        println!("         MaxCapacity: {:?}", storage.maximum_capacity());
        if verbose {
            println!("         StorageType: {:?}", storage.storage_type());
            println!("         FilesystemType: {:?}", storage.filesystem_type());
            println!(
                "         AccessCapability: {:?}",
                storage.access_capability()
            );
            println!(
                "         FreeSpaceInBytes: {:?}",
                storage.free_space_in_bytes()
            );
            println!(
                "         FreeSpaceInObjects: {:?}",
                storage.free_space_in_objects()
            );
            println!(
                "         VolumeIdentifier: {}",
                storage.volume_identifier().unwrap_or("(null)")
            );
        }
    }

    Ok(())
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
pub fn filetree(selector: DeviceSelector, verbose: bool) -> Result<()> {
    let device = Device::get(&selector)?;

    for (id, storage) in device.storage_pool().iter() {
        let name = storage
            .description()
            .map_or_else(|| id.to_string(), |v| v.to_owned());

        let spinner = create_spinner(&format!("Scanning {}", &name));

        let result = recursive_file_tree(
            storage,
            Parent::Root,
            format!("Storage: {}", &name),
            verbose,
            &spinner,
        );

        spinner.finish_and_clear();

        match result {
            Some(tree) => ptree::print_tree(&tree).map_err(Error::Io)?,
            None => println!("Storage: {} - no activity files found", &name),
        }
    }

    Ok(())
}

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

fn recursive_file_tree(
    storage: &Storage,
    parent: Parent,
    text: String,
    verbose: bool,
    spinner: &ProgressBar,
) -> Option<StringItem> {
    let files = storage.files_and_folders(parent);
    let mut children: Vec<StringItem> = Vec::new();

    for file in files {
        spinner.tick();
        if matches!(file.ftype(), Filetype::Folder) {
            let result = recursive_file_tree(
                storage,
                Parent::Folder(file.id()),
                file.name().to_string(),
                verbose,
                spinner,
            );
            if let Some(item) = result {
                children.push(item);
            }
        } else if verbose || is_activity_file(file.name()) {
            children.push(StringItem {
                text: file.name().to_string(),
                children: Vec::new(),
            });
        }
    }

    if verbose || !children.is_empty() {
        return Some(StringItem { text, children });
    }

    None
}
