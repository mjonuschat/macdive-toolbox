#[macro_use]
extern crate diesel;

use clap::Clap;

mod arguments;
mod lightroom;
mod macdive;
mod types;

use arguments::Options;
use std::convert::TryInto;

// TODO: Exit code handling
fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    println!("Options: #{:#?}", &options);
    println!("MacDive Database: #{:#?}", options.macdive_database());
    println!(
        "Lightroom Metadata Presets: #{:#?}",
        options.lightroom_metadata()
    );

    let connection = macdive::establish_connection(&options.macdive_database()?)?;
    let sites: Vec<types::DiveSite> = macdive::sites(&connection)?
        .into_iter()
        .map(|site| site.try_into())
        .collect::<Result<Vec<types::DiveSite>, types::ConversionError>>()?;

    dbg!(sites);
    Ok(())
}
