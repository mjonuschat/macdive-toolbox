// use anyhow::{bail, Context};
use crate::types::{LocationOverride, Overrides};
use anyhow::Context;
use clap::{AppSettings, Clap, ValueHint};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

static LIGHTROOM_DATA: &str = "Adobe/Lightroom/Metadata Presets/";
static MACDIVE_DATA: &str = "Macdive/MacDive.sqlite";

#[derive(Clap, Debug)]
#[clap(author, about, version, name = "MacDive Dive Site Exporter", setting=AppSettings::ColorAuto, setting=AppSettings::ColoredHelp)]
pub struct Options {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[clap(short, long, parse(from_occurrences))]
    verbose: u8,
    /// Path to the MacDive database file
    #[clap(short, long, parse(from_os_str), value_hint=ValueHint::FilePath)]
    database: Option<PathBuf>,
    /// Path to the Lightroom Settings directory
    #[clap(short, long, parse(from_os_str), value_hint=ValueHint::DirPath)]
    lightroom: Option<PathBuf>,
    /// Path to the Location overrides file
    #[clap(short='o', long, parse(from_os_str), value_hint=ValueHint::FilePath)]
    pub locations: Option<PathBuf>,
    /// Google Maps API key for reverse geocoding
    #[clap(short, long, value_hint=ValueHint::Other)]
    pub api_key: Option<String>,
}

#[derive(Error, Debug)]
pub enum PathError {
    #[error("Path `{0}` could not be resolved")]
    Canonicalize(#[from] std::io::Error),
    #[error("Path to user's data directory could not be detected")]
    DataDir,
    #[error("File or directory `{0}` is not accessible")]
    Inaccessible(String),
}

impl Options {
    fn resolve_path(
        &self,
        path: &Option<PathBuf>,
        data_directory: &str,
    ) -> Result<PathBuf, PathError> {
        let p = match path {
            Some(v) => std::fs::canonicalize(v).map_err(PathError::Canonicalize)?,
            None => dirs::data_dir()
                .ok_or(PathError::DataDir)
                .map(|p| p.join(PathBuf::from(data_directory)))?,
        };

        let _ =
            std::fs::metadata(&p).map_err(|_e| PathError::Inaccessible(p.display().to_string()))?;

        Ok(p)
    }

    pub fn overrides(&self) -> anyhow::Result<Overrides> {
        match &self.locations {
            Some(path) => {
                let c = std::fs::read_to_string(path)
                    .with_context(|| format!("Could not read file {}", &path.display()))?;
                Ok(toml::from_str(&c)?)
            }
            None => Ok(Overrides {
                locations: HashMap::new(),
            }),
        }
    }

    pub fn location_overrides(&self) -> Vec<LocationOverride> {
        self.overrides()
            .map(|v| v.locations.iter().map(|(_, v)| v.clone()).collect())
            .unwrap_or_else(|_| Vec::new())
    }

    pub fn lightroom_metadata(&self) -> Result<PathBuf, PathError> {
        self.resolve_path(&self.lightroom, LIGHTROOM_DATA)
    }

    pub fn macdive_database(&self) -> Result<PathBuf, PathError> {
        self.resolve_path(&self.database, MACDIVE_DATA)
    }
}
