use anyhow::Result;
use clap::Parser;

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
    }
    Ok(())
}
