use data;
use reqwest::{Client, Error};
use serde::de::DeserializeOwned;
use serde_json;

trait PaginatedResult<T> {
    fn page(&self) -> u64;
    fn last_page(&self) -> u64;
    fn results(self) -> Vec<T>;
}

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
        info!("Creating API Client for v0.9");
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

    fn paginate_api<R, T>(&self, base_url: &str) -> Result<Vec<T>, Error>
    where
        R: DeserializeOwned + PaginatedResult<T>,
    {
        let mut page_number = 1;
        let mut total_pages = 1;
        let mut results = vec![];
        let client = Client::new();

        info!("Making paginated requests for API {}", base_url);

        while page_number <= total_pages {
            let url = [base_url, format!("{}", page_number).as_str()].join("/");
            let result = client.get(&url).send()?.text()?;

            let result: R = serde_json::from_str(&result).unwrap();
            info!(
                "\t fetching page {} of {}",
                result.page(),
                result.last_page()
            );

            total_pages = result.last_page();
            page_number = result.page() + 1;

            results.append(&mut result.results());
        }

        Ok(results)
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

        self.paginate_api::<ItemListings, data::ItemListing>(&base_url)
    }

    pub fn item_search(&self, search: &str) -> Result<Vec<data::Item>, Error> {
        let base_url = self.api_method_url("item-search");
        let base_url = [base_url.as_str(), &format!("{}", search)].join("/");

        self.paginate_api::<Items, data::Item>(&base_url)
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
pub struct Items {
    pub count: u64,
    pub page: u64,
    pub last_page: u64,
    pub results: Vec<data::Item>,
}

impl PaginatedResult<data::Item> for Items {
    fn page(&self) -> u64 {
        self.page
    }

    fn last_page(&self) -> u64 {
        self.last_page
    }

    fn results(self) -> Vec<data::Item> {
        self.results
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

impl PaginatedResult<data::ItemListing> for ItemListings {
    fn page(&self) -> u64 {
        self.page
    }

    fn last_page(&self) -> u64 {
        self.last_page
    }

    fn results(self) -> Vec<data::ItemListing> {
        self.results
    }
}
