use std::collections::HashMap;
use std::io::Write as IoWrite;
use std::path::Path;
use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

use crate::error::{Error, Result};

use super::preset::MetadataPreset;

/// Compiled regex for extracting the `id` UUID field from an existing `.lrtemplate` file.
///
/// Matches lines of the form:
/// ```text
///     id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
/// ```
/// and captures the UUID into the named group `uuid`.
static LRTEMPLATE_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(concat!(
        r#"^\s+id\s=\s"(?P<uuid>[0-9a-f]{8}-[0-9a-f]{4}-"#,
        r#"[1-5][0-9a-f]{3}-[89AB][0-9a-f]{3}-[0-9a-f]{12})",$"#
    ))
    .multi_line(true)
    .case_insensitive(true)
    .build()
    // SAFETY: The regex literal is verified at compile time via tests; panic here
    // is an unrecoverable programmer error.
    .expect("LRTEMPLATE_ID_RE is a valid regex")
});

/// Walk `path` recursively and return a map from UUID to [`DirEntry`] for every
/// `.lrtemplate` file found.
///
/// Only files ending with `.lrtemplate` are considered; all other entries are
/// skipped.  Entries whose UUIDs cannot be extracted or parsed are returned as
/// errors.
///
/// # Arguments
///
/// * `path` - Root directory to search for `.lrtemplate` files.
///
/// # Errors
///
/// Returns [`Error::Io`] if a file cannot be read, [`Error::LightroomParsing`]
/// if the UUID field is missing, or [`Error::InvalidUuid`] if the UUID string
/// cannot be parsed.
pub fn read_existing_presets(path: &Path) -> Result<HashMap<Uuid, DirEntry>> {
    /// Returns `true` for directory entries and `.lrtemplate` files so that
    /// `WalkDir::filter_entry` prunes everything else early.
    fn is_dir_or_lrtemplate(entry: &DirEntry) -> bool {
        if entry.path().is_dir() {
            return true;
        }

        entry
            .file_name()
            .to_str()
            .map(|s| s.ends_with(".lrtemplate"))
            .unwrap_or(false)
    }

    WalkDir::new(path)
        .into_iter()
        .filter_entry(is_dir_or_lrtemplate)
        .filter_map(|e| e.ok())
        .filter(|entry| !entry.path().is_dir())
        .map(|entry| {
            let content = std::fs::read_to_string(entry.path())?;
            let uuid = LRTEMPLATE_ID_RE
                .captures(&content)
                .and_then(|v| v.name("uuid"))
                .ok_or(Error::LightroomParsing)
                .and_then(|v| {
                    Uuid::parse_str(&v.as_str().to_lowercase()).map_err(Error::InvalidUuid)
                })?;

            Ok((uuid, entry))
        })
        .collect::<Result<HashMap<Uuid, DirEntry>>>()
}

/// Write a single Lightroom metadata preset to `path`.
///
/// Creates or truncates the file at `path` and writes `content` in UTF-8.
///
/// # Arguments
///
/// * `path` - Destination file path (typically `<presets-dir>/<name>.lrtemplate`).
/// * `content` - Rendered template content to write.
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be created or written.
pub fn write_preset(path: &Path, content: &str) -> Result<()> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Render and write all `presets` into `path`, reusing existing filenames where
/// possible.
///
/// If a preset's UUID already appears in `existing`, its current filename is
/// reused; otherwise a new file named `MacDive-<uuid>.lrtemplate` is created.
///
/// # Arguments
///
/// * `path` - Directory into which preset files are written.
/// * `presets` - Slice of [`MetadataPreset`] values to render and write.
/// * `existing` - Map of known UUID → [`DirEntry`] pairs (from [`read_existing_presets`]).
///
/// # Errors
///
/// Returns [`Error::Template`] if rendering fails, or [`Error::Io`] if writing fails.
pub fn write_presets(
    path: &Path,
    presets: &[MetadataPreset],
    existing: &HashMap<Uuid, DirEntry>,
) -> Result<()> {
    use askama::Template as _;

    for preset in presets {
        let content = preset
            .render()
            .map_err(|e| Error::Template(e.to_string()))?;
        let filename = existing
            .get(&preset.id)
            .and_then(|v| v.file_name().to_str().map(|v| v.to_string()))
            .unwrap_or_else(|| format!("MacDive-{}.lrtemplate", &preset.id));

        write_preset(path.join(filename).as_path(), &content)?;
    }

    Ok(())
}
