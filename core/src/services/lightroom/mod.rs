//! Lightroom metadata preset generation service.
//!
//! Provides types and functions for generating Adobe Lightroom `.lrtemplate`
//! metadata preset files from MacDive dive site data.
//!
//! # Usage
//!
//! ```ignore
//! use macdive_toolbox_core::services::lightroom::{MetadataPreset, read_existing_presets, write_presets};
//! use std::convert::TryInto;
//!
//! let existing = read_existing_presets(&output_dir)?;
//! let presets: Vec<MetadataPreset> = sites
//!     .into_iter()
//!     .map(|s| s.try_into())
//!     .collect::<Result<_>>()?;
//! write_presets(&output_dir, &presets, &existing)?;
//! ```

mod io;
mod preset;

pub use io::{read_existing_presets, write_preset, write_presets};
pub use preset::MetadataPreset;
