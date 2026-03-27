/// Re-exports of the geocoding service from `macdive-toolbox-core`.
///
/// All geocoding logic now lives in `macdive_toolbox_core::services::geocoding`.
/// This module exists purely as a compatibility shim so existing call sites in
/// the toolbox crate do not need to change their import paths.
pub use macdive_toolbox_core::services::geocoding::{apply_overrides, geocode_site};
