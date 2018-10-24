#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

extern crate backoff;
extern crate chrono;
extern crate csv;
extern crate failure;
extern crate itertools;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate stderrlog;

#[macro_use]
mod custom_serde;
mod api;
mod data;

use chrono::{DateTime, Utc};
use clap::{App, AppSettings, Arg, ArgMatches};
use itertools::Itertools;
use std::fs;
use std::path::Path;
use std::str::FromStr;

// Output Listing
#[derive(Serialize, Debug)]
struct ListingOutput<'a> {
    #[serde(with = "::custom_serde::timestamp")]
    pub timestamp: &'a DateTime<Utc>,

    #[serde(rename = "type")]
    pub listing_type: api::ListingType,
    pub unit_price: u64,
    pub quantity: u64,
    pub listings: u64,
}

impl<'a> ListingOutput<'a> {
    pub fn from_listing(listing: &'a data::ItemListing, listing_type: api::ListingType) -> Self {
        Self {
            timestamp: &listing.timestamp,
            listing_type,
            unit_price: listing.unit_price,
            quantity: listing.quantity,
            listings: listing.listings,
        }
    }
}

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
                .number_of_values(1)
                .long("item-id")
                .short("i")
                .required_unless("item_name")
                .required_unless("all"),
        ).arg(
            Arg::with_name("item_name")
                .help("Item name to search for in lieu of specifying an item ID")
                .long("item-name")
                .short("n")
                .takes_value(true)
                .number_of_values(1)
                .multiple(true)
                .required_unless("item_id")
                .required_unless("all"),
        ).arg(
            Arg::with_name("all")
                .help("Find pricing data for all items")
                .long("all")
                .short("a")
                .conflicts_with_all(&["item_id", "item_name"]),
        ).arg(
            Arg::with_name("output")
                .help("Path to directory to output CSV files to")
                .default_value("output")
                .takes_value(true),
        ).arg(
            Arg::with_name("max_backoff")
                .help(
                    "Max duration, in seconds, for the exponential Backoff delay between API calls",
                ).default_value("1")
                .long("--max-backoff")
                .takes_value(true),
        )
}

fn search_items<'a>(
    api: &api::Api,
    args: &ArgMatches<'a>,
) -> Result<Vec<data::Item>, failure::Error> {
    let item_names: Vec<&str> = match args.values_of("item_name") {
        Some(items) => items.collect(),
        None => return Ok(vec![]),
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
        .into_iter();

    let item_searches = Iterator::flatten(item_searches).collect::<Vec<_>>();

    info!("Items found: {:?}", item_searches);

    Ok(item_searches)
}

fn listings<'a, I>(api: &api::Api, args: &ArgMatches<'a>, items: I) -> Result<(), failure::Error>
where
    I: Iterator<Item = data::Item>,
{
    let items = items.unique();
    let output = args.value_of("output").expect("Value to be present");
    let output = Path::new(output);

    let output = if output.is_relative() {
        std::env::current_dir()?.join(output)
    } else {
        output.to_path_buf()
    };

    fs::create_dir_all(&output)?;

    for item in items {
        let buy = api.listings(item.id, api::ListingType::Buy)?;
        let sell = api.listings(item.id, api::ListingType::Sell)?;

        let buy_output = buy
            .iter()
            .map(|listing| ListingOutput::from_listing(listing, api::ListingType::Buy))
            .rev();

        let sell_output = sell
            .iter()
            .map(|listing| ListingOutput::from_listing(listing, api::ListingType::Sell))
            .rev();

        let listings_output =
            buy_output.merge_by(sell_output, |left, right| left.timestamp <= right.timestamp);

        let path = output.join(format!("{}.csv", item.name));
        let mut wtr = csv::Writer::from_path(&path)?;

        for listing in listings_output {
            wtr.serialize(listing)?;
        }
    }
    Ok(())
}

fn main() -> Result<(), failure::Error> {
    let args = make_parser().get_matches();
    let verbose = args.occurrences_of("verbosity") as usize;
    let verbose = if verbose == 0 { 2 } else { verbose };

    stderrlog::new().verbosity(verbose).init()?;

    let api = api::Api::new(
        api::ApiFormat::Json,
        value_t!(args, "max_backoff", u64).unwrap_or_else(|e| e.exit()),
    );

    let item_ids: Box<Iterator<Item = data::Item>> = if args.occurrences_of("all") > 0 {
        info!("Retrieving data for ALL items");
        Box::new(api.items()?.into_iter())
    } else {
        let args_item_ids: Vec<u64> = match args.values_of("item_id") {
            Some(ids) => {
                let ids: Result<Vec<u64>, _> = ids.map(FromStr::from_str).collect();
                ids?
            }
            None => vec![],
        };

        let items = args_item_ids
            .into_iter()
            .map(|id| api.item(id))
            .collect::<Result<Vec<data::Item>, _>>()?;

        Box::new(
            items
                .into_iter()
                .merge_by(search_items(&api, &args)?.into_iter(), |left, right| {
                    left.id <= right.id
                }),
        )
    };

    listings(&api, &args, item_ids.into_iter())
}
