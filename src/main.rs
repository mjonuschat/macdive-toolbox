use anyhow::Result;
use clap::Clap;
use indicatif::ProgressBar;
use prettytable::{Cell, Row, Table};

use std::convert::TryInto;

mod arguments;
mod errors;
mod geocode;
mod inaturalist;
mod lightroom;
mod macdive;
mod types;

use crate::inaturalist::{Taxon, TaxonCategoryName, TaxonGroupName};
use crate::macdive::models::{Critter, CritterUpdate};
use crate::types::{CritterCategoryOverride, Overrides};
use arguments::Options;
use console::{style, Emoji};
use errors::ConversionError;
use futures::StreamExt;
use itertools::Itertools;
use lightroom::MetadataPreset;
use std::collections::HashMap;
use surf::connect;
use uuid::Uuid;

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

async fn export(options: &Options) -> Result<()> {
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
    let connection = macdive::establish_connection(&options.macdive_database()?).await?;
    let sites = macdive::sites(&connection)
        .await?
        .into_iter()
        .map(|site| site.try_into())
        .collect::<Result<Vec<types::DiveSite>, ConversionError>>()?;

    println!(
        "{} {}Looking up addresses for dive sites...",
        style("[3/4]").bold().dim(),
        SATELLITE
    );
    let mut sites: Vec<types::DiveSite> = sites
        .into_iter()
        .filter(|site| options.force || !existing.contains_key(&site.uuid))
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
                        geocode::apply_overrides(site, &options.location_overrides())
                            .map_err(ConversionError::GeocodingError)
                    })
            })
            .collect::<Result<Vec<_>, ConversionError>>()?;
    }
    let presets = sites
        .into_iter()
        .map(|site| site.try_into())
        .collect::<Result<Vec<MetadataPreset>, ConversionError>>()?;
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

async fn critters(options: &Options) -> Result<()> {
    let connection = macdive::establish_connection(&options.macdive_database()?).await?;
    let critters = crate::macdive::critters(&connection).await?;

    let species = critters
        .iter()
        .filter_map(|c| c.species.as_deref())
        .collect::<Vec<_>>();

    crate::inaturalist::cache_species(&species).await?;

    for critter in critters {
        if let Some(scientific_name) = critter.species.as_deref() {
            let taxon = crate::inaturalist::get_taxon_by_name(scientific_name).await?;

            let current_name = critter
                .name
                .as_deref()
                .map(|v| change_case::title_case(v.trim()));
            let preferred_name = taxon
                .preferred_common_name
                .as_deref()
                .map(|v| change_case::title_case(v.trim()));

            let scientific_name = change_case::title_case(scientific_name);

            let mut changeset: CritterUpdate = CritterUpdate {
                id: critter.id,
                ..Default::default()
            };

            if let Some(preferred_scientific_name) = taxon.name.as_deref() {
                let preferred_scientific_name =
                    change_case::sentence_case(preferred_scientific_name);
                let current_scientific_name = change_case::sentence_case(&scientific_name);

                if current_scientific_name != preferred_scientific_name {
                    println!(
                        "Mismatched scientific name: MacDive: {} => iNat: {}",
                        current_scientific_name, preferred_scientific_name
                    );
                    changeset.scientific_name = Some(preferred_scientific_name);
                }
            }

            match (current_name, preferred_name) {
                (Some(current_name), Some(preferred_name)) if preferred_name != current_name => {
                    // TODO: Make this a nice table
                    println!(
                        "Mismatched common name: MacDive {:?} => iNat: {:?}",
                        &current_name, &preferred_name
                    );
                    changeset.common_name = Some(preferred_name);
                }
                (None, Some(preferred_name)) => {
                    println!(
                        "Found new common name for {}: {}",
                        &scientific_name, &preferred_name
                    );
                    changeset.common_name = Some(preferred_name);
                }
                (Some(_), Some(_)) => {
                    // Pass, names are identical
                }
                (Some(_), None) => {
                    // Pass, no registered common name in iNaturalist
                }
                (None, None) => {
                    println!("Woha, no common name for species: {}", &scientific_name)
                }
            }

            // TODO: Guard with command line flag!
            // if changeset.has_changes() {
            //     crate::macdive::update_critter(&changeset, &connection).await?;
            // }
        }
    }
    Ok(())
}

async fn critter_categories(options: &Options) -> Result<()> {
    let connection = macdive::establish_connection(&options.macdive_database()?).await?;

    let critters = crate::macdive::critters(&connection).await?;
    let _categories = crate::macdive::critter_categories(&connection)
        .await?
        .into_iter()
        .filter_map(|category| match category.name.as_deref() {
            Some(name) => {
                let key = change_case::lower_case(name);
                Some((key, category))
            }
            None => None,
        })
        .collect::<HashMap<_, _>>();

    let mut assignments: HashMap<String, Vec<(Option<String>, Option<String>)>> = HashMap::new();
    for critter in critters {
        if let Some(scientific_name) = critter.species.as_deref() {
            if let Ok(taxon) = crate::inaturalist::get_taxon_by_name(scientific_name).await {
                if let Ok(group_name) = taxon
                    .group_name(&options.critter_categories_overrides())
                    .await
                {
                    assignments
                        .entry(group_name.to_string())
                        .or_insert_with(Vec::new)
                        .push((taxon.name, taxon.preferred_common_name));
                }
            }
        }
    }

    for key in assignments.keys().sorted() {
        println!("## {}", key);
        if let Some(critters) = assignments.get(key) {
            for (name, common_name) in critters {
                println!(
                    "  {} ({})",
                    name.as_deref().unwrap_or(""),
                    common_name.as_deref().unwrap_or("")
                )
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::parse();

    // export(&options).await?;
    // critters(&options).await?;
    critter_categories(&options).await?;
    Ok(())
}
