use chrono::{DateTime, Utc};
use serde::Deserializer;

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemListing {
    // serialize_with = "serialize_time"
    #[serde(rename = "listing_datetime", with = "timestamp")]
    pub timestamp: DateTime<Utc>,

    pub unit_price: u64,
    pub quantity: u64,
    pub listings: u64,
}

mod timestamp {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    // 2012-09-08 00:00:00 UTC
    const FORMAT: &str = "%F %T UTC";

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}
