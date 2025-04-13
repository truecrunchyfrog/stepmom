use serde::Deserialize;

use crate::product::Product;

#[derive(Deserialize)]
pub struct ShopItem
{
    pub description: String,
    pub cost: u64,
    pub product: Product
}
