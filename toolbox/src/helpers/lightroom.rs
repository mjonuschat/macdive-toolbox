//! Re-exports the Lightroom service from `macdive-toolbox-core`.
//!
//! All implementation has moved to `core::services::lightroom`.
pub use macdive_toolbox_core::services::lightroom::{
    MetadataPreset, read_existing_presets, write_presets,
};
