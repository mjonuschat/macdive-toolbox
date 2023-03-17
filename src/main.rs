use anyhow::Result;
use clap::Parser;
use tracing::Level;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

mod arguments;
mod commands;
mod errors;
mod geocode;
mod inaturalist;
mod lightroom;
mod macdive;
mod types;

use arguments::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_filter(Targets::default().with_default(Level::DEBUG)),
        )
        .with(
            Targets::default()
                .with_target("macdive_exporter", Level::TRACE)
                .with_target("surf", LevelFilter::OFF),
        )
        .init();

    let args = Cli::parse();
    let database = args.macdive_database()?;

    match &args.command {
        Commands::LightroomMetadata(options) => {
            commands::lightroom::export_lightroom_metadata_presets(
                &database,
                options,
                &args.overrides()?.locations(),
            )
            .await?
        }
        Commands::DiffCritters => commands::critters::diff_critters(&database).await?,
        Commands::DiffCritterCategories => {
            commands::critters::diff_critter_categories(
                &database,
                args.overrides()?.critter_categories(),
            )
            .await?
        }
        Commands::CritterImport(options) => commands::critters::critter_import(options).await?,
    }
    Ok(())
}
