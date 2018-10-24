use chrono::{DateTime, Utc};

// Hard-coded from https://www.gw2spidy.com/api/v0.9/json/rarities
enum_number!(Rarity {
    Junk = 0,
    Common = 1,
    Fine = 2,
    Masterwork = 3,
    Rare = 4,
    Exotic = 5,
    Ascended = 6,
    Legendary = 7,
});

#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    #[serde(rename = "data_id")]
    pub id: u64,
    pub name: String,
    pub rarity: Rarity,
    pub restriction_level: u32,
    pub img: String,
    #[serde(with = "::custom_serde::timestamp")]
    pub price_last_changed: DateTime<Utc>,
    pub max_offer_unit_price: u64,
    pub min_sale_unit_price: u64,
    pub offer_availability: u64,
    pub sale_availability: u64,

    pub sale_price_change_last_hour: i32,
    pub offer_price_change_last_hour: i32,

    // TODO
    pub type_id: u64,
    pub sub_type_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemListing {
    #[serde(
        rename = "listing_datetime",
        with = "::custom_serde::timestamp"
    )]
    pub timestamp: DateTime<Utc>,

    pub unit_price: u64,
    pub quantity: u64,
    pub listings: u64,
}
