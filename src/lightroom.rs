use crate::errors::{ConversionError, LightroomTemplateError};
use crate::types::{DecimalToDms, DiveSite};

use std::convert::{TryFrom, TryInto};

use askama::Template;
use google_maps::LatLng;
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

static LRTEMPLATE_ID_RE: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(concat!(
        r#"^\s+id\s=\s"(?P<uuid>[0-9a-f]{8}-[0-9a-f]{4}-"#,
        r#"[1-5][0-9a-f]{3}-[89AB][0-9a-f]{3}-[0-9a-f]{12})",$"#
    ))
    .multi_line(true)
    .case_insensitive(true)
    .build()
    .unwrap()
});

mod filters {
    use std::borrow::Cow;

    /// Basic quoting of backslashes and double quotes in strings
    pub fn quote(s: &str) -> askama::Result<Cow<str>> {
        let mut output = String::with_capacity(s.len() + 4);
        output.push('"');

        for c in s.chars() {
            match c {
                '"' | '\\' => output += &format!("{}", c.escape_default()),
                _ => output.push(c),
            }
        }

        output.push('"');
        Ok(output.into())
    }
}

#[derive(Template)]
#[template(path = "metadata_preset.lrtemplate", escape = "none")]
pub struct MetadataPreset {
    pub id: Uuid,
    pub gps: String,
    pub title: String,
    pub location: String,
    pub city: String,
    pub county: String,
    pub state: String,
    pub country: String,
    pub iso_country_code: String,
    pub scene: u64,
    pub version: u64,
}

impl TryFrom<DiveSite> for MetadataPreset {
    type Error = ConversionError;

    fn try_from(site: DiveSite) -> Result<Self, Self::Error> {
        let latlng: LatLng = site.clone().try_into()?;
        let county = site.county.unwrap_or_else(|| String::from("Unknown"));

        Ok(Self {
            id: site.uuid,
            gps: latlng.to_dms()?,
            title: format!(
                "[Location] {county}: {name}",
                county = &county,
                name = &site.name
            ),
            city: site.locality.unwrap_or_default(),
            county,
            country: site.country,
            iso_country_code: site.iso_country_code,
            location: site.name,
            state: site.state.unwrap_or_default(),
            scene: Default::default(),
            version: 0,
        })
    }
}

impl Default for MetadataPreset {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            gps: r#"0°00'00.0"N 0°00'00.0"E"#.to_string(),
            title: "".to_string(),
            city: "".to_string(),
            county: "".to_string(),
            country: "".to_string(),
            iso_country_code: "".to_string(),
            location: "".to_string(),
            state: "".to_string(),
            scene: 20000917,
            version: 0,
        }
    }
}

pub fn read_existing_presets(
    path: &Path,
) -> Result<HashMap<Uuid, DirEntry>, LightroomTemplateError> {
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
                .ok_or(LightroomTemplateError::Parsing)
                .and_then(|v| {
                    Uuid::parse_str(&v.as_str().to_lowercase())
                        .map_err(LightroomTemplateError::InvalidUuid)
                })?;

            Ok((uuid, entry))
        })
        .collect::<Result<HashMap<Uuid, DirEntry>, LightroomTemplateError>>()
}

pub fn write_preset(path: &Path, content: &str) -> Result<(), LightroomTemplateError> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}

pub fn write_presets(
    path: &Path,
    presets: &[MetadataPreset],
) -> Result<(), LightroomTemplateError> {
    for preset in presets {
        let content = preset.render()?;
        let filename = format!("MacDive-{}.lrtemplate", preset.id);

        write_preset(path.join(filename).as_path(), &content)?
    }

    Ok(())
}
