use data;
use reqwest::{Client, Error};
use serde_json;

pub struct Api {
    version: String,
    format: ApiFormat,
    base_url: String,
}

pub enum ApiFormat {
    Csv,
    Json,
}

impl ToString for ApiFormat {
    fn to_string(&self) -> String {
        match self {
            ApiFormat::Csv => "csv".to_string(),
            ApiFormat::Json => "json".to_string(),
        }
    }
}

impl Api {
    fn new(format: ApiFormat) -> Self {
        Self {
            version: "v0.9".to_string(),
            format: format,
            base_url: "https://www.gw2spidy.com/api".to_string(),
        }
    }

    fn api_method_url(&self, method: &str) -> String {
        [
            self.base_url.as_str(),
            self.version.as_str(),
            self.format.to_string().as_str(),
            method,
        ]
            .join("/")
    }

    pub fn listings(
        &self,
        item_id: u64,
        listing_type: ListingType,
    ) -> Result<Vec<data::ItemListing>, Error> {
        let base_url = self.api_method_url("listings");
        let base_url = [
            base_url.as_str(),
            &format!("{}", item_id),
            listing_type.to_string().as_str(),
        ]
            .join("/");

        let mut page_number = 1;
        let mut total_pages = 1;
        let mut results = vec![];
        let client = Client::new();

        while page_number <= total_pages {
            let url = [base_url.as_str(), format!("{}", page_number).as_str()].join("/");
            let result = client.get(&url).send()?.text()?;

            let mut result: ItemListings = serde_json::from_str(&result).unwrap();

            total_pages = result.last_page;
            page_number = result.page + 1;

            results.append(&mut result.results);
        }

        Ok(results)
    }
}

impl Default for Api {
    fn default() -> Self {
        Self::new(ApiFormat::Json)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ListingType {
    Sell,
    Buy,
}

impl ToString for ListingType {
    fn to_string(&self) -> String {
        match self {
            ListingType::Sell => "sell".to_string(),
            ListingType::Buy => "buy".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemListings {
    #[serde(rename = "sell-or-buy")]
    pub listing_type: ListingType,
    pub count: u64,
    pub page: u64,
    pub last_page: u64,
    pub total: u64,
    pub results: Vec<data::ItemListing>,
}

// https://www.gw2spidy.com/api/v0.9/csv/listings/19976/buy/1
