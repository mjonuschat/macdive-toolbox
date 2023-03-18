use crate::errors::PathError;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

const ACTIVITY_FILE_TYPES: [&str; 3] = ["fit", "gpx", "tcx"];

pub fn create_output_dir(path: &Path) -> Result<(), PathError> {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                Ok(())
            } else {
                Err(PathError::Inaccessible(path.to_string_lossy().to_string()))
            }
        }
        Err(_e) => Ok(std::fs::create_dir_all(path)?),
    }
}

pub fn is_activity_file(file: &str) -> bool {
    let extension = Path::new(&file.to_lowercase())
        .extension()
        .and_then(|v| v.to_str().map(|v| v.to_owned()));

    match extension {
        Some(ext) => ACTIVITY_FILE_TYPES.contains(&ext.as_str()),
        None => false,
    }
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

pub fn read_existing_activities(path: &Path) -> Vec<String> {
    WalkDir::new(path)
        .into_iter()
        .filter_entry(is_dir_or_activity)
        .filter_map(|entry| entry.ok())
        .filter(|entry| !entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<String>>()
}
