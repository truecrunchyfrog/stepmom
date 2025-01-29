use std::sync::Arc;

use charming::{Chart, ImageRenderer};
use poise::serenity_prelude::CreateAttachment;
use resvg::{tiny_skia::Pixmap, usvg::{Options, Transform, Tree}};

use crate::Error;

pub fn render_chart_to_bytes(renderer: &mut ImageRenderer, chart: &Chart) -> Result<Vec<u8>, Error> {
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

pub fn render_chart_to_attachment(renderer: &mut ImageRenderer, chart: &Chart) -> Result<CreateAttachment, Error> {
    Ok(CreateAttachment::bytes(render_chart_to_bytes(renderer, chart)?, "chart.png"))
}
