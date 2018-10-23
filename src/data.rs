use chrono::{DateTime, Utc};
use serde::Deserializer;

// From https://serde.rs/enum-number.html
macro_rules! enum_number {
    ($name:ident { $($variant:ident = $value:expr, )* }) => {
        use std::fmt;

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $name {
            $($variant = $value,)*
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                // Serialize the enum as a u64.
                serializer.serialize_u64(*self as u64)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                struct Visitor;

                impl<'de> ::serde::de::Visitor<'de> for Visitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("positive integer")
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<$name, E>
                    where
                        E: ::serde::de::Error,
                    {
                        // Rust does not come with a simple way of converting a
                        // number to an enum, so use a big `match`.
                        match value {
                            $( $value => Ok($name::$variant), )*
                            _ => Err(E::custom(
                                format!("unknown {} value: {}",
                                stringify!($name), value))),
                        }
                    }
                }

                // Deserialize the enum from a u64.
                deserializer.deserialize_u64(Visitor)
            }
        }
    }
}

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
}

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
