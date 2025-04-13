use std::sync::Arc;

use charming::{Chart, ImageRenderer};
use poise::serenity_prelude::CreateAttachment;
use resvg::{tiny_skia::Pixmap, usvg::{Options, Transform, Tree}};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ChartTheme {
    Default = 0,
    Dark,
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

impl Into<charming::theme::Theme> for ChartTheme {
    fn into(self) -> charming::theme::Theme {
        match self {
            Self::Default => charming::theme::Theme::Default,
            Self::Dark => charming::theme::Theme::Dark,
            Self::Vintage => charming::theme::Theme::Vintage,
            Self::Westeros => charming::theme::Theme::Westeros,
            Self::Essos => charming::theme::Theme::Essos,
            Self::Wonderland => charming::theme::Theme::Wonderland,
            Self::Walden => charming::theme::Theme::Walden,
            Self::Chalk => charming::theme::Theme::Chalk,
            Self::Infographic => charming::theme::Theme::Infographic,
            Self::Macarons => charming::theme::Theme::Macarons,
            Self::Roma => charming::theme::Theme::Roma,
            Self::Shine => charming::theme::Theme::Shine,
            Self::PurplePassion => charming::theme::Theme::PurplePassion,
            Self::Halloween => charming::theme::Theme::Halloween,
        }
    }
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
