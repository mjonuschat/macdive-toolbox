use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

mod api;
mod models;

use crate::inaturalist::get_taxon_by_id;
use crate::types::CritterCategoryConfig;
pub(in crate::inaturalist) use api::*;
pub use models::*;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/// SQLx: Return type used for cache_taxa table
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedTaxon {
    pub id: i32,
    pub uuid: Option<Uuid>,
    pub name: String,
    pub taxon: Json<Taxon>,
}

#[derive(Clone, Debug, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

impl PartialEq for TaxonGroupName {
    fn eq(&self, other: &Self) -> bool {
        use TaxonGroupName::*;

        match (self, other) {
            (Unspecified, Unspecified) => true,
            (Custom(lhs), Custom(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Phylum(lhs), Phylum(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Subphylum(lhs), Subphylum(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Class(lhs), Class(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Subclass(lhs), Subclass(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Infraclass(lhs), Infraclass(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Superorder(lhs), Superorder(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Order(lhs), Order(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Suborder(lhs), Suborder(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Infraorder(lhs), Infraorder(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Parvorder(lhs), Parvorder(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Superfamily(lhs), Superfamily(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Family(lhs), Family(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Subfamily(lhs), Subfamily(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Genus(lhs), Genus(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (_, _) => false,
        }
    }
}

impl Hash for TaxonGroupName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use TaxonGroupName::*;

        match self {
            Unspecified => "".hash(state),
            Custom(v) => v.hash(state),
            Phylum(v) => v.hash(state),
            Subphylum(v) => v.hash(state),
            Class(v) => v.hash(state),
            Subclass(v) => v.hash(state),
            Infraclass(v) => v.hash(state),
            Superorder(v) => v.hash(state),
            Order(v) => v.hash(state),
            Suborder(v) => v.hash(state),
            Infraorder(v) => v.hash(state),
            Parvorder(v) => v.hash(state),
            Superfamily(v) => v.hash(state),
            Family(v) => v.hash(state),
            Subfamily(v) => v.hash(state),
            Genus(v) => v.hash(state),
        }
    }
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

    pub fn prefer_higher_common_name(
        &self,
        class: &str,
        overrides: &CritterCategoryConfig,
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
        overrides: &CritterCategoryConfig,
        offline: bool,
    ) -> anyhow::Result<TaxonGroupName>;
}

#[async_trait::async_trait]
impl TaxonCategoryName for Taxon {
    async fn group_name(
        &self,
        overrides: &CritterCategoryConfig,
        offline: bool,
    ) -> anyhow::Result<TaxonGroupName> {
        let mut group = TaxonGroupName::Unspecified;
        if let Some(ancestor_ids) = &self.ancestor_ids {
            for ancestor_id in ancestor_ids.iter() {
                let ancestor = get_taxon_by_id(*ancestor_id, offline).await?;
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
                            group = TaxonGroupName::Class(name);
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
                        if let Some(v) = overrides.group_names.get(&group.to_string()) {
                            group = TaxonGroupName::Custom(v.to_owned())
                        }
                    }
                    Some(_) | None => {}
                }
            }
        }

        Ok(group)
    }
}
