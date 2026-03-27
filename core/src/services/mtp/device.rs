use libmtp_rs::{
    device::{
        MtpDevice,
        raw::{RawDevice, detect_raw_devices},
    },
    error::{Error as MtpError, MtpErrorKind},
};
use std::ops::Deref;

use crate::error::{Error, Result};

/// Returns all raw MTP devices currently attached to the USB bus.
///
/// # Errors
///
/// Returns `Error::Mtp` if the underlying libmtp call fails or no device is attached.
pub(super) fn get_raw_devices() -> Result<Vec<RawDevice>> {
    detect_raw_devices().map_err(|e| match e {
        MtpError::MtpError { kind, ref text } => match kind {
            MtpErrorKind::NoDeviceAttached => Error::Mtp("no MTP device attached".to_string()),
            _ => Error::Mtp(text.clone()),
        },
        other => Error::Mtp(other.to_string()),
    })
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
#[derive(Debug)]
pub struct Device {
    /// Human-readable name: the device's friendly name if available, otherwise
    /// `"<Manufacturer> <Model>"`.
    pub name: String,
    /// Serial number reported by the device, or `"Unknown"` if unavailable.
    pub serial: String,
    inner: MtpDevice,
}

impl Device {
    /// Wrap an already-opened [`MtpDevice`], eagerly reading its name and serial.
    pub fn new(device: MtpDevice) -> Self {
        Self {
            name: Self::friendly_name(&device),
            serial: Self::serial_number(&device),
            inner: device,
        }
    }

    /// Open a device that matches `selector`.
    ///
    /// When `selector` is [`DeviceSelector::First`] and more than one device is
    /// present, a warning is printed and the first device is returned.
    ///
    /// # Errors
    ///
    /// Returns `Error::Mtp` if:
    /// - no devices are attached,
    /// - a raw device cannot be opened,
    /// - no device matches the selector.
    pub fn get(selector: &DeviceSelector) -> Result<Device> {
        let raw_devices = get_raw_devices()?;

        if raw_devices.len() > 1 && matches!(selector, DeviceSelector::First) {
            println!(
                "Found {} MTP devices, defaulting to first one found.",
                raw_devices.len()
            );
            println!("Please select a specific device using manufacturer/model/serial number");
        }

        for raw_device in raw_devices {
            let opened = raw_device.open_uncached();
            match opened {
                Some(device) => match selector {
                    DeviceSelector::First => return Ok(Self::new(device)),
                    DeviceSelector::ManufacturerName(pattern) => {
                        if let Ok(name) = device.manufacturer_name()
                            && name.contains(pattern)
                        {
                            return Ok(Self::new(device));
                        }
                    }
                    DeviceSelector::ModelName(pattern) => {
                        if let Ok(name) = device.model_name()
                            && name.contains(pattern)
                        {
                            return Ok(Self::new(device));
                        }
                    }
                    DeviceSelector::SerialNumber(pattern) => {
                        if let Ok(serial) = device.serial_number()
                            && serial == *pattern
                        {
                            return Ok(Self::new(device));
                        }
                    }
                },
                None => {
                    let entry = raw_device.device_entry();
                    println!(
                        "Could not open device (Vendor {:04x}, Product {:04x}), skipping...",
                        entry.vendor_id, entry.product_id
                    );
                }
            }
        }

        Err(Error::Mtp(
            "no device matching selection criteria found".to_string(),
        ))
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
}

impl Deref for Device {
    type Target = MtpDevice;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
