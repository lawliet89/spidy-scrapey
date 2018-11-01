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
use std::fmt;
use std::fs;
use std::ops::Deref;
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

/// "Total" count
pub struct Total(Option<usize>);

impl Deref for Total {
    type Target = Option<usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Total {
    fn default() -> Self {
        Total(None)
    }
}

impl fmt::Display for Total {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "unknown"),
            Some(count) => write!(f, "{}", count),
        }
    }
}

impl From<Option<usize>> for Total {
    fn from(count: Option<usize>) -> Self {
        Total(count)
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

fn listings<'a, I>(api: &api::Api, args: &ArgMatches<'a>, items: I) -> Result<(), failure::Error>
where
    I: Iterator<Item = Result<data::Item, reqwest::Error>>,
{
    let output = args.value_of("output").expect("Value to be present");
    let output = Path::new(output);

    let output = if output.is_relative() {
        std::env::current_dir()?.join(output)
    } else {
        output.to_path_buf()
    };

    fs::create_dir_all(&output)?;

    let mut counter: usize = 1;

    let mut items = items.peekable();
    let (_, hint) = items.size_hint();
    let total: Total = From::from(hint);

    while items.peek().is_some() {
        let item = items.next().expect("to be some");
        match item {
            Ok(item) => {
                match listing(api, &item, &total, counter, &output) {
                    Ok(()) => {
                        counter = counter + 1;
                    }
                    Err(e) => {
                        error!("Error with item {}: {}", item.name, e);
                    }
                };
            }
            Err(e) => error!("{}", e),
        }
    }

    Ok(())
}

fn listing(
    api: &api::Api,
    item: &data::Item,
    total: &Total,
    counter: usize,
    output: &Path,
) -> Result<(), failure::Error> {
    info!(
        "[{} of {}] Fetching item listings for \"{}\"",
        counter, total, item.name
    );
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
    info!(
        "[{} of {}] Writing item listings for \"{}\" to \"{}\"",
        counter,
        total,
        item.name,
        path.to_str().unwrap_or_else(|| "unknown")
    );
    let mut wtr = csv::Writer::from_path(&path)?;

    for listing in listings_output {
        wtr.serialize(listing)?;
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

    if args.occurrences_of("all") > 0 {
        info!("Retrieving data for ALL items");
        listings(&api, &args, api.items_lazy())
    } else {
        let args_item_ids: Vec<u64> = match args.values_of("item_id") {
            Some(ids) => {
                let ids: Result<Vec<u64>, _> = ids.map(FromStr::from_str).collect();
                ids?
            }
            None => vec![],
        };

        let items = args_item_ids.into_iter().map(|id| api.item(id));

        let item_names: Vec<&str> = match args.values_of("item_name") {
            Some(items) => items.collect(),
            None => vec![],
        };

        let item_searches = item_names.into_iter().map(|item| {
            info!("Including items from search term \"{}\"", item);
            api.item_search_lazy(item)
        });

        let items = Iterator::flatten(item_searches)
            .chain(items)
            .unique_by(|item| match item {
                Ok(v) => v.name.to_string(),
                Err(e) => format!("{}", e),
            });
        listings(&api, &args, items)
    }
}
