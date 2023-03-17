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

use crate::arguments::{CritterCommands, LightroomCommands};
use arguments::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    let log_level = match args.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    // Logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_filter(Targets::default().with_default(Level::DEBUG)),
        )
        .with(
            Targets::default()
                .with_target("macdive_exporter", log_level)
                .with_target("surf", LevelFilter::OFF),
        )
        .init();

    let database = args.macdive_database()?;

    match &args.command {
        Commands::Lightroom { command, options } => match command {
            LightroomCommands::ExportSites { force } => {
                commands::lightroom::export_lightroom_metadata_presets(
                    &database,
                    options,
                    &args.config()?.locations(),
                    *force,
                )
                .await?
            }
        },
        Commands::Critters { command } => match command {
            CritterCommands::Validate => {
                commands::critters::diff_critters(&database, args.offline).await?
            }
            CritterCommands::ValidateCategories => {
                commands::critters::diff_critter_categories(
                    &database,
                    &args.config()?.into(),
                    args.offline,
                )
                .await?
            }
            CritterCommands::PrepareImport(options) => {
                commands::critters::critter_import(options, &args.config()?.into(), args.offline)
                    .await?
            }
        },
    }
    Ok(())
}
