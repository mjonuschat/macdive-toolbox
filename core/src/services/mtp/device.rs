use std::fmt;

use mtp_rs::mtp::{MtpDevice, MtpDeviceInfo};

use crate::error::{Error, Result};

/// Maps an `mtp_rs::Error` into our crate-level `Error::Mtp`.
///
/// Checks `is_exclusive_access()` to surface a macOS-specific diagnostic
/// message when `ptpcamerad` claims the USB interface.
fn map_mtp_error(e: mtp_rs::Error) -> Error {
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

/// Returns all MTP devices currently visible on the USB bus without opening them.
///
/// # Errors
///
/// Returns `Error::Mtp` if the underlying USB enumeration fails or no device
/// is attached.
pub(super) fn list_devices() -> Result<Vec<MtpDeviceInfo>> {
    let devices = MtpDevice::list_devices().map_err(map_mtp_error)?;
    if devices.is_empty() {
        return Err(Error::Mtp("no MTP device attached".to_string()));
    }
    Ok(devices)
}

/// Selects which MTP device to open when multiple devices are attached.
#[derive(Debug, Clone)]
pub enum DeviceSelector {
    /// Open the first device found on the USB bus.
    First,
    /// Open the device whose manufacturer name contains the given string.
    ManufacturerName(String),
    /// Open the device whose model name contains the given string.
    ModelName(String),
    /// Open the device whose serial number matches exactly.
    SerialNumber(String),
}

/// An opened MTP device with its friendly name and serial number pre-fetched.
///
/// Wraps [`MtpDevice`] and eagerly reads the device info so callers can
/// access `name` and `serial` without additional I/O.
pub struct Device {
    /// Human-readable name: the device model if available, otherwise
    /// `"<Manufacturer> <Model>"` with fallbacks to `"Unknown"`.
    pub name: String,
    /// Serial number reported by the device, or `"Unknown"` if unavailable.
    pub serial: String,
    /// The underlying mtp-rs device handle.
    inner: MtpDevice,
}

// MtpDevice does not implement Debug, so we provide a manual impl that
// prints the pre-fetched name and serial instead.
impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("name", &self.name)
            .field("serial", &self.serial)
            .finish_non_exhaustive()
    }
}

impl Device {
    /// Wrap an already-opened [`MtpDevice`], eagerly reading its name and serial.
    pub fn new(device: MtpDevice) -> Self {
        let info = device.device_info();
        let name = if info.model.is_empty() {
            let manufacturer = if info.manufacturer.is_empty() {
                "Unknown"
            } else {
                &info.manufacturer
            };
            let model = if info.model.is_empty() {
                "Unknown"
            } else {
                &info.model
            };
            format!("{} {}", manufacturer, model)
        } else {
            info.model.clone()
        };
        let serial = if info.serial_number.is_empty() {
            "Unknown".to_string()
        } else {
            info.serial_number.clone()
        };

        Self {
            name,
            serial,
            inner: device,
        }
    }

    /// Open a device that matches `selector`.
    ///
    /// When `selector` is [`DeviceSelector::First`] and more than one device is
    /// present, a warning is logged and the first device is returned.
    ///
    /// # Errors
    ///
    /// Returns `Error::Mtp` if:
    /// - no devices are attached,
    /// - a device cannot be opened,
    /// - no device matches the selector.
    pub async fn get(selector: &DeviceSelector) -> Result<Device> {
        let device_infos = list_devices()?;

        if device_infos.len() > 1 && matches!(selector, DeviceSelector::First) {
            tracing::warn!(
                count = device_infos.len(),
                "Found {} MTP devices, defaulting to first one found. \
                 Please select a specific device using manufacturer/model/serial number",
                device_infos.len()
            );
        }

        for info in &device_infos {
            // Pre-open filtering: check fields from MtpDeviceInfo (Option<String>)
            // before paying the cost of opening the device.
            let matches_selector = match selector {
                DeviceSelector::First => true,
                DeviceSelector::ManufacturerName(pattern) => info
                    .manufacturer
                    .as_ref()
                    .is_some_and(|name| name.contains(pattern.as_str())),
                DeviceSelector::ModelName(pattern) => info
                    .product
                    .as_ref()
                    .is_some_and(|name| name.contains(pattern.as_str())),
                DeviceSelector::SerialNumber(pattern) => {
                    info.serial_number.as_ref().is_some_and(|sn| sn == pattern)
                }
            };

            if !matches_selector {
                continue;
            }

            match MtpDevice::open_by_location(info.location_id).await {
                Ok(device) => return Ok(Self::new(device)),
                Err(e) => {
                    tracing::warn!(
                        vendor_id = info.vendor_id,
                        product_id = info.product_id,
                        error = %e,
                        "Could not open device (Vendor {:04x}, Product {:04x}), skipping",
                        info.vendor_id,
                        info.product_id
                    );
                }
            }
        }

        Err(Error::Mtp(
            "no device matching selection criteria found".to_string(),
        ))
    }

    /// Returns a reference to the inner [`MtpDevice`] for direct access to
    /// storages and other low-level operations.
    pub fn inner(&self) -> &MtpDevice {
        &self.inner
    }
}
