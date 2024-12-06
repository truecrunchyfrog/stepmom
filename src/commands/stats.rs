use std::{sync::Arc, time::Duration};

use charming::{component::{Axis, Title}, element::{AreaStyle, AxisType}, series::Line, theme::Theme, Chart, ImageRenderer};
use chrono::{NaiveDate, Utc};
use humantime::{format_duration, parse_duration};
use poise::{serenity_prelude::{AutocompleteChoice, CreateAllowedMentions, CreateAttachment, MessageBuilder, User}, CreateReply};
use resvg::{tiny_skia::Pixmap, usvg::{Options, Transform, Tree}};

use crate::{leaderboard::{real_leaderboard_start_datetime, user_place}, prelude::{user_balance, ActOnUser}, study::user_streak, Context, Error};

#[derive(poise::ChoiceParameter)]
enum Statistic {
    Time,
    #[name = "Video time"]
    VideoTime,
    Balance
}

async fn autocomplete_period(
    _ctx: Context<'_>,
    _partial: &str,
) -> impl Iterator<Item = AutocompleteChoice> {
    [
        30.5,
        3.0 * 30.5,
        6.0 * 30.5,
        9.0 * 30.5,
        365.0,
        7.0,
        7.0 * 2.0,
        7.0 * 3.0,
        1.0
    ].iter().map(|&s| {
        let duration_str = format_duration(Duration::from_secs_f32(s * 24.0 * 60.0 * 60.0)).to_string();
        AutocompleteChoice::new(duration_str.clone(), duration_str.clone())
    })
}

/// View user statistics and data.
#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn stats(
    ctx: Context<'_>,
    #[description = "User to view stats for"]
    user: Option<User>,
    #[description = "Statistic to view"]
    statistic: Option<Statistic>,
    #[description = "Date"]
    date: Option<String>,
    #[description = "Period"]
    #[autocomplete = "autocomplete_period"]
    period: Option<String>
) -> Result<(), Error> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());

    match statistic {
        Some(stat) => {
            let msg = ctx.send(
                CreateReply::default()
                .content("Generating chart...")
            ).await?;

            let uid = i64::from(user.id);

            let date = date.map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d"))
                .unwrap_or(Ok(Utc::now().date_naive()))?;
            let period = period.map(|p| parse_duration(&p))
                .unwrap_or(Ok(Duration::from_secs((30.5 * 24.0 * 60.0 * 60.0) as u64)))?;

            let start = (date - chrono::Duration::from_std(period)?).to_string();
            let end = date.to_string();

            // TODO this is not ideal, very hacky
            let dates = sqlx::query!("
            WITH RECURSIVE date_range AS (
                SELECT DATE($1) AS date
                UNION ALL
                SELECT DATE(date, '+1 day')
                FROM date_range
                WHERE DATE(date, '+1 day') <= DATE($2)
            ) SELECT * FROM date_range
            ", start, end)
                .fetch_all(&ctx.data().db_pool)
                .await.unwrap();

            let (title, y_axis_label, data) = match stat {
                Statistic::Time => {
                    let data = sqlx::query!("
                    WITH RECURSIVE date_range AS (
                        SELECT DATE($1) AS date
                        UNION ALL
                        SELECT DATE(date, '+1 day')
                        FROM date_range
                        WHERE DATE(date, '+1 day') <= DATE($2)
                    ),
                    sessions_with_dates AS (
                        SELECT id, user_id, length, date
                        FROM date_range
                        LEFT JOIN study_sessions
                            ON date = DATE(ended)
                    )
                    SELECT uid, date, COALESCE(SUM(length), 0) AS daily_time
                    FROM sessions_with_dates
                    LEFT JOIN users
                        ON user_id = users.id
                    WHERE uid IS NULL OR uid = $3
                    GROUP BY user_id, date
                    ORDER BY date
                    ", start, end, uid)
                        .fetch_all(&ctx.data().db_pool)
                        .await.unwrap();

                    ("Study time", "hours/day",
                     data
                     .iter()
                     .map(|r| r.daily_time as f64 / 3600.0)
                     .collect::<Vec<_>>())
                }
                Statistic::VideoTime => {
                    let data = sqlx::query!("
                    WITH RECURSIVE date_range AS (
                        SELECT DATE($1) AS date
                        UNION ALL
                        SELECT DATE(date, '+1 day')
                        FROM date_range
                        WHERE DATE(date, '+1 day') <= DATE($2)
                    ),
                    sessions_with_dates AS (
                        SELECT id, user_id, video_length, date
                        FROM date_range
                        LEFT JOIN study_sessions
                            ON date = DATE(ended)
                    )
                    SELECT uid, date, COALESCE(SUM(video_length), 0) AS daily_video_time
                    FROM sessions_with_dates
                    LEFT JOIN users
                        ON user_id = users.id
                    WHERE uid IS NULL OR uid = $3
                    GROUP BY user_id, date
                    ORDER BY date
                    ", start, end, uid)
                        .fetch_all(&ctx.data().db_pool)
                        .await.unwrap();

                    ("Video time", "hours/day",
                     data
                     .iter()
                     .map(|r| r.daily_video_time as f64 / 3600.0)
                     .collect::<Vec<_>>())
                }
                Statistic::Balance => {
                    let data = sqlx::query!("
                    WITH RECURSIVE date_range AS (
                        SELECT DATE($1) AS date
                        UNION ALL
                        SELECT DATE(date, '+1 day')
                        FROM date_range
                        WHERE DATE(date, '+1 day') <= DATE($2)
                    ),
                    transactions_with_dates AS (
                        SELECT id, user_id, coins_diff, timestamp, date
                        FROM date_range
                        LEFT JOIN coin_transactions
                            ON date = DATE(timestamp, 'unixepoch')
                    )
                    SELECT
                        uid, date,
                        COALESCE(SUM(coins_diff) OVER (ORDER BY date), 0) AS balance
                    FROM transactions_with_dates
                    LEFT JOIN users
                        ON user_id = users.id
                    WHERE uid IS NULL OR uid = $3
                    GROUP BY user_id, date
                    ORDER BY date
                    ", start, end, uid)
                        .fetch_all(&ctx.data().db_pool)
                        .await.unwrap();

                    ("Balance", "Coins",
                         data
                         .iter()
                         .map(|r| r.balance as f64)
                         .collect::<Vec<_>>())
                }
            };

            let chart = Chart::new()
                .title(Title::new().text(title))
                .x_axis(
                    Axis::new()
                    .type_(AxisType::Category)
                    .name("Time")
                    .data(
                        dates
                        .iter()
                        .flat_map(|r| &r.date)
                        .collect()))
                .y_axis(Axis::new().type_(AxisType::Value).name(y_axis_label))
                .series(
                    Line::new()
                    .area_style(AreaStyle::new())
                    .data(data)
                );

            let mut renderer =
                Box::new(
                    ImageRenderer::new(1024, 512)
                    .theme(Theme::Walden));

            let svg_string = renderer.render(&chart)?;
            drop(renderer);

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

            let png = pixmap.encode_png()?;

            ctx.send(
                CreateReply::default()
                .attachment(CreateAttachment::bytes(png, "chart.png"))
            ).await?;
            msg.delete(ctx).await?;
        }
        None => {
            let act_on_user_ctx = ActOnUser(&ctx.data().db_pool, user.id);

            let balance = user_balance(&act_on_user_ctx).await;
            let place = user_place(&act_on_user_ctx, real_leaderboard_start_datetime()).await;
            let streak = user_streak(&act_on_user_ctx).await;

            ctx.send(CreateReply::default()
                .content(
                    MessageBuilder::new()
                    .user(ctx.author())
                    .push_line("")

                    .push(":ladder: ")
                    .push_line(
                        place
                        .map(|p| format!("**{}** - leaderboard place", p))
                            .unwrap_or("Not on leaderboard".to_string()))

                    .push(":wing: ")
                    .push_bold(streak.to_string())
                    .push_line(" day streak")

                    .push(":purse: ")
                    .push_bold(balance.to_string())
                    .push_line(" coins")

                    .build()
                )
                .allowed_mentions(CreateAllowedMentions::new())).await.unwrap();
        }
    }

    Ok(())
}
