use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use clap::{AppSettings, ColorChoice, ValueHint};

use crate::errors::PathError;
use crate::types::{CritterCategoryOverride, LocationOverride, Overrides};

static LIGHTROOM_DATA: &str = "Adobe/Lightroom/Metadata Presets/";
static MACDIVE_DATA: &str = "MacDive/MacDive.sqlite";

fn resolve_path(path: &Option<PathBuf>, data_directory: &str) -> Result<PathBuf, PathError> {
    let p = match path {
        Some(v) => std::fs::canonicalize(v).map_err(PathError::Canonicalize)?,
        None => dirs::data_dir()
            .ok_or(PathError::DataDir)
            .map(|p| p.join(PathBuf::from(data_directory)))?,
    };

    let _ = std::fs::metadata(&p).map_err(|_e| PathError::Inaccessible(p.display().to_string()))?;

    Ok(p)
}

#[derive(clap::Parser, Debug)]
#[clap(author, about, version, name = "MacDive Dive Site Exporter", color=ColorChoice::Auto, setting=AppSettings::ColoredHelp)]
pub(crate) struct Cli {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,
    /// Path to the MacDive database file
    #[clap(short, long, parse(from_os_str), value_hint=ValueHint::FilePath)]
    pub database: Option<PathBuf>,
    #[clap(subcommand)]
    pub(crate) command: Commands,
    /// Path to the Location overrides file
    #[clap(short='c', long, parse(from_os_str), value_hint=ValueHint::FilePath)]
    config: Option<PathBuf>,
}

impl Cli {
    pub fn macdive_database(&self) -> Result<PathBuf, PathError> {
        resolve_path(&self.database, MACDIVE_DATA)
    }

    pub fn overrides(&self) -> anyhow::Result<Overrides> {
        match &self.config {
            Some(path) => {
                let c = std::fs::read_to_string(path)
                    .with_context(|| format!("Could not read file {}", &path.display()))?;
                Ok(serde_yaml::from_str(&c)?)
            }
            None => Ok(Overrides {
                locations: HashMap::new(),
                critter_categories: CritterCategoryOverride::default(),
            }),
        }
    }
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum Commands {
    LightroomMetadata(LightroomOptions),
    DiffCritters,
    DiffCritterCategories,
}

#[derive(Debug, clap::Args)]
#[clap(args_conflicts_with_subcommands = true)]
pub(crate) struct LightroomOptions {
    /// Path to the Lightroom Settings directory
    #[clap(short, long, parse(from_os_str), value_hint=ValueHint::DirPath)]
    lightroom: Option<PathBuf>,
    /// Google Maps API key for reverse geocoding
    #[clap(short, long, value_hint=ValueHint::Other)]
    pub(crate) api_key: Option<String>,
    /// Force export and overwrite all existing files
    #[clap(short, long)]
    pub(crate) force: bool,
}

impl LightroomOptions {
    pub fn lightroom_metadata(&self) -> Result<PathBuf, PathError> {
        resolve_path(&self.lightroom, LIGHTROOM_DATA)
    }
}
