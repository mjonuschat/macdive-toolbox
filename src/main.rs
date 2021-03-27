#[macro_use]
extern crate diesel;

use clap::Clap;

mod arguments;
mod errors;
mod geocode;
mod lightroom;
mod macdive;
mod types;

use crate::geocode::geocode_site;
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
        .map(|site| {
            site.try_into().and_then(|site| {
                if let Some(key) = &options.api_key {
                    Ok(geocode_site(site, key)?)
                } else {
                    Ok(site)
                }
            })
        })
        .collect::<Result<Vec<types::DiveSite>, types::ConversionError>>()?;

    use prettytable::{Cell, Row, Table};
    let mut table = Table::new();
    for site in sites {
        table.add_row(Row::new(vec![
            Cell::new(&site.name),
            Cell::new(&site.locality.unwrap_or_else(|| "".to_string())),
            Cell::new(
                &site
                    .county
                    .map(|v| match v.strip_suffix(" County") {
                        Some(s) => s.to_string(),
                        None => v.to_string(),
                    })
                    .unwrap_or_else(|| "".to_string()),
            ),
            Cell::new(&site.state.unwrap_or_else(|| "".to_string())),
            Cell::new(&site.country),
        ]));
    }
    table.set_titles(Row::new(vec![
        Cell::new("Site"),
        Cell::new("Locality"),
        Cell::new("County"),
        Cell::new("State"),
        Cell::new("Country"),
    ]));

    table.printstd();
    Ok(())
}
