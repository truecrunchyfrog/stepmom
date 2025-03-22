use poise::serenity_prelude::Role;

use crate::booster::Booster;

pub struct ShopProduct {
    cost: u64,
    item: ProductItem
}

pub enum ProductItem {
    Booster(Booster),
    Role(Role, String),
    // ChartColor
}
