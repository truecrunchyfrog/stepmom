use std::{mem::transmute, sync::Arc};

use charming::{Chart, ImageRenderer};
use poise::serenity_prelude::{CreateAttachment, UserId};
use resvg::{tiny_skia::Pixmap, usvg::{Options, Transform, Tree}};
use serde::{Deserialize, Serialize};

use crate::DbConn;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
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

impl From<ChartTheme> for charming::theme::Theme {
    fn from(val: ChartTheme) -> Self {
        use ChartTheme::*;
        use charming::theme::Theme as T;
        match val {
            Dark => T::Dark,
            Vintage => T::Vintage,
            Westeros => T::Westeros,
            Essos => T::Essos,
            Wonderland => T::Wonderland,
            Walden => T::Walden,
            Chalk => T::Chalk,
            Infographic => T::Infographic,
            Macarons => T::Macarons,
            Roma => T::Roma,
            Shine => T::Shine,
            PurplePassion => T::PurplePassion,
            Halloween => T::Halloween,
        }
    }
}

pub async fn user_owned_themes(conn: DbConn<'_>, uid: UserId) -> anyhow::Result<Vec<ChartTheme>> {
    let uid: i64 = uid.into();

    Ok(sqlx::query!("
    SELECT chart_theme_id FROM owned_chart_themes
    WHERE user_id = (SELECT id FROM users WHERE uid = $1)
    ", uid)
        .fetch_all(conn)
        .await?
        .into_iter()
        .map(|chart_theme| unsafe { transmute::<u8, ChartTheme>(chart_theme.chart_theme_id as u8) })
        .collect())
}

pub async fn user_selected_theme(conn: DbConn<'_>, uid: UserId) -> anyhow::Result<ChartTheme> {
    let uid: i64 = uid.into();

    Ok(sqlx::query!("
    SELECT chart_theme_id FROM selected_chart_themes selected
    JOIN owned_chart_themes owned ON
        selected.owned_chart_theme_id = owned.id
    WHERE selected.user_id = (SELECT id FROM users WHERE uid = $2)
    ", uid)
        .fetch_optional(conn)
        .await?
        .map(|chart_theme| unsafe { transmute::<u8, ChartTheme>(chart_theme.chart_theme_id as u8) })
        .unwrap_or(DEFAULT_CHART_THEME))
}

pub fn render_chart_to_bytes(renderer: &mut ImageRenderer, chart: &Chart) -> anyhow::Result<Vec<u8>> {
    let svg_string = renderer.render(chart)?;

    let mut font_db = resvg::usvg::fontdb::Database::new();
    font_db.load_system_fonts();

    let options = Options {
        fontdb: Arc::new(font_db),
        ..Default::default()
    };
    let rtree = Tree::from_str(&svg_string, &options)?;

    let size = rtree.size();
    let mut pixmap = Pixmap::new(size.width() as u32, size.height() as u32).unwrap();
    resvg::render(&rtree, Transform::identity(), &mut pixmap.as_mut());

    Ok(pixmap.encode_png()?)
}

pub fn render_chart_to_attachment(renderer: &mut ImageRenderer, chart: &Chart, filename: Option<&str>) -> anyhow::Result<CreateAttachment> {
    Ok(CreateAttachment::bytes(
        render_chart_to_bytes(renderer, chart)?,
        filename.unwrap_or("chart.png")))
}
