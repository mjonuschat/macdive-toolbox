use crate::helpers::mtp::get_raw_devices;

use anyhow::Result;
use libmtp_rs::device::{MtpDevice, StorageSort};

fn device_info(mut device: MtpDevice, verbose: bool) -> Result<()> {
    println!("Device info:");
    println!("   Manufacturer: {}", device.manufacturer_name()?);
    println!("   Model: {}", device.model_name()?);
    println!("   Serial number: {}", device.serial_number()?);

    device.update_storage(StorageSort::NotSorted)?;
    println!("\n   Storage Devices:");
    let storage_pool = device.storage_pool();
    for (_id, storage) in storage_pool.iter() {
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

pub fn detect(verbose: u8) -> Result<()> {
    println!("Listing raw device(s)");
    let raw_devices = get_raw_devices()?;

    println!("   Found {} device(s):", &raw_devices.len());
    for raw_device in raw_devices.iter() {
        let device_entry = raw_device.device_entry();
        println!(
            "   {}: {} ({:04x}:{:04x}) @ bus {}, dev {}",
            &device_entry.vendor,
            &device_entry.product,
            &device_entry.vendor_id,
            &device_entry.product_id,
            &raw_device.bus_number(),
            &raw_device.dev_number(),
        )
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
                println!("Unable to open raw device {}", i)
            }
        }
    }
    Ok(())
}
