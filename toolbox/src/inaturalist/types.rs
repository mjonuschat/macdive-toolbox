use crate::inaturalist::get_taxon_by_id;
use crate::types::CritterCategoryConfig;
pub use macdive_toolbox_core::domain::TaxonGroupName;
pub use macdive_toolbox_core::services::inaturalist::types::Taxon;
use sea_orm::DbConn;

/// Trait for resolving a taxon's category group name from its ancestry.
#[async_trait::async_trait]
pub trait TaxonCategoryName {
    /// Walk the taxon's ancestor chain and determine its category group name.
    ///
    /// # Arguments
    ///
    /// * `db` - Cache database connection for ancestor lookups
    /// * `overrides` - User-supplied category configuration overrides
    /// * `offline` - When true, only use cached ancestor data
    async fn group_name(
        &self,
        db: &DbConn,
        overrides: &CritterCategoryConfig,
        offline: bool,
    ) -> anyhow::Result<TaxonGroupName>;
}

// TODO: Consider moving TaxonCategoryName to core once anyhow is removed from its signature
#[async_trait::async_trait]
impl TaxonCategoryName for Taxon {
    async fn group_name(
        &self,
        db: &DbConn,
        overrides: &CritterCategoryConfig,
        offline: bool,
    ) -> anyhow::Result<TaxonGroupName> {
        let mut group = TaxonGroupName::Unspecified;
        if let Some(ancestor_ids) = &self.ancestor_ids {
            for ancestor_id in ancestor_ids.iter() {
                let ancestor = get_taxon_by_id(db, *ancestor_id, offline).await?;
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
