use crate::cli::{MtpOptions, MtpSyncOptions};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use macdive_toolbox_core::services::mtp::{self, Device, DeviceSelector};
use macdive_toolbox_core::util::fs;

pub(crate) async fn detect(verbose: u8) -> Result<()> {
    Ok(mtp::detect(verbose).await?)
}

pub(crate) async fn listfiles(selector: DeviceSelector, verbose: bool) -> Result<()> {
    Ok(mtp::filetree(selector, verbose).await?)
}

pub(crate) async fn sync(config: &MtpOptions, options: &MtpSyncOptions) -> Result<()> {
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
            // Retrieve the storage by ID so we can download from the correct
            // storage unit. The `storage()` call is async because it fetches
            // fresh storage-info metadata from the device.
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
