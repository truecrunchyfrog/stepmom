use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageReader, Pixel, Rgb, RgbaImage};
use sqlx::SqlitePool;
use rand::prelude::*;

pub struct TradeCard {
    id: i64,
    name: String,
    quote: String,
    author_id: i64,
    weight: i64,
    color: i64,
    emote_id: i64,
    sell_coins: i64
}

impl TradeCard {
    pub fn create_card_image(&self) -> anyhow::Result<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
        Ok(add_border_to_image(
            ImageReader::open(Path::new("trade-card-images").join(self.id.to_string()))?.decode()?,
            Rgb::<u8>([
                (self.color / (256 * 256)) as u8,
                (self.color / 256 % 256) as u8,
                (self.color % 256) as u8
            ]).to_rgba(),
            50))
    }
}

pub async fn fetch_cards(pool: &SqlitePool) -> anyhow::Result<Vec<TradeCard>> {
    Ok(sqlx::query_as!(TradeCard, r#"
    SELECT c.id, c.name, c.quote, c.author_id, r.weight, r.color, r.emote_id, r.sell_coins
    FROM trade_cards c
    JOIN trade_card_rarities r
        ON c.rarity_id = r.id
    "#)
        .fetch_all(pool)
        .await?)
}

pub fn pick_random_card<'a>(cards: &'a Vec<TradeCard>) -> anyhow::Result<&'a TradeCard> {
    Ok(cards.choose_weighted(&mut thread_rng(), |card| card.weight)?)
}

pub fn add_border_to_image(image: DynamicImage, border_color: image::Rgba<u8>, border_thickness: u32) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let (width, height) = image.dimensions();

    let mut bordered_image = RgbaImage::from_fn(
        width + 2 * border_thickness,
        height + 2 * border_thickness,
        |_, _| border_color);

    image::imageops::overlay(
        &mut bordered_image,
        &image,
        border_thickness.into(),
        border_thickness.into());

    bordered_image
}
