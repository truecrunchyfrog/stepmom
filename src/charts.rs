use anyhow::anyhow;
use plotters::{coord::Shift, prelude::*};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use poise::serenity_prelude::{CreateAttachment, UserId};
use serde::{Deserialize, Serialize};

use crate::DbConn;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, FromPrimitive)]
pub enum ChartTheme {
    Dark = 0,
    Vintage,
    Westeros,
    Essos,
    Wonderland,
    Walden,
    Chalk,
    Infographic,
    Macarons,
    Roma,
    Shine,
    PurplePassion,
    Halloween
}

const DEFAULT_CHART_THEME: ChartTheme = ChartTheme::Walden;

pub async fn user_owned_themes(conn: DbConn<'_>, uid: UserId) -> anyhow::Result<Vec<ChartTheme>> {
    let uid: i64 = uid.into();

    sqlx::query!("
    SELECT chart_theme_id FROM owned_chart_themes
    WHERE user_id = (SELECT id FROM users WHERE uid = $1)
    ", uid)
        .fetch_all(conn)
        .await?
        .into_iter()
        .map(|chart_theme|
            ChartTheme::from_i64(chart_theme.chart_theme_id).ok_or(anyhow!("Invalid chart theme with ID.")))
        .collect()
}

pub async fn user_selected_theme(conn: DbConn<'_>, uid: UserId) -> anyhow::Result<ChartTheme> {
    let uid: i64 = uid.into();

    sqlx::query!("
    SELECT chart_theme_id FROM selected_chart_themes selected
    JOIN owned_chart_themes owned ON
        selected.owned_chart_theme_id = owned.id
    WHERE selected.user_id = (SELECT id FROM users WHERE uid = $2)
    ", uid)
        .fetch_optional(conn)
        .await?
        .map(|chart_theme|
            ChartTheme::from_i64(chart_theme.chart_theme_id).ok_or(anyhow!("Invalid chart theme with ID.")))
        .unwrap_or(Ok(DEFAULT_CHART_THEME))
}

pub async fn render_chart_to_bytes(chart: impl AsyncFnOnce(DrawingArea<BitMapBackend<'_>, Shift>) -> anyhow::Result<()>) -> anyhow::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    {
        let root_drawing_area = BitMapBackend::with_buffer(&mut buffer, (1024, 512)).into_drawing_area();
        chart(root_drawing_area).await?;
    }
    Ok(buffer)
}

pub fn bytes_to_attachment(bytes: Vec<u8>, filename: Option<&str>) -> CreateAttachment {
    CreateAttachment::bytes(bytes, filename.unwrap_or("chart.png"))
}
