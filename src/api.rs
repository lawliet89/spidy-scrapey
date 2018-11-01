use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use data;
use reqwest::{Client, Error};
use serde::de::DeserializeOwned;
use std::collections::VecDeque;
use std::iter::FromIterator;
use std::marker;
use std::thread::sleep;
use std::time::Duration;

pub trait PaginatedResult<T> {
    fn page(&self) -> u64;
    fn last_page(&self) -> u64;
    fn results(self) -> Vec<T>;
    fn count(&self) -> Option<usize> {
        None
    }
}

pub struct Api {
    version: String,
    format: ApiFormat,
    base_url: String,
    max_interval: u64,
}

pub enum ApiFormat {
    #[allow(dead_code)]
    Csv,
    Json,
}

pub struct PaginatedIterator<R, T> {
    base_url: String,
    client: Client,
    page_number: u64,
    total_pages: u64,
    page: VecDeque<T>,
    backoff: ExponentialBackoff,
    size_hint: Option<usize>,
    _marker: marker::PhantomData<R>,
}

impl<R, T> Iterator for PaginatedIterator<R, T>
where
    R: DeserializeOwned + PaginatedResult<T>,
{
    type Item = Result<T, Error>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.page.is_empty() {
            if self.page_number > self.total_pages {
                // We are done
                return None;
            } else {
                // Request for a new page
                let duration = self
                    .backoff
                    .next_backoff()
                    .unwrap_or_else(|| Duration::new(0, 0));
                debug!(
                    "Sleeping {}.{} seconds before the next request",
                    duration.as_secs(),
                    duration.subsec_millis()
                );

                let url = [&self.base_url, format!("{}", self.page_number).as_str()].join("/");
                debug!("Making paginated requests for API {}", url);
                let result = self.client.get(&url).send();

                if let Err(e) = result {
                    return Some(Err(e));
                }

                let result: Result<R, Error> = result.expect("OK to unwrap").json();

                if let Err(e) = result {
                    return Some(Err(e));
                }

                let result = result.expect("OK to unwrap");
                debug!("\t Page {} of {}", result.page(), result.last_page());

                self.total_pages = result.last_page();
                self.page_number = result.page() + 1;

                if self.size_hint.is_none() {
                    self.size_hint = result.count();
                }

                self.page = VecDeque::from_iter(result.results().into_iter());
            }
        }
        Some(Ok(self.page.pop_front().expect("Not to be empty")))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.size_hint)
    }
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
    pub fn new(format: ApiFormat, max_interval: u64) -> Self {
        debug!("Creating API Client for v0.9");
        Self {
            version: "v0.9".to_string(),
            format: format,
            base_url: "https://www.gw2spidy.com/api".to_string(),
            max_interval,
        }
    }

    fn new_backoff(&self) -> ExponentialBackoff {
        ExponentialBackoff {
            max_interval: Duration::new(self.max_interval, 0),
            ..Default::default()
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

    fn paginate_api_lazy<R, T>(&self, base_url: &str) -> PaginatedIterator<R, T>
    where
        R: DeserializeOwned + PaginatedResult<T>,
    {
        PaginatedIterator::<R, T> {
            base_url: base_url.to_string(),
            client: Client::new(),
            page_number: 1,
            total_pages: 1,
            page: VecDeque::new(),
            backoff: self.new_backoff(),
            size_hint: None,
            _marker: Default::default(),
        }
    }

    fn paginate_api<R, T>(&self, base_url: &str) -> Result<Vec<T>, Error>
    where
        R: DeserializeOwned + PaginatedResult<T>,
    {
        let mut page_number = 1;
        let mut total_pages = 1;
        let mut results = vec![];
        let client = Client::new();

        let mut backoff = self.new_backoff();

        debug!("Making paginated requests for API {}", base_url);

        while page_number <= total_pages {
            let duration = backoff
                .next_backoff()
                .unwrap_or_else(|| Duration::new(0, 0));
            debug!(
                "Sleeping {}.{} seconds before the next request",
                duration.as_secs(),
                duration.subsec_millis()
            );
            sleep(duration);

            let url = [base_url, format!("{}", page_number).as_str()].join("/");
            let result: R = client.get(&url).send()?.json()?;
            debug!(
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

    pub fn item_search_lazy(&self, search: &str) -> PaginatedIterator<Items, data::Item> {
        let base_url = self.api_method_url("item-search");
        let base_url = [base_url.as_str(), &format!("{}", search)].join("/");

        self.paginate_api_lazy(&base_url)
    }

    pub fn items(&self) -> Result<Vec<data::Item>, Error> {
        let base_url = self.api_method_url("items");
        let base_url = [base_url.as_str(), "all"].join("/");

        self.paginate_api::<Items, data::Item>(&base_url)
    }

    pub fn items_lazy(&self) -> PaginatedIterator<Items, data::Item> {
        let base_url = self.api_method_url("items");
        let base_url = [base_url.as_str(), "all"].join("/");

        self.paginate_api_lazy(&base_url)
    }

    pub fn item(&self, id: u64) -> Result<data::Item, Error> {
        let base_url = self.api_method_url("item");
        let url = [base_url.as_str(), &format!("{}", id)].join("/");

        let client = Client::new();
        debug!("Requesting Item data for ID {}", id);
        let result: Item = client.get(&url).send()?.json()?;
        Ok(result.result)
    }
}

impl Default for Api {
    fn default() -> Self {
        Self::new(ApiFormat::Json, 1)
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

    fn count(&self) -> Option<usize> {
        Some((self.count * self.last_page) as usize)
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

    fn count(&self) -> Option<usize> {
        Some((self.count * self.last_page) as usize)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    pub result: data::Item,
}
