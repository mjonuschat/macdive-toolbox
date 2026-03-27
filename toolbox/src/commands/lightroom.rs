use crate::cli::LightroomOptions;
use crate::errors::ConversionError;
use crate::types::dive_site_from_entity;
use comfy_table::*;
use console::{Emoji, style};
use futures::StreamExt;
use indicatif::ProgressBar;
use macdive_toolbox_core::db::DatabaseManager;
use macdive_toolbox_core::domain::{DiveSite, LocationOverride};
use macdive_toolbox_core::macdive::queries;
use macdive_toolbox_core::services::geocoding;
use macdive_toolbox_core::services::lightroom::{
    MetadataPreset, read_existing_presets, write_presets,
};

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
    let existing = read_existing_presets(&options.lightroom_metadata()?)?;

    println!(
        "{} {}Fetching dive sites from MacDive...",
        style("[2/4]").bold().dim(),
        DIVING_MASK
    );
    let sites = queries::sites(db.macdive())
        .await?
        .into_iter()
        .map(dive_site_from_entity)
        .collect::<Result<Vec<DiveSite>, ConversionError>>()?;

    println!(
        "{} {}Looking up addresses for dive sites...",
        style("[3/4]").bold().dim(),
        SATELLITE
    );
    let mut sites: Vec<DiveSite> = sites
        .into_iter()
        .filter(|site| force || !existing.contains_key(&site.uuid))
        .collect();
    let pb = ProgressBar::new(sites.len() as u64);

    if let Some(key) = &options.api_key {
        sites = futures::stream::iter(sites)
            .map(|site| {
                pb.inc(1);
                geocoding::geocode_site(site, key)
            })
            .buffer_unordered(10usize)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|item| {
                item.map_err(ConversionError::from).and_then(|site| {
                    geocoding::apply_overrides(site, overrides).map_err(ConversionError::from)
                })
            })
            .collect::<anyhow::Result<Vec<_>, ConversionError>>()?;
    }
    let presets = sites
        .into_iter()
        .map(|site| MetadataPreset::try_from(site).map_err(ConversionError::from))
        .collect::<Result<Vec<MetadataPreset>, ConversionError>>()?;
    pb.finish_and_clear();

    println!(
        "{} {}Writing Lightroom Metadata Presets...",
        style("[4/4]").bold().dim(),
        FILE_FOLDER
    );
    write_presets(&options.lightroom_metadata()?, &presets, &existing)?;

    if !presets.is_empty() {
        print_summary(&presets);
    }

    Ok(())
}
