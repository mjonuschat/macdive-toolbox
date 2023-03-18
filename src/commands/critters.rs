use crate::arguments::{MacdiveImportFormat, PrepareImportOptions};
use crate::inaturalist::{Taxon, TaxonCategoryName, TaxonGroupName};
use crate::macdive;
use crate::macdive::models::CritterUpdate;
use crate::parsers::species::sanitize_species_name;
use crate::types::{CritterCategoryConfig, CritterConfig};
use futures::StreamExt;
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
struct CritterItem {
    name: Option<String>,
    species: Option<String>,
    size: f32,
    category: Option<String>,
    #[serde(skip_serializing)]
    original_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Critters {
    schema: String,
    critter: Vec<CritterItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CritterDoc {
    critters: Critters,
}

impl Default for Critters {
    fn default() -> Self {
        Self {
            schema: "1.0.0".to_string(),
            critter: vec![],
        }
    }
}

pub async fn diff_critters(database: &Path, offline: bool) -> anyhow::Result<()> {
    let connection = macdive::establish_connection(database).await?;
    let critters = crate::macdive::critters(&connection).await?;

    let species = critters
        .iter()
        .filter_map(|c| c.species.as_deref())
        .collect::<Vec<_>>();

    crate::inaturalist::cache_species(&species, offline).await?;

    for critter in critters {
        if let Some(scientific_name) = critter.species.as_deref() {
            tracing::trace!("Looking up {scientific_name} on iNaturalist");
            let taxon = match crate::inaturalist::get_taxon_by_name(scientific_name, offline).await
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        scientific_name = scientific_name,
                        "Failed to retrieve taxon: {e}"
                    );
                    continue;
                }
            };

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

pub async fn diff_critter_categories(
    database: &Path,
    overrides: &CritterCategoryConfig,
    offline: bool,
) -> anyhow::Result<()> {
    let connection = macdive::establish_connection(database).await?;

    let critters = crate::macdive::critters(&connection).await?;

    // Categories that currently are in MacDive
    let mut current_categories = crate::macdive::critter_categories(&connection)
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

    let critter_groups: HashMap<String, TaxonGroupName> =
        futures::stream::iter(critters.iter().filter_map(|c| c.species.clone()))
            .filter_map(|scientific_name| async move {
                if let Ok(taxon) =
                    crate::inaturalist::get_taxon_by_name(&scientific_name, offline).await
                {
                    if let Ok(group_name) = taxon.group_name(overrides, offline).await {
                        return Some((scientific_name, group_name));
                    }
                } else {
                    tracing::error!(
                        scientific_name = scientific_name.as_str(),
                        "Taxon lookup failed"
                    )
                }

                None
            })
            .collect()
            .await;

    let current_names: HashSet<String> = current_categories
        .keys()
        .map(|v| change_case::lower_case(v))
        .collect();

    let desired_names: HashSet<String> = critter_groups
        .values()
        .map(|v| change_case::lower_case(&v.to_string()))
        .collect();

    let mut extraneous_categories: Vec<String> = current_names
        .difference(&desired_names)
        .map(|v| v.to_owned())
        .collect();

    let mut category_index: HashMap<_, _> = current_categories
        .iter()
        .map(|(k, v)| (v.id, k.to_owned()))
        .collect();

    for critter in critters {
        if let Some(scientific_name) = &critter.species {
            let current_category = &critter.category.and_then(|id| {
                category_index
                    .get(&id)
                    .and_then(|key| current_categories.get(key))
            });
            let desired_category = &critter_groups
                .get(scientific_name)
                .and_then(|v| current_categories.get(&change_case::lower_case(&v.to_string())));

            match (current_category, desired_category) {
                (Some(cc), Some(dc)) if cc.id != dc.id => {
                    // TODO: Delta
                    eprintln!(
                        "Re-Assigning: {:?} ({:?}): {:?} => {:?}",
                        &critter.name, &critter.species, &cc.name, &dc.name
                    );
                    // crate::macdive::update_critter(
                    //     &CritterUpdate {
                    //         id: critter.id,
                    //         category: Some(dc.id),
                    //         common_name: critter.name,
                    //         ..Default::default()
                    //     },
                    //     &connection,
                    // )
                    // .await?;
                }
                (Some(_), Some(_)) => {
                    // Old and new category are identical
                }
                (None, Some(dc)) => {
                    eprintln!(
                        "Assigning: {:?} ({:?}): --- => {:?}",
                        &critter.name, &critter.species, &dc.name
                    );
                    // crate::macdive::update_critter(
                    //     &CritterUpdate {
                    //         id: critter.id,
                    //         category: Some(dc.id),
                    //         common_name: critter.name,
                    //         ..Default::default()
                    //     },
                    //     &connection,
                    // )
                    // .await?;
                }
                (Some(_cc), None) => match &critter_groups.get(scientific_name) {
                    Some(new_category) => {
                        let category = extraneous_categories
                            .pop()
                            .and_then(|key| current_categories.remove(&key));

                        match category {
                            Some(mut c) => {
                                let old_name = c.name.clone();
                                let new_name = new_category.to_string();
                                c.name = Some(new_name.clone());
                                let key = change_case::lower_case(&new_category.to_string());
                                let id = c.id;

                                current_categories.insert(key.clone(), c);
                                category_index.insert(id, key);

                                eprintln!(
                                    "Renaming category {:?} => {:?}",
                                    old_name,
                                    new_category.to_string()
                                );
                                eprintln!(
                                    "Re-Assigning: {:?} ({:?}): {:?} => {:?}",
                                    &critter.name, &critter.species, &old_name, &new_name
                                );

                                // crate::macdive::update_critter_category(
                                //     id,
                                //     &change_case::title_case(&new_category.to_string()),
                                //     &connection,
                                // )
                                // .await?;
                                //
                                // crate::macdive::update_critter(
                                //     &CritterUpdate {
                                //         id: critter.id,
                                //         category: Some(id),
                                //         common_name: critter.name,
                                //         ..Default::default()
                                //     },
                                //     &connection,
                                // )
                                // .await?;
                            }
                            None => {
                                eprintln!("Brand spanking new category needed: {}", new_category)
                            }
                        }
                    }
                    None => eprintln!(
                        "This should not happen - no new category: {}",
                        scientific_name
                    ),
                },
                (None, None) => {
                    let new_category = &critter_groups.get(scientific_name).unwrap();
                    eprintln!("New category required [2]: {}", new_category);
                }
            }
        }
    }
    // println!("Missing categories: {:#?}", &missing);
    println!("Extraneous categories: {:#?}", &extraneous_categories);
    // println!("Existing categories: {:#?}", &existing);

    Ok(())
}

pub(crate) async fn critter_import(
    options: &PrepareImportOptions,
    config: &CritterConfig,
    offline: bool,
) -> anyhow::Result<()> {
    let file = File::open(&options.source)?;
    let reader = BufReader::new(file).lines();
    let names: Vec<String> = reader
        .filter_map(Result::ok)
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .filter_map(|name| sanitize_species_name(&name).ok())
        .collect();

    let pb = ProgressBar::new(names.len() as u64);
    let results = futures::stream::iter(names)
        .filter_map(|scientific_name| {
            let scientific_name = scientific_name;
            pb.inc(1);
            async move {
                if let Ok(taxon) =
                    crate::inaturalist::get_taxon_by_name(&scientific_name, offline).await
                {
                    if let Ok(group_name) = taxon.group_name(&config.categories, offline).await {
                        return Some((taxon, group_name, scientific_name));
                    }
                    Some((taxon, TaxonGroupName::Unspecified, scientific_name))
                } else {
                    tracing::debug!(
                        scientific_name = scientific_name.as_str(),
                        "Taxon lookup failed"
                    );
                    if options.skip_invalid {
                        return None;
                    }
                    Some((
                        Taxon {
                            name: Some(scientific_name.clone()),
                            preferred_common_name: None,
                            ..Default::default()
                        },
                        TaxonGroupName::Unspecified,
                        scientific_name,
                    ))
                }
            }
        })
        .collect::<Vec<_>>()
        .await;

    let critters = Critters {
        critter: results
            .into_iter()
            .map(|(taxon, group_name, original_name)| CritterItem {
                name: taxon
                    .preferred_common_name
                    .as_deref()
                    .map(|v| change_case::title_case(v.trim())),
                species: taxon
                    .name
                    .as_deref()
                    .map(|v| change_case::title_case(v.trim())),
                original_name: Some(original_name),
                category: Some(group_name.to_string()),
                ..Default::default()
            })
            .filter(|critter| critter.name.is_some())
            .collect(),
        ..Default::default()
    };

    match options.format {
        MacdiveImportFormat::Xml => {
            let result = xml_serde::to_string(&critters)?.replace(
                "<critters xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">",
                "<!DOCTYPE critters SYSTEM \"http://www.mac-dive.com/macdive_critters.dtd\">\n<critters>",
            );
            std::fs::write(&options.dest, result)?;
        }
        MacdiveImportFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(File::create(&options.dest)?);
            critters
                .critter
                .iter()
                .map(|t| {
                    wtr.write_record([
                        &t.name.as_deref().unwrap_or_default(),
                        &t.species.as_deref().unwrap_or_default(),
                        &t.category.as_deref().unwrap_or_default(),
                        &t.original_name.as_deref().unwrap_or_default(),
                    ])
                })
                .collect::<Result<Vec<_>, _>>()?;
            wtr.flush()?;
        }
    };
    Ok(())
}
