use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

mod api;
mod models;

use crate::inaturalist::get_taxon_by_id;
use crate::types::CritterCategoryOverride;
pub(in crate::inaturalist) use api::*;
pub use models::*;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

/// SQLx: Return type used for cache_taxa table
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedTaxon {
    pub id: i32,
    pub uuid: Option<Uuid>,
    pub name: String,
    pub taxon: Json<Taxon>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaxonGroupName {
    Unspecified,
    Custom(String),
    Phylum(String),
    Subphylum(String),
    Class(String),
    Subclass(String),
    Infraclass(String),
    Superorder(String),
    Order(String),
    Suborder(String),
    Infraorder(String),
    Parvorder(String),
    Superfamily(String),
    Family(String),
    Subfamily(String),
    Genus(String),
}

impl TaxonGroupName {
    fn normalize(name: &str) -> String {
        change_case::title_case(
            name.to_lowercase()
                .trim_start_matches("true")
                .trim_start_matches("false")
                .trim_start_matches("typical")
                .trim_end_matches("and allies")
                .trim()
                .trim_end_matches(','),
        )
    }

    pub(crate) fn ignore_common_name(
        &self,
        class: &str,
        name: &str,
        overrides: &CritterCategoryOverride,
    ) -> bool {
        overrides
            .ignored_common_names
            .get(class)
            .map(|list| list.contains(&name.to_string()))
            .unwrap_or(false)
    }

    pub fn prefer_higher_common_name(
        &self,
        class: &str,
        overrides: &CritterCategoryOverride,
    ) -> bool {
        overrides
            .preferred_higher_ranks
            .get(class)
            .map(|list| list.contains(self))
            .unwrap_or(false)
    }
}

impl Display for TaxonGroupName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TaxonGroupName::Unspecified => write!(f, "Unknown"),
            TaxonGroupName::Custom(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Phylum(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Subphylum(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Class(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Subclass(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Infraclass(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Superorder(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Order(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Suborder(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Infraorder(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Parvorder(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Superfamily(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Family(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Subfamily(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Genus(name) => write!(f, "{}", Self::normalize(name)),
        }
    }
}

#[async_trait::async_trait]
pub trait TaxonCategoryName {
    async fn group_name(
        &self,
        overrides: &CritterCategoryOverride,
    ) -> anyhow::Result<TaxonGroupName>;
}

#[async_trait::async_trait]
impl TaxonCategoryName for Taxon {
    async fn group_name(
        &self,
        overrides: &CritterCategoryOverride,
    ) -> anyhow::Result<TaxonGroupName> {
        let mut group = TaxonGroupName::Unspecified;
        if let Some(ancestor_ids) = &self.ancestor_ids {
            for ancestor_id in ancestor_ids.iter() {
                let ancestor = get_taxon_by_id(*ancestor_id).await?;
                match ancestor.rank.as_deref() {
                    Some("phylum") => {
                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Phylum(name);
                        }
                    }
                    Some("subphylum") => {
                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Subphylum(name);
                        }
                    }
                    Some("class") => {
                        if group.prefer_higher_common_name("class", overrides) {
                            continue;
                        }
                        if let Some(name) = ancestor.preferred_common_name {
                            if !group.ignore_common_name("class", &name, overrides) {
                                group = TaxonGroupName::Class(name);
                            }
                        }
                    }
                    Some("subclass")
                        if matches!(
                            group,
                            TaxonGroupName::Phylum(_) | TaxonGroupName::Class(_)
                        ) =>
                    {
                        if group.prefer_higher_common_name("subclass", overrides) {
                            continue;
                        }

                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Subclass(name)
                        }
                    }
                    Some("infraclass")
                        if matches!(
                            group,
                            TaxonGroupName::Phylum(_)
                                | TaxonGroupName::Class(_)
                                | TaxonGroupName::Subclass(_)
                        ) =>
                    {
                        if group.prefer_higher_common_name("infraclass", overrides) {
                            continue;
                        }

                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Infraclass(name)
                        }
                    }
                    Some("superorder")
                        if matches!(
                            group,
                            TaxonGroupName::Phylum(_)
                                | TaxonGroupName::Class(_)
                                | TaxonGroupName::Subclass(_)
                                | TaxonGroupName::Infraclass(_)
                        ) =>
                    {
                        if group.prefer_higher_common_name("superorder", overrides) {
                            continue;
                        }

                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Superorder(name)
                        }
                    }
                    Some("order")
                        if matches!(
                            group,
                            TaxonGroupName::Phylum(_)
                                | TaxonGroupName::Class(_)
                                | TaxonGroupName::Subclass(_)
                                | TaxonGroupName::Infraclass(_)
                                | TaxonGroupName::Superorder(_)
                        ) =>
                    {
                        if group.prefer_higher_common_name("order", overrides) {
                            continue;
                        }

                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Order(name)
                        }
                    }
                    Some("suborder") if matches!(group, TaxonGroupName::Order(_)) => {
                        if group.prefer_higher_common_name("suborder", overrides) {
                            continue;
                        }

                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Suborder(name)
                        }
                    }
                    Some("infraorder")
                        if matches!(
                            group,
                            TaxonGroupName::Phylum(_)
                                | TaxonGroupName::Class(_)
                                | TaxonGroupName::Subclass(_)
                                | TaxonGroupName::Superorder(_)
                                | TaxonGroupName::Suborder(_)
                        ) =>
                    {
                        if group.prefer_higher_common_name("infraorder", overrides) {
                            continue;
                        }

                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Infraorder(name)
                        }
                    }
                    Some("superfamily")
                        if matches!(
                            group,
                            TaxonGroupName::Order(_)
                                | TaxonGroupName::Infraclass(_)
                                | TaxonGroupName::Subclass(_)
                                | TaxonGroupName::Infraorder(_)
                        ) =>
                    {
                        if group.prefer_higher_common_name("superfamily", overrides) {
                            continue;
                        }

                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Superfamily(name)
                        }
                    }
                    // OVERRIDE: Superfamily("Box and Moon Crabs")
                    // OVERRIDE: Superfamily("Cowries, Trivia, and Allies")
                    // OVERRIDE: Infraorder("Spiny and Slipper Lobsters")
                    // OVERRIDE: Infraorder("Coral and Glass Sponge Shrimps")
                    Some("family")
                        if matches!(
                            group,
                            TaxonGroupName::Phylum(_)
                                | TaxonGroupName::Order(_)
                                | TaxonGroupName::Infraorder(_)
                                | TaxonGroupName::Subclass(_)
                                | TaxonGroupName::Superfamily(_)
                        ) =>
                    {
                        if group.prefer_higher_common_name("family", overrides) {
                            continue;
                        }

                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Family(name)
                        }
                    }
                    Some("subfamily") if matches!(group, TaxonGroupName::Family(_)) => {
                        if group.prefer_higher_common_name("subfamily", overrides) {
                            continue;
                        }

                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Subfamily(name);
                        }
                    }
                    Some("genus") if matches!(group, TaxonGroupName::Subfamily(_)) => {
                        if group.prefer_higher_common_name("genus", overrides) {
                            continue;
                        }
                        if let Some(name) = ancestor.preferred_common_name {
                            group = TaxonGroupName::Genus(name);
                        }
                    }
                    Some("species") => {
                        if let Some(v) = overrides.group_names.get(&group) {
                            group = v.clone();
                        }
                    }
                    Some(_) | None => {}
                }
            }
        }

        Ok(group)
    }
}
