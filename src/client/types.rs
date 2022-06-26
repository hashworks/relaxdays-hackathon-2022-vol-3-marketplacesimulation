use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct Article {
    pub id: usize,
    pub tags: Vec<usize>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct Tag {
    pub id: usize,
    pub similar_tags: Vec<usize>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Supplier {
    pub id: usize,
    pub stock: Vec<Stock>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Stock {
    pub article_id: usize,
    pub stock: usize,
    pub price: f64,
}

impl Eq for Stock {}

impl PartialEq for Stock {
    fn eq(&self, other: &Self) -> bool {
        // f64 doesn't implement Eq, so lets convert them to be on the safe site
        // Prices always have a maximum of six digits after the floating point.
        self.article_id == other.article_id
            && self.stock == other.stock
            && (self.price * 1e6) as usize == (other.price * 1e6) as usize
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct BuyFromSupplierBody {
    pub count: usize,
    pub price_per_unit: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Player {
    pub id: usize,
    pub money: f64,
    pub stock: Vec<PlayerStock>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct PlayerStock {
    pub article_id: usize,
    pub stock: usize,
}

impl Eq for Player {}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        // f64 doesn't implement Eq, so lets convert them to be on the safe site
        // Prices always have a maximum of six digits after the floating point.
        self.id == other.id
            && self.stock == other.stock
            && (self.money * 1e6) as usize == (other.money * 1e6) as usize
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Listing {
    pub id: usize,
    pub player: usize,
    pub article: usize,
    pub count: usize,
    pub price: f64,
}

impl Eq for Listing {}

impl PartialEq for Listing {
    fn eq(&self, other: &Self) -> bool {
        // f64 doesn't implement Eq, so lets convert them to be on the safe site
        // Prices always have a maximum of six digits after the floating point.
        self.id == other.id
            && self.player == other.player
            && self.player == other.player
            && self.article == other.count
            && (self.price * 1e6) as usize == (other.price * 1e6) as usize
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateListingBody {
    pub article: usize,
    pub count: usize,
    pub price: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct UpdateListingBody {
    pub count: usize,
    pub price: f64,
}
