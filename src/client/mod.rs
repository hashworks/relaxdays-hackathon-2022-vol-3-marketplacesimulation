mod helper;
pub mod types;

use std::collections::HashMap;

use reqwest::StatusCode;

use self::types::*;

static USER_AGENT: &str = "marketplacesimulation-client-kromlinger-justin/0.1.0";
static HACKATHON_API_URL: &str = "https://hackathon-game.relaxdays.cloud";

macro_rules! error_failed_to_receive_response {
    () => {
        "{}: Failed to receive response, failing silently ({})"
    };
}

macro_rules! error_failed_to_parse_type {
    () => {
        "{}: Failed to parse type, failing silently ({})"
    };
}

pub struct Client {
    api_url: String,
    reqwest_client: reqwest::Client,
    user_id: String,
    api_key: String,

    pub player: Player,
    pub articles: Vec<Article>,
    pub article_price_history: HashMap<usize, ArticlePriceHistory>,
    pub tags: Vec<Tag>,
    pub tag_trend_levels: HashMap<usize, usize>,
    pub suppliers: Vec<Supplier>,
    pub listings: Vec<Listing>,

    pub bedazzlement_listings: Vec<usize>,
}

pub struct ArticlePriceHistory {
    pub supplier_price_history: Vec<f64>,
}

impl ArticlePriceHistory {
    pub fn new(supplier_price: f64) -> Self {
        Self {
            supplier_price_history: vec![supplier_price],
        }
    }

    pub fn average_price(&self) -> f64 {
        let mut sum = 0.0;
        for price in &self.supplier_price_history {
            sum += price;
        }
        sum / self.supplier_price_history.len() as f64
    }

    fn record_supplier_price(&mut self, supplier_price: f64) {
        if self.supplier_price_history.last() != Some(&supplier_price) {
            self.supplier_price_history.push(supplier_price);
        }
    }
}

impl Client {
    pub async fn new(
        api_url: Option<String>,
        user_id: String,
        api_key: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let reqwest_client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

        let api_url = match api_url {
            Some(url) => url,
            None => HACKATHON_API_URL.to_string(),
        };

        let mut client = Self {
            api_url,
            reqwest_client,
            user_id,
            api_key,

            articles: Vec::new(),
            article_price_history: HashMap::new(),
            tags: Vec::new(),
            tag_trend_levels: HashMap::new(),
            suppliers: Vec::new(),
            player: Player {
                id: 0,
                money: std::f64::MAX, // Unrealistic number, so we can initialize cleanly
                stock: Vec::new(),
            },
            listings: Vec::new(),

            bedazzlement_listings: Vec::new(),
        };

        // This will be the only time we return a hard error
        if !client.fetch_player_self().await {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to fetch player self",
            )));
        }
        if !client.fetch_articles().await {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to fetch articles",
            )));
        }
        if !client.fetch_tags().await {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to fetch tags",
            )));
        }
        if !client.fetch_suppliers().await {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to fetch suppliers",
            )));
        }
        if !client.fetch_listings().await {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to fetch listings",
            )));
        }

        Ok(client)
    }

    fn requestbuilder(
        &self,
        endpoint: &str,
        request_type: reqwest::Method,
    ) -> reqwest::RequestBuilder {
        self.reqwest_client
            .request(request_type, &format!("{}{}", self.api_url, endpoint))
    }

    fn authorized_requestbuilder(
        &self,
        endpoint: &str,
        request_type: reqwest::Method,
    ) -> reqwest::RequestBuilder {
        self.requestbuilder(endpoint, request_type)
            .basic_auth(&self.user_id, Some(&self.api_key))
    }

    pub async fn fetch_articles(&mut self) -> bool {
        let endpoint = "/article";

        let res = match self
            .requestbuilder(endpoint, reqwest::Method::GET)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!(error_failed_to_receive_response!(), endpoint, e);
                return false;
            }
        };

        let status = res.status();

        let latest = match res.json::<Vec<Article>>().await {
            Ok(parsed_type) => parsed_type,
            Err(e) => {
                eprintln!(error_failed_to_parse_type!(), endpoint, e);
                return false;
            }
        };

        self.articles = latest.clone();

        status == StatusCode::OK
    }

    pub async fn fetch_tags(&mut self) -> bool {
        let endpoint = "/tag";

        let res = match self
            .requestbuilder(endpoint, reqwest::Method::GET)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!(error_failed_to_receive_response!(), endpoint, e);
                return false;
            }
        };

        let status = res.status();

        let latest = match res.json::<Vec<Tag>>().await {
            Ok(parsed_type) => parsed_type,
            Err(e) => {
                eprintln!(error_failed_to_parse_type!(), endpoint, e);
                return false;
            }
        };

        if self.tags != latest {
            // Tag-List change, initialize the tag trend levels
            for tag in &latest {
                if !self.tag_trend_levels.contains_key(&tag.id) {
                    self.tag_trend_levels.insert(tag.id, 0);
                }
            }
        }

        self.tags = latest.clone();

        status == StatusCode::OK
    }

    pub async fn fetch_suppliers(&mut self) -> bool {
        let endpoint = "/supplier";

        let res = match self
            .requestbuilder(endpoint, reqwest::Method::GET)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!(error_failed_to_receive_response!(), endpoint, e);
                return false;
            }
        };

        let status = res.status();

        let latest = match res.json::<Vec<Supplier>>().await {
            Ok(parsed_type) => parsed_type,
            Err(e) => {
                eprintln!(error_failed_to_parse_type!(), endpoint, e);
                return false;
            }
        };

        if self.suppliers != latest {
            // Supplier-Stock changes, save potential article price changes
            // WARNING: Due to a bug that won't be fixed, all suppliers have the same price for an article
            latest
                .iter()
                .flat_map(|supplier| {
                    supplier
                        .stock
                        .iter()
                        .map(|stock| (stock.article_id, stock.price))
                })
                .for_each(|(article_id, price)| {
                    self.article_price_history
                        .entry(article_id)
                        .and_modify(|article_price_history| {
                            article_price_history.record_supplier_price(price)
                        })
                        .or_insert_with(|| ArticlePriceHistory::new(price));
                });
        }

        self.suppliers = latest.clone();

        status == StatusCode::OK
    }

    pub async fn buy_from_supplier(
        &mut self,
        supplier_id: usize,
        article_id: usize,
        count: usize,
        price_per_unit: f64,
    ) -> bool {
        let endpoint = format!("/supplier/{}/article/{}/buy", supplier_id, article_id);

        let res = match self
            .authorized_requestbuilder(&endpoint, reqwest::Method::POST)
            .json(&BuyFromSupplierBody {
                count,
                price_per_unit,
            })
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!(error_failed_to_receive_response!(), endpoint, e);
                return false;
            }
        };

        res.status() == StatusCode::OK
    }

    pub async fn fetch_player_self(&mut self) -> bool {
        let endpoint = "/player/self";

        let res = match self
            .authorized_requestbuilder(endpoint, reqwest::Method::GET)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!(error_failed_to_receive_response!(), endpoint, e);
                return false;
            }
        };

        let status = res.status();

        let mut latest = match res.json::<Vec<Player>>().await {
            Ok(parsed_type) => parsed_type,
            Err(e) => {
                eprintln!(error_failed_to_parse_type!(), endpoint, e);
                return false;
            }
        };

        if latest.len() != 1 {
            eprintln!(
                error_failed_to_parse_type!(),
                endpoint, "Expected one player"
            );
            return false;
        }

        self.player = latest.pop().unwrap().clone();

        status == StatusCode::OK
    }

    pub async fn fetch_listings(&mut self) -> bool {
        let endpoint = "/listing";

        let res = match self
            .requestbuilder(endpoint, reqwest::Method::GET)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!(error_failed_to_receive_response!(), endpoint, e);
                return false;
            }
        };

        let status = res.status();

        let latest = match res.json::<Vec<Listing>>().await {
            Ok(parsed_type) => parsed_type,
            Err(e) => {
                eprintln!(error_failed_to_parse_type!(), endpoint, e);
                return false;
            }
        };

        self.listings = latest.clone();

        status == StatusCode::OK
    }

    pub async fn create_listing(
        &mut self,
        article_id: usize,
        count: usize,
        price_per_unit: f64,
    ) -> Option<usize> {
        let endpoint = "/listing/new";

        let res = match self
            .authorized_requestbuilder(&endpoint, reqwest::Method::POST)
            .json(&CreateListingBody {
                article: article_id,
                count,
                price: price_per_unit,
            })
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!(error_failed_to_receive_response!(), endpoint, e);
                return None;
            }
        };

        let json = match res.json::<HashMap<String, usize>>().await {
            Ok(parsed_type) => parsed_type,
            Err(e) => {
                eprintln!(error_failed_to_parse_type!(), endpoint, e);
                return None;
            }
        };

        match json.get("id") {
            Some(id) => Some(*id),
            None => {
                eprintln!(error_failed_to_parse_type!(), endpoint, "No id found");
                None
            }
        }
    }

    pub async fn _delete_listing(&mut self, listing_id: usize) -> bool {
        let endpoint = format!("/listing/{}", listing_id);

        let res = match self
            .authorized_requestbuilder(&endpoint, reqwest::Method::DELETE)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!(error_failed_to_receive_response!(), endpoint, e);
                return false;
            }
        };

        res.status() == StatusCode::OK
    }

    pub async fn update_listing(
        &mut self,
        listing_id: usize,
        count: usize,
        price_per_unit: f64,
    ) -> bool {
        let endpoint = format!("/listing/{}", listing_id);

        let res = match self
            .authorized_requestbuilder(&endpoint, reqwest::Method::PUT)
            .json(&UpdateListingBody {
                count,
                price: price_per_unit,
            })
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                eprintln!(error_failed_to_receive_response!(), endpoint, e);
                return false;
            }
        };

        res.status() == StatusCode::OK
    }
}
