use std::{mem::transmute, sync::Arc};

use charming::{Chart, ImageRenderer};
use poise::serenity_prelude::{CreateAttachment, UserId};
use resvg::{tiny_skia::Pixmap, usvg::{Options, Transform, Tree}};
use serde::{Deserialize, Serialize};

use crate::DbConn;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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

impl Into<charming::theme::Theme> for ChartTheme {
    fn into(self) -> charming::theme::Theme {
        use charming::theme::Theme as T;
        match self {
            Self::Dark => T::Dark,
            Self::Vintage => T::Vintage,
            Self::Westeros => T::Westeros,
            Self::Essos => T::Essos,
            Self::Wonderland => T::Wonderland,
            Self::Walden => T::Walden,
            Self::Chalk => T::Chalk,
            Self::Infographic => T::Infographic,
            Self::Macarons => T::Macarons,
            Self::Roma => T::Roma,
            Self::Shine => T::Shine,
            Self::PurplePassion => T::PurplePassion,
            Self::Halloween => T::Halloween,
        }
    }
}

pub async fn get_user_theme(conn: DbConn<'_>, uid: UserId) -> anyhow::Result<ChartTheme> {
    let uid = i64::from(uid);

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

pub fn render_chart_to_attachment(renderer: &mut ImageRenderer, chart: &Chart) -> anyhow::Result<CreateAttachment> {
    Ok(CreateAttachment::bytes(render_chart_to_bytes(renderer, chart)?, "chart.png"))
}
