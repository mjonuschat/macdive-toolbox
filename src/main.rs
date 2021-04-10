#[macro_use]
extern crate diesel;

use anyhow::Result;
use clap::Clap;
use indicatif::ProgressBar;
use prettytable::{Cell, Row, Table};

use std::convert::TryInto;

mod arguments;
mod errors;
mod geocode;
mod lightroom;
mod macdive;
mod types;

use arguments::Options;
use console::{style, Emoji};
use errors::ConversionError;
use lightroom::MetadataPreset;

fn print_summary(presets: &[MetadataPreset]) {
    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(Row::new(vec![
        Cell::new("Site").style_spec("b"),
        Cell::new("City").style_spec("b"),
        Cell::new("Region").style_spec("b"),
        Cell::new("State").style_spec("b"),
        Cell::new("Country").style_spec("b"),
        Cell::new("GPS").style_spec("b"),
    ]));

    for site in presets {
        table.add_row(Row::new(vec![
            Cell::new(&site.location),
            Cell::new(&site.city),
            Cell::new(&site.region),
            Cell::new(&site.state),
            Cell::new(&site.country),
            Cell::new(&site.gps),
        ]));
    }

    table.printstd();
}

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static DIVING_MASK: Emoji<'_, '_> = Emoji("🤿️  ", "");
static SATELLITE: Emoji<'_, '_> = Emoji("🛰️   ", "");
static FILE_FOLDER: Emoji<'_, '_> = Emoji("📂  ", "");

// TODO: Exit code handling
fn main() -> Result<()> {
    let options = Options::parse();

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
    let connection = macdive::establish_connection(&options.macdive_database()?)?;
    let sites = macdive::sites(&connection)?
        .into_iter()
        .map(|site| site.try_into())
        .collect::<Result<Vec<types::DiveSite>, ConversionError>>()?;

    println!(
        "{} {}Looking up addresses for dive sites...",
        style("[3/4]").bold().dim(),
        SATELLITE
    );
    let sites: Vec<types::DiveSite> = sites
        .into_iter()
        .filter(|site| options.force || !existing.contains_key(&site.uuid))
        .collect();
    let pb = ProgressBar::new(sites.len() as u64);
    let presets = sites
        .into_iter()
        .map(|site| {
            let s = if let Some(key) = &options.api_key {
                geocode::geocode_site(site, key)
            } else {
                Ok(site)
            };
            pb.inc(1);
            s.map_err(ConversionError::GeocodingError)
                .and_then(|site| {
                    geocode::apply_overrides(site, &options.location_overrides())
                        .map_err(ConversionError::GeocodingError)
                })
                .and_then(|site| site.try_into())
        })
        .collect::<Result<Vec<MetadataPreset>, errors::ConversionError>>()?;
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
