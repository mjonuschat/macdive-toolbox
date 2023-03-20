use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressState, ProgressStyle};
use std::time::Duration;
use tracing::Level;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

mod arguments;
mod commands;
mod errors;
mod helpers;
mod inaturalist;
mod macdive;
mod parsers;
mod types;

use crate::arguments::{CritterCommands, LightroomCommands, MtpCommands};
use crate::helpers::database;
use arguments::{Cli, Commands};
use migration::{Migrator, MigratorTrait};

fn setup_logging(verbose: u8) -> Result<()> {
    let log_level = match verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let indicatif_layer = IndicatifLayer::new()
        .with_progress_style(
            ProgressStyle::with_template(
                "{color_start}{span_child_prefix}{span_fields} -- {span_name} {wide_msg} {elapsed_subsec}{color_end}",
            )?
                .with_key(
                    "elapsed_subsec",
                    helpers::progress::elapsed_subsec,
                )
                .with_key(
                    "color_start",
                    |state: &ProgressState, writer: &mut dyn std::fmt::Write| {
                        let elapsed = state.elapsed();

                        if elapsed > Duration::from_secs(8) {
                            // Red
                            let _ = write!(writer, "\x1b[{}m", 1 + 30);
                        } else if elapsed > Duration::from_secs(4) {
                            // Yellow
                            let _ = write!(writer, "\x1b[{}m", 3 + 30);
                        }
                    },
                )
                .with_key(
                    "color_end",
                    |state: &ProgressState, writer: &mut dyn std::fmt::Write| {
                        if state.elapsed() > Duration::from_secs(4) {
                            let _ =write!(writer, "\x1b[0m");
                        }
                    },
                ),
        )
        .with_span_child_prefix_symbol("↳ ")
        .with_span_child_prefix_indent(" ")
        .with_max_progress_bars(
            20,
            Some(
                ProgressStyle::with_template(
                    "...and {pending_progress_bars} more not shown above.",
                )?
            ),
        );

    // Logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_writer(indicatif_layer.get_stderr_writer())
                .with_filter(Targets::default().with_default(Level::TRACE)),
        )
        .with(indicatif_layer)
        .with(
            Targets::default()
                .with_target("macdive_toolbox", log_level)
                .with_target("surf", LevelFilter::OFF),
        )
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    setup_logging(args.verbose)?;

    // Apply all pending migrations
    let db = database::connect().await?;
    Migrator::up(db, None).await?;

    match &args.command {
        Commands::Lightroom { command, options } => match command {
            LightroomCommands::ExportSites { force } => {
                commands::lightroom::export_lightroom_metadata_presets(
                    &args.macdive_database()?,
                    options,
                    &args.config()?.locations(),
                    *force,
                )
                .await?
            }
        },
        Commands::Critters { command } => match command {
            CritterCommands::Validate => {
                commands::critters::diff_critters(&args.macdive_database()?, args.offline).await?
            }
            CritterCommands::ValidateCategories => {
                commands::critters::diff_critter_categories(
                    &args.macdive_database()?,
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
        Commands::Mtp { command, options } => match command {
            MtpCommands::Detect => commands::mtp::detect(args.verbose)?,
            MtpCommands::ListFiles { .. } => {
                let verbose = args.verbose > 0;
                commands::mtp::listfiles(options.to_owned().into(), verbose)?
            }
            MtpCommands::Sync(params) => commands::mtp::sync(options, params)?,
        },
    }

    Ok(())
}
