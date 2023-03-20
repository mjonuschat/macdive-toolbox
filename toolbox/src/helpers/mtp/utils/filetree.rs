use anyhow::Result;
use indicatif::ProgressBar;
use libmtp_rs::object::{filetypes::Filetype, Object};
use libmtp_rs::storage::{Parent, Storage};
use ptree::item::StringItem;

use crate::helpers::mtp::{types::DeviceSelector, Device};
use crate::helpers::{fs, progress};

pub fn filetree(selector: DeviceSelector, verbose: bool) -> Result<()> {
    let device = Device::get(&selector)?;

    for (id, storage) in device.storage_pool().iter() {
        let name = storage
            .description()
            .map_or_else(|| id.to_string(), |v| v.to_owned());

        let spinner = progress::create_spinner(&format!("Scanning {}", &name))?;

        let result = recursive_file_tree(
            storage,
            Parent::Root,
            format!("Storage: {}", &name),
            verbose,
            &spinner,
        );

        spinner.finish_and_clear();

        match result {
            Some(tree) => ptree::print_tree(&tree)?,
            None => println!("Storage: {} - no activity files found", &name),
        }
    }

    Ok(())
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
                children.push(item)
            }
        } else if verbose || fs::is_activity_file(file.name()) {
            children.push(StringItem {
                text: file.name().to_string(),
                children: Vec::new(),
            })
        }
    }

    if verbose || !children.is_empty() {
        return Some(StringItem { text, children });
    }

    None
}
