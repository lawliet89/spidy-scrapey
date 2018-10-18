#[macro_use]
extern crate serde_derive;

extern crate chrono;
extern crate csv;
extern crate failure;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

mod api;
mod data;

use csv::WriterBuilder;

fn main() -> Result<(), failure::Error> {
    let api = api::Api::default();
    let listings = api.listings(19976, api::ListingType::Buy)?;

    let mut wtr = WriterBuilder::new().from_path("test.csv")?;

    for listing in listings.iter() {
        wtr.serialize(listing)?;
    }
    Ok(())
}
