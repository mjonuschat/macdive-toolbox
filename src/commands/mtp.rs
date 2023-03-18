use crate::arguments::{MtpOptions, MtpSyncOptions};
use crate::helpers::fs;
use crate::helpers::mtp;
use crate::helpers::mtp::Device;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

pub(crate) fn detect(verbose: u8) -> Result<()> {
    mtp::detect(verbose)
}

pub(crate) fn listfiles(selector: mtp::types::DeviceSelector, verbose: bool) -> Result<()> {
    mtp::filetree(selector, verbose)
}

pub(crate) fn sync(config: &MtpOptions, options: &MtpSyncOptions) -> Result<()> {
    let device = Device::get(&config.to_owned().into())?;
    let dst_folder = options
        .output
        .join(format!("{} - {}", &device.name, &device.serial));

    let storage = device.storage_pool();
    let files = device.activity_files(&options.activity_dir())?;

    let total_progress = ProgressBar::new(files.len() as u64);
    total_progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos:>7}/{len:7} {msg}")?,
    );

    fs::create_output_dir(&dst_folder)?;
    let existing = fs::read_existing_activities(&dst_folder);

    for file in files {
        total_progress.set_message(file.name().to_owned());

        if !existing.contains(&file.name().to_string()) {
            let dst = dst_folder.join(file.name());

            storage.get_file_to_path(file, dst)?;
        }

        total_progress.inc(1);
    }
    total_progress.finish();

    Ok(())
}
