use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use clap::{ArgAction, ColorChoice, ValueHint};

use crate::errors::PathError;
use crate::types::{ApplicationConfig, CritterConfig};

static LIGHTROOM_DATA: &str = "Adobe/Lightroom/Metadata Presets/";
static MACDIVE_DATA: &str = "MacDive/MacDive.sqlite";
static ACTIVITY_DIR: &str = "GARMIN/Activity";

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
#[clap(author, about, version, name = "MacDive Dive Site Exporter", color=ColorChoice::Auto)]
pub(crate) struct Cli {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[clap(short, long, action=ArgAction::Count)]
    pub verbose: u8,
    /// Path to the MacDive database file
    #[clap(short, long, value_hint=ValueHint::FilePath)]
    pub database: Option<PathBuf>,
    /// Path to the configuration file
    #[clap(short='c', long, value_hint=ValueHint::FilePath)]
    config: Option<PathBuf>,
    /// Offline mode
    #[clap(short, long, default_value_t = false)]
    pub(crate) offline: bool,
    /// Subcommands
    #[clap(subcommand)]
    pub(crate) command: Commands,
}

impl Cli {
    pub fn macdive_database(&self) -> Result<PathBuf, PathError> {
        resolve_path(&self.database, MACDIVE_DATA)
    }

    pub fn config(&self) -> anyhow::Result<ApplicationConfig> {
        match &self.config {
            Some(path) => {
                let c = std::fs::read_to_string(path)
                    .with_context(|| format!("Could not read config file {}", &path.display()))?;
                Ok(serde_yaml::from_str(&c)?)
            }
            None => Ok(ApplicationConfig {
                locations: HashMap::new(),
                critters: CritterConfig::default(),
            }),
        }
    }
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum Commands {
    Lightroom {
        #[clap(subcommand)]
        command: LightroomCommands,
        #[clap(flatten)]
        options: LightroomOptions,
    },
    Critters {
        #[clap(subcommand)]
        command: CritterCommands,
    },
    Mtp {
        #[clap(subcommand)]
        command: MtpCommands,
        #[clap(flatten)]
        options: MtpOptions,
    },
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum LightroomCommands {
    ExportSites {
        /// Force export and overwrite all existing files
        #[clap(short, long)]
        force: bool,
    },
}

#[derive(Clone, Debug, clap::Args)]
pub(crate) struct LightroomOptions {
    /// Path to the Lightroom Settings directory
    #[clap(short, long, value_hint=ValueHint::DirPath)]
    lightroom: Option<PathBuf>,
    /// Google Maps API key for reverse geocoding
    #[clap(short, long, value_hint=ValueHint::Other)]
    pub(crate) api_key: Option<String>,
}

impl LightroomOptions {
    pub fn lightroom_metadata(&self) -> Result<PathBuf, PathError> {
        resolve_path(&self.lightroom, LIGHTROOM_DATA)
    }
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum CritterCommands {
    Validate,
    ValidateCategories,
    PrepareImport(PrepareImportOptions),
}

#[derive(Debug, clap::Args)]
pub(crate) struct PrepareImportOptions {
    /// File format
    #[clap(long, default_value_t = false)]
    pub(crate) skip_invalid: bool,
    /// File format
    #[clap(short, long, default_value = "xml")]
    #[arg(value_enum)]
    pub(crate) format: MacdiveImportFormat,
    /// Path to the Lightroom Settings directory
    #[clap(short, long, value_hint=ValueHint::DirPath)]
    pub(crate) source: PathBuf,
    /// Path to the Lightroom Settings directory
    #[clap(short, long, value_hint=ValueHint::DirPath)]
    pub(crate) dest: PathBuf,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub(crate) enum MacdiveImportFormat {
    Xml,
    Csv,
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum MtpCommands {
    #[clap(about = "Detect MTP devices")]
    Detect,
    #[clap(about = "Show tree of folders and activity files")]
    ListFiles {
        /// Show all files and folders
        #[clap(long)]
        all: bool,
    },
    Sync(MtpSyncOptions),
}

#[derive(Clone, Debug, clap::Args)]
pub(crate) struct MtpOptions {
    /// Select device by model name
    #[clap(short, long, conflicts_with_all=&["manufacturer", "serial"])]
    pub model: Option<String>,
    /// Select device by manufacturer name
    #[clap(short='a', long, conflicts_with_all=&["model", "serial"])]
    pub manufacturer: Option<String>,
    /// Select device by serial number
    #[clap(short, long, conflicts_with_all=&["model", "manufacturer"])]
    pub serial: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct MtpSyncOptions {
    /// Path to the activity files on the MTP device
    #[clap(short, long, value_hint=ValueHint::DirPath, default_value = ACTIVITY_DIR)]
    pub input: PathBuf,
    /// Path to where the downloaded activities are being written
    #[clap(short, long, value_hint = ValueHint::DirPath, default_value = ".")]
    pub output: PathBuf,
    /// Force export and overwrite all existing files
    #[clap(short, long)]
    pub force: bool,
}

impl MtpSyncOptions {
    pub fn activity_dir(&self) -> PathBuf {
        self.input.to_owned()
    }
}
