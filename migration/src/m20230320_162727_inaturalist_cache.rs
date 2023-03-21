use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TaxonCache::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaxonCache::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TaxonCache::TaxonId)
                            .integer()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(TaxonCache::MatchedName)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(TaxonCache::Taxon).json().not_null())
                    .col(
                        ColumnDef::new(TaxonCache::DownloadedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TaxonCache::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum TaxonCache {
    Table,
    Id,
    TaxonId,
    MatchedName,
    Taxon,
    DownloadedAt,
}
