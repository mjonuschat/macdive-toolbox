use std::convert::TryFrom;

use askama::Template;
use google_maps::LatLng;
use uuid::Uuid;

use crate::domain::{DecimalToDms, DiveSite};
use crate::error::{Error, Result};

/// Custom Askama template filters for Lightroom preset rendering.
pub(super) mod filters {
    /// Wraps a string value in quotes, escaping inner quotes and backslashes.
    ///
    /// This is required for the Lightroom `.lrtemplate` file format, which uses
    /// Lua-style table syntax where string values must be double-quoted.
    ///
    /// # Examples
    ///
    /// A string like `Hello "World"` becomes `"Hello \"World\""`.
    #[askama::filter_fn]
    pub fn quote(s: impl std::fmt::Display, _: &dyn askama::Values) -> askama::Result<String> {
        let s = s.to_string();
        let mut output = String::with_capacity(s.len() + 4);
        output.push('"');

        for c in s.chars() {
            match c {
                // Escape double-quotes and backslashes per Lua string literal rules.
                '"' | '\\' => output += &format!("{}", c.escape_default()),
                _ => output.push(c),
            }
        }

        output.push('"');
        Ok(output)
    }
}

/// Askama template for generating an Adobe Lightroom metadata preset file.
///
/// The rendered output is a `.lrtemplate` file in Lua table syntax that carries
/// IPTC/XMP location metadata for a dive site.
#[derive(Template)]
#[template(path = "metadata_preset.lrtemplate", escape = "none")]
pub struct MetadataPreset {
    /// The unique identifier for this preset (from the MacDive dive site UUID).
    pub id: Uuid,
    /// GPS coordinates in degrees-minutes-seconds notation (e.g. `37°46'10"N 122°28'36"W`).
    pub gps: String,
    /// The display title shown in Lightroom's preset list.
    pub title: String,
    /// IPTC sublocation — the name of the specific dive site.
    pub location: String,
    /// IPTC city — the nearest locality/city.
    pub city: String,
    /// IPTC sub-location region name.
    pub region: String,
    /// IPTC state/province.
    pub state: String,
    /// IPTC country name.
    pub country: String,
    /// ISO 3166-1 alpha-2 country code.
    pub iso_country_code: String,
    /// IPTC scene code (numeric identifier for the type of scene).
    pub scene: u64,
    /// Template format version (always 0 for MacDive exports).
    #[allow(dead_code)]
    pub version: u64,
}

impl TryFrom<DiveSite> for MetadataPreset {
    type Error = Error;

    /// Convert a [`DiveSite`] domain object into a [`MetadataPreset`] ready for rendering.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidLatitude`] or [`Error::InvalidLongitude`] if the site's
    /// coordinates cannot be converted to a [`LatLng`], or [`Error::InvalidGps`] if the
    /// DMS conversion fails.
    fn try_from(site: DiveSite) -> Result<Self> {
        let latlng: LatLng = site.clone().try_into()?;
        let region = site.region.unwrap_or_else(|| String::from("Unknown"));

        Ok(Self {
            id: site.uuid,
            gps: latlng.to_dms()?,
            title: format!(
                "[Location] {region}: {name}",
                region = &region,
                name = &site.name
            ),
            city: site.locality.unwrap_or_default(),
            region,
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
            // Null Island as a safe zero-coordinate default.
            gps: r#"0°00'00.0"N 0°00'00.0"E"#.to_string(),
            title: String::new(),
            city: String::new(),
            region: String::new(),
            country: String::new(),
            iso_country_code: String::new(),
            location: String::new(),
            state: String::new(),
            scene: 20000917,
            version: 0,
        }
    }
}
