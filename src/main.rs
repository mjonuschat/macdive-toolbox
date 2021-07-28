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
            if changeset.has_changes() {
                crate::macdive::update_critter(&changeset, &connection).await?;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::parse();
    let connection = macdive::establish_connection(&options.macdive_database()?).await?;

    // export(&options).await?;
    // critters(&options).await?;
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

    let config = Overrides {
        locations: HashMap::new(),
        critter_categories: CritterCategoryOverride {
            group_names: maplit::hashmap! {
                TaxonGroupName::Order("Side-gilled sea slugs".to_string()) => TaxonGroupName::Custom("Sea Slugs".to_string()),
                TaxonGroupName::Infraorder("Caridean Shrimps".to_string()) => TaxonGroupName::Custom("Shrimps".to_string()),
                TaxonGroupName::Infraorder("Coral and Glass Sponge Shrimps".to_string()) => TaxonGroupName::Custom("Shrimps".to_string()),
                TaxonGroupName::Infraorder("Spiny and Slipper Lobsters".to_string()) => TaxonGroupName::Custom("Lobsters".to_string()),
                TaxonGroupName::Class("Brittle Stars".to_string()) => TaxonGroupName::Custom("Sea Stars".to_string()),
                TaxonGroupName::Class("Sea Urchins, Sand Dollars, and Heart Urchins".to_string()) => TaxonGroupName::Custom("Sea Urchins".to_string()),
                TaxonGroupName::Subclass("Caenogastropods".to_string()) => TaxonGroupName::Custom("Sea Snails".to_string()),
                TaxonGroupName::Subclass("Sedentary and Tube-dwelling Bristleworms".to_string()) => TaxonGroupName::Custom("Tube Worms".to_string()),
                TaxonGroupName::Family("Bristle Worms".to_string()) => TaxonGroupName::Custom("Worms".to_string()),
                TaxonGroupName::Infraclass("Euthyneuran Gastropods".to_string()) => TaxonGroupName::Custom("Sea Slugs".to_string()),
            },
            ignored_common_names: maplit::hashmap! {
                "class".to_string() => vec!["Demosponges".to_string()]
            },
            preferred_higher_ranks: maplit::hashmap! {
                "subclass".to_string() => vec![
                    TaxonGroupName::Class("Sea Anemones and Corals".to_string()),
                ],
                "infraclass".to_string() => vec![
                    TaxonGroupName::Class("Bivalves".to_string()),
                    TaxonGroupName::Subclass("sea urchins".to_string()),
                ],
                "superorder".to_string() => vec![
                    TaxonGroupName::Class("Sea Stars".to_string()),
                    TaxonGroupName::Infraclass("Sharks".to_string()),
                    TaxonGroupName::Infraclass("Euthyneuran Gastropods".to_string()),
                ],
                "order".to_string() => vec![
                    TaxonGroupName::Class("Sea Anemones and Corals".to_string()),
                    TaxonGroupName::Class("Sea Urchins, Sand Dollars, and Heart Urchins".to_string()),
                    TaxonGroupName::Class("Sea Stars".to_string()),
                    TaxonGroupName::Class("Brittle Stars".to_string()),
                    TaxonGroupName::Class("Bivalves".to_string()),
                    TaxonGroupName::Infraclass("Sharks".to_string()),
                    TaxonGroupName::Infraclass("Rays".to_string()),
                    TaxonGroupName::Infraclass("Euthyneuran Gastropods".to_string()),
                    TaxonGroupName::Superorder("Squids and Cuttlefishes".to_string()),
                ],
                "suborder".to_string() => vec![
                    TaxonGroupName::Order("Octopuses".to_string()),
                ],
                "superfamily".to_string() => vec![
                    TaxonGroupName::Subclass("Caenogastropods".to_string()),
                    TaxonGroupName::Infraclass("Euthyneuran Gastropods".to_string()),
                    TaxonGroupName::Order("Nudibranchs".to_string()),
                    TaxonGroupName::Order("Octopuses".to_string()),
                    TaxonGroupName::Order("Scallops and Allies".to_string()),
                    TaxonGroupName::Suborder("Prawns".to_string()),
                    TaxonGroupName::Infraorder("True Crabs".to_string()),
                ],
                "family".to_string() => vec![
                    TaxonGroupName::Superorder("Squids and Cuttlefishes".to_string()),
                    TaxonGroupName::Order("Flatfishes".to_string()),
                    TaxonGroupName::Order("True Eels".to_string()),
                    TaxonGroupName::Order("Nudibranchs".to_string()),
                    TaxonGroupName::Suborder("Prawns".to_string()),
                    TaxonGroupName::Class("Sea Stars".to_string()),
                    TaxonGroupName::Subclass("Mantis Shrimps".to_string()),
                    TaxonGroupName::Subclass("Sedentary and Tube-dwelling Bristleworms".to_string()),
                    TaxonGroupName::Subclass("Caenogastropods".to_string()),
                    TaxonGroupName::Superfamily("Hermit Crabs".to_string()),
                ],
                "subfamily".to_string() => vec![
                    TaxonGroupName::Family("Cardinalfishes".to_string()),
                    TaxonGroupName::Family("Pufferfishes".to_string()),
                    TaxonGroupName::Family("Morays".to_string()),
                    TaxonGroupName::Family("Scorpionfishes".to_string()),
                    TaxonGroupName::Family("Jacks".to_string()),
                    TaxonGroupName::Family("Wrasses".to_string()),
                ],
                "genus".to_string() => vec![
                    TaxonGroupName::Subfamily("Damselfishes".to_string()),
                    TaxonGroupName::Subfamily("Groupers".to_string()),
                ]
            },
        },
    };

    println!("{}", serde_yaml::to_string(&config)?);
    let overrides = std::fs::read_to_string("examples/config.yaml")?;
    let overrides: HashMap<String, Vec<TaxonGroupName>> = serde_yaml::from_str(&overrides)?;

    for critter in critters {
        if let Some(scientific_name) = critter.species.as_deref() {
            if let Ok(taxon) = crate::inaturalist::get_taxon_by_name(scientific_name).await {
                if let Ok(group_name) = taxon.group_name(&config.critter_categories).await {
                    println!(
                        "{} ({}): {}",
                        taxon.preferred_common_name.as_deref().unwrap_or(""),
                        taxon.name.as_deref().unwrap_or(""),
                        group_name
                    )
                }
            }
        }
    }
    Ok(())
}
