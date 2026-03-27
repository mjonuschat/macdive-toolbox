use crate::cli::{MtpOptions, MtpSyncOptions};
use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use macdive_toolbox_core::services::mtp::{
    self, DetectedDevice, DetectedStorage, Device, DeviceDetectionResult, DeviceSelector,
};
use macdive_toolbox_core::util::fs;

/// Check if macOS ptpcamerad is running, which races with MTP device access.
#[cfg(target_os = "macos")]
fn warn_if_ptpcamerad_running() {
    use std::process::Command;
    let output = Command::new("pgrep").arg("-x").arg("ptpcamerad").output();
    if let Ok(result) = output
        && result.status.success()
    {
        eprintln!(
            "{} macOS ptpcamerad is running and may interfere with MTP device access.",
            style("Warning:").yellow().bold(),
        );
        eprintln!("  To stop it: {}", style("killall ptpcamerad").cyan(),);
        eprintln!();
    }
}

#[cfg(not(target_os = "macos"))]
fn warn_if_ptpcamerad_running() {}

pub(crate) async fn detect(verbose: u8) -> Result<()> {
    warn_if_ptpcamerad_running();

    let results = mtp::detect_devices().await?;

    if results.is_empty() {
        println!("No MTP devices detected.");
        if cfg!(target_os = "macos") {
            println!(
                "\n{} If a device is connected, macOS ptpcamerad may be claiming it.",
                style("Hint:").dim(),
            );
            println!("  Try: {}", style("killall ptpcamerad").cyan(),);
        }
        return Ok(());
    }

    println!(
        "{}",
        style(format!("Found {} device(s):", results.len())).bold()
    );
    println!();

    for (i, result) in results.iter().enumerate() {
        match result {
            DeviceDetectionResult::Connected(device) => {
                print_device(i, device, verbose);
            }
            DeviceDetectionResult::Failed {
                vendor_id,
                product_id,
                error,
            } => {
                println!(
                    "  {} Device {:04x}:{:04x} -- {}",
                    style(format!("[{}]", i + 1)).dim(),
                    vendor_id,
                    product_id,
                    style(error).red(),
                );
            }
        }
    }

    Ok(())
}

/// Format and print a single detected device.
fn print_device(index: usize, device: &DetectedDevice, verbose: u8) {
    println!(
        "  {} {} {}",
        style(format!("[{}]", index + 1)).dim(),
        style(&device.model).green().bold(),
        style(format!("({})", &device.manufacturer)).dim(),
    );
    println!("      Serial: {}", style(&device.serial_number).cyan());

    if verbose >= 2 {
        println!("      Firmware: {}", &device.device_version);
    }

    if device.storages.is_empty() {
        println!("      {}", style("No storage found").yellow());
    } else {
        for storage in &device.storages {
            print_storage(storage, verbose);
        }
    }
    println!();
}

/// Format and print a single storage unit.
fn print_storage(storage: &DetectedStorage, verbose: u8) {
    let capacity = bytefmt::format(storage.max_capacity);
    let free = bytefmt::format(storage.free_space_bytes);

    println!(
        "      Storage: {} ({} total, {} free)",
        style(&storage.description).bold(),
        capacity,
        style(free).green(),
    );

    if verbose >= 1 {
        println!("        Type: {}", storage.storage_type);
        println!("        Filesystem: {}", storage.filesystem_type);
        if !storage.volume_identifier.is_empty() {
            println!("        Volume: {}", storage.volume_identifier);
        }
    }
}

pub(crate) async fn listfiles(selector: DeviceSelector, verbose: bool) -> Result<()> {
    warn_if_ptpcamerad_running();
    Ok(mtp::filetree(selector, verbose).await?)
}

pub(crate) async fn sync(config: &MtpOptions, options: &MtpSyncOptions) -> Result<()> {
    warn_if_ptpcamerad_running();
    let device = Device::get(&config.to_owned().into()).await?;
    let dst_folder = options
        .output
        .join(format!("{} - {}", &device.name, &device.serial));

    let files = device.activity_files(&options.activity_dir()).await?;

    let total_progress = ProgressBar::new(files.len() as u64);
    total_progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos:>7}/{len:7} {msg}")?,
    );

    fs::create_dir(&dst_folder)?;
    let existing = mtp::read_existing_activities(&dst_folder);

    for (obj, storage_id) in files {
        total_progress.set_message(obj.filename.clone());

        if !existing.contains(&obj.filename) {
            let dst = dst_folder.join(&obj.filename);
            let storage = device
                .inner()
                .storage(storage_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to access storage: {}", e))?;
            let data = storage
                .download(obj.handle)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to download {}: {}", &obj.filename, e))?;
            std::fs::write(&dst, &data)?;
        }

        total_progress.inc(1);
    }
    total_progress.finish();

    Ok(())
}
