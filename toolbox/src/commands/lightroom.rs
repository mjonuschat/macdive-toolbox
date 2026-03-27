use comfy_table::*;
use console::{Emoji, style};
use futures::StreamExt;
use indicatif::ProgressBar;
use macdive_toolbox_core::db::DatabaseManager;
use std::convert::TryInto;

use crate::arguments::LightroomOptions;
use crate::errors::ConversionError;
use crate::helpers::lightroom::MetadataPreset;
use crate::helpers::{geocode, lightroom};
use crate::types::{self, LocationOverride, dive_site_from_entity};

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static DIVING_MASK: Emoji<'_, '_> = Emoji("🤿️  ", "");
static SATELLITE: Emoji<'_, '_> = Emoji("🛰️   ", "");
static FILE_FOLDER: Emoji<'_, '_> = Emoji("📂  ", "");

fn print_summary(presets: &[MetadataPreset]) {
    let mut table = Table::new();
    table
        .load_preset("││──╞═╪╡┆    ┬┴┌┐└┘")
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Site").add_attribute(Attribute::Bold),
            Cell::new("City").add_attribute(Attribute::Bold),
            Cell::new("Region").add_attribute(Attribute::Bold),
            Cell::new("State").add_attribute(Attribute::Bold),
            Cell::new("Country").add_attribute(Attribute::Bold),
            Cell::new("GPS").add_attribute(Attribute::Bold),
        ]);

    for site in presets {
        table.add_row(vec![
            Cell::new(&site.location),
            Cell::new(&site.city),
            Cell::new(&site.region),
            Cell::new(&site.state),
            Cell::new(&site.country),
            Cell::new(&site.gps),
        ]);
    }

    println!("{table}");
}

pub(crate) async fn export_lightroom_metadata_presets(
    db: &DatabaseManager,
    options: &LightroomOptions,
    overrides: &[LocationOverride],
    force: bool,
) -> anyhow::Result<()> {
    println!(
        "{} {}Locating existing metadata presets...",
        style("[1/4]").bold().dim(),
        LOOKING_GLASS
    );
    let existing = lightroom::read_existing_presets(&options.lightroom_metadata()?)?;

    println!(
        "{} {}Fetching dive sites from MacDive...",
        style("[2/4]").bold().dim(),
        DIVING_MASK
    );
    let sites = macdive_toolbox_core::macdive::queries::sites(db.macdive())
        .await?
        .into_iter()
        .map(dive_site_from_entity)
        .collect::<Result<Vec<types::DiveSite>, ConversionError>>()?;

    println!(
        "{} {}Looking up addresses for dive sites...",
        style("[3/4]").bold().dim(),
        SATELLITE
    );
    let mut sites: Vec<types::DiveSite> = sites
        .into_iter()
        .filter(|site| force || !existing.contains_key(&site.uuid))
        .collect();
    let pb = ProgressBar::new(sites.len() as u64);

    if let Some(key) = &options.api_key {
        sites = futures::stream::iter(sites)
            .map(|site| {
                pb.inc(1);
                geocode::geocode_site(site, key)
            })
            .buffer_unordered(10usize)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|item| {
                item.map_err(ConversionError::GeocodingError)
                    .and_then(|site| {
                        geocode::apply_overrides(site, overrides)
                            .map_err(ConversionError::GeocodingError)
                    })
            })
            .collect::<anyhow::Result<Vec<_>, ConversionError>>()?;
    }
    let presets = sites
        .into_iter()
        .map(|site| site.try_into())
        .collect::<anyhow::Result<Vec<MetadataPreset>, ConversionError>>()?;
    pb.finish_and_clear();

    println!(
        "{} {}Writing Lightroom Metadata Presets...",
        style("[4/4]").bold().dim(),
        FILE_FOLDER
    );
    lightroom::write_presets(&options.lightroom_metadata()?, &presets, &existing)?;

    if !presets.is_empty() {
        print_summary(&presets);
    }

    Ok(())
}
