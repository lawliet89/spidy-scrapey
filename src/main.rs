#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

extern crate chrono;
extern crate csv;
extern crate failure;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate stderrlog;

mod api;
mod data;

use clap::{App, AppSettings, Arg, ArgMatches};
use csv::WriterBuilder;
use std::str::FromStr;

fn make_parser<'a, 'b>() -> App<'a, 'b>
where
    'a: 'b,
{
    App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .global_setting(AppSettings::DontCollapseArgsInUsage)
        .global_setting(AppSettings::NextLineHelp)
        .about(
            "Fetch price listing data from GW2Spidy. \
             Specify items by IDs or their names.",
        ).arg(
            Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Increase message verbosity"),
        ).arg(
            Arg::with_name("item_id")
                .help("Item ID to fetch pricing data for")
                .required(true)
                .multiple(true)
                .takes_value(true)
                .required_unless("item_name"),
        ).arg(
            Arg::with_name("item_name")
                .help("Item name to search for in lieu of specifying an item ID")
                .long("item-name")
                .short("n")
                .takes_value(true)
                .number_of_values(1)
                .multiple(true),
        )
}

fn main() -> Result<(), failure::Error> {
    let args = make_parser().get_matches();
    let verbose = args.occurrences_of("verbosity") as usize;
    stderrlog::new().verbosity(verbose).init()?;

    let api = api::Api::default();

    let item_names: Vec<&str> = match args.values_of("item_name") {
        Some(items) => items.collect(),
        None => vec![],
    };

    info!("Item names to search for: {}", item_names.join(", "));
    let item_searches = item_names
        .into_iter()
        .map(|item| {
            let result = api.item_search(item);
            match &result {
                Ok(v) => {
                    let len = v.len();
                    if len == 0 {
                        warn!("Search term \"{}\" return no result", item);
                    } else if len > 1 {
                        warn!(
                            "Search term \"{}\" returned more than one results ({} found)",
                            item, len
                        );
                    }
                }
                Err(e) => error!("Error searching for \"{}\": {}", item, e),
            }
            result
        }).collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    info!("Items found: {:?}", item_searches);

    let item_ids: Vec<u64> = match args.values_of("item_id") {
        Some(ids) => {
            let ids: Result<Vec<u64>, _> = ids.map(FromStr::from_str).collect();
            ids?
        }
        None => vec![],
    };

    // let item_searches =

    // let listings = api.listings(19976, api::ListingType::Buy)?;

    // let mut wtr = WriterBuilder::new().from_path("test.csv")?;

    // for listing in listings.iter() {
    //     wtr.serialize(listing)?;
    // }
    Ok(())
}
