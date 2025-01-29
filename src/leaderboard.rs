use std::time::Duration;

use charming::{component::{Axis, Legend, Title}, element::AxisType, series::{Line, Scatter}, theme::Theme, Chart, ImageRenderer};
use poise::serenity_prelude::{ChannelId, Context, CreateMessage, MessageBuilder, UserId};
use sqlx::{types::time::{OffsetDateTime, Time}, SqlitePool};
use time::Date;
use tokio_cron_scheduler::Job;

use crate::{charts::render_chart_to_attachment, prelude::ActOnUser, rewards::{user_claim_reward, Reward}, Data};

pub fn top_leaderboard_rewards() -> [Vec<Reward>; 3] {
    let expiration = Duration::from_secs(30 * 24 * 60 * 60);
    [
        vec![ // First place
            Reward::Coins(10000),
            Reward::Booster { multiplier: 800, expiration }
        ],
        vec![ // Second place
            Reward::Coins(4000),
            Reward::Booster { multiplier: 600, expiration }
        ],
        vec![ // Third place
            Reward::Coins(2000),
            Reward::Booster { multiplier: 400, expiration }
        ]
    ]
}

pub fn real_leaderboard_start_datetime() -> OffsetDateTime {
    let today = OffsetDateTime::now_utc().date();

    OffsetDateTime::new_utc(
        Date::from_calendar_date(today.year(), today.month(), 1).unwrap(),
        Time::MIDNIGHT
    )
}

pub async fn user_place(ctx: &ActOnUser<'_>, after: OffsetDateTime) -> Option<u16> {
    let uid = ctx.uid();

    sqlx::query!("
    SELECT
        ROW_NUMBER() OVER (ORDER BY SUM(length) DESC) AS place
    FROM users
    JOIN study_sessions ON users.id = study_sessions.user_id
    LEFT JOIN leaderboard_optout ON users.id = leaderboard_optout.user_id
    WHERE leaderboard_optout.user_id IS NULL AND ended > $1
    GROUP BY users.id
    HAVING users.id IN (SELECT id FROM users WHERE uid = $2)
    ", after, uid)
        .fetch_optional(ctx.0)
        .await.unwrap()
        .map(|r| r.place as u16)
}

pub async fn fetch_leaderboard(pool: &SqlitePool, after: OffsetDateTime, limit: Option<i16>) -> Vec<(UserId, Duration)> {
    let limit = limit.unwrap_or(-1);

    sqlx::query!("
    SELECT
        users.uid AS uid,
        SUM(length) AS study_amount
    FROM users
    JOIN study_sessions ON users.id = study_sessions.user_id
    LEFT JOIN leaderboard_optout AS optout ON users.id = optout.user_id
    WHERE optout.user_id IS NULL AND ended > $1
    GROUP BY users.id
    HAVING SUM(length) IS NOT NULL
    ORDER BY SUM(length) DESC
    LIMIT $2
    ", after, limit)
        .fetch_all(pool)
        .await.unwrap()
        .iter()
        .map(|r| (
                UserId::new(r.uid as u64),
                Duration::from_secs(r.study_amount as u64)
        ))
        .collect()
}

pub fn leaderboard_new_month_job(ctx: &Context, data: &Data) -> Job {
    let db = data.db_pool.clone();
    let http = ctx.http.clone();
    let channel_id = ChannelId::new(data.config.channels.leaderboard_announcement_channel);

    //Job::new_async("0 0 0 1 * *", move |_, _| {
    Job::new_async("0 * * * * *", move |_, _| {
        let db = db.clone();
        let http = http.clone();

        Box::pin(async move {
            // TODO Ping Newsfeed role

            let month_start = real_leaderboard_start_datetime();
            let leaderboard = fetch_leaderboard(&db, month_start, Some(10)).await;

            let top_with_rewards =
                leaderboard
                .iter()
                .zip(top_leaderboard_rewards())
                .collect::<Vec<_>>();

            for ((uid, _), rewards) in top_with_rewards.iter() {
                for reward in rewards {
                    user_claim_reward(
                        &ActOnUser(&db, *uid),
                        *reward,
                        "Monthly challenge reward".to_string()
                    ).await;
                }
            }

            let gift_to_users = sqlx::query!("
            SELECT uid
            FROM users
            JOIN study_sessions
                ON users.id = study_sessions.user_id
            WHERE ended > $1
            GROUP BY users.id
            HAVING COALESCE(SUM(length), 0) > 10 * 60 * 60
            ", month_start)
                .fetch_all(&db)
                .await.unwrap()
                .iter()
                .map(|r| UserId::new(r.uid as u64))
                .collect::<Vec<_>>();

            let community_gift =
                    Reward::Booster {
                        multiplier: 150,
                        expiration: Duration::from_secs(30 * 24 * 60 * 60)
                    };

            for uid in gift_to_users {
                user_claim_reward(
                    &ActOnUser(&db, uid),
                    community_gift,
                    "Monthly Challenge Community Gift".to_string()
                ).await;
            }

            // TODO this is not ideal, very hacky
            let dates = sqlx::query!("
            WITH RECURSIVE date_range AS (
                SELECT DATE($1) AS date
                UNION ALL
                SELECT DATE(date, '+1 day')
                FROM date_range
                WHERE DATE(date, '+1 day') <= DATE('now')
            ) SELECT * FROM date_range
            ", month_start)
                .fetch_all(&db)
                .await.unwrap();

            let mut top_ten_chart = Chart::new()
                .title(Title::new().text("Progression history of the top 10 placed frogs"))
                .x_axis(Axis::new().type_(AxisType::Category)
                    .name("Time")
                    .data(
                        dates
                        .iter()
                        .flat_map(|r| &r.date)
                        .collect()))
                .y_axis(Axis::new().type_(AxisType::Value).name("Total hours"))
                .legend(Legend::new().top("bottom"));

            for (uid, _) in &leaderboard {
                let user_progress = {
                    let uid = i64::from(uid.clone());
                    sqlx::query!("
                    WITH RECURSIVE date_range AS (
                        SELECT DATE($1) AS date
                        UNION ALL
                        SELECT DATE(date, '+1 day')
                        FROM date_range
                        WHERE DATE(date, '+1 day') <= DATE('now')
                    ),
                    sessions_with_dates AS (
                        SELECT id, user_id, length, date
                        FROM date_range
                        LEFT JOIN study_sessions
                            ON date = DATE(ended)
                    )
                    SELECT date, COALESCE(SUM(length) OVER (ORDER BY date), 0) AS cumulative_time
                    FROM sessions_with_dates
                    LEFT JOIN users
                        ON user_id = users.id
                    WHERE uid IS NULL OR uid = $2
                    GROUP BY user_id, date
                    ORDER BY date
                    ", month_start, uid)
                        .fetch_all(&db)
                        .await.unwrap()
                        .iter()
                        .map(|r| r.cumulative_time as f64 / 3600.0)
                        .collect()
                };

                top_ten_chart = top_ten_chart.series(
                    Line::new()
                    .name(uid.to_user(&http).await.map_or("[unknown user]".to_string(), |u| u.name))
                    .data(user_progress)
                );
            }

            let charts = [
                top_ten_chart
            ];

            let attachments = charts
                .iter()
                .flat_map(|c| render_chart_to_attachment(
                    &mut ImageRenderer::new(1024, 512)
                    .theme(Theme::Walden),
                    c
                ))
                .collect::<Vec<_>>();

            let message = CreateMessage::new()
                .content({
                    let mut b = MessageBuilder::new();
                    b.push_line("## Monthly Pond Challenge");
                    b.push_line("[insert month name] is over!");
                    b.push_line("That means that we have some frogs to celebrate, interesting data to show, and a new month to look forward to.");
                    b.push_line("Let's hop right into it...");

                    b.push_line("## This Month in Numbers");

                    let community_study_data = sqlx::query!("
                    SELECT
                        COUNT(DISTINCT user_id) AS user_count,
                        COALESCE(SUM(length), 0) AS time,
                        COUNT(*) AS session_count,
                        COALESCE(AVG(length), 0) AS avg_time
                    FROM study_sessions
                    WHERE ended > $1
                    ", month_start)
                        .fetch_one(&db)
                        .await.unwrap();

                    b.push("* We were a total of ");
                    b.push_bold(community_study_data.user_count.to_string());
                    b.push_line(" frogs studying this month.");

                    b.push("* Together we studied a total of ");
                    b.push_bold(
                        humantime::format_duration(Duration::from_secs(
                            community_study_data.time as u64
                        )).to_string()
                    );
                    b.push_line(".");

                    b.push("* We had ");
                    b.push_bold(community_study_data.session_count.to_string());
                    b.push_line(" sessions.");

                    b.push("* That makes the average session ");
                    b.push_bold(
                        humantime::format_duration(Duration::from_secs(
                            community_study_data.avg_time as u64
                        )).to_string()
                    );
                    b.push_line(" long.");

                    b.push_line("## The Top Frogs");
                    b.push_line("Reveal each \"spoiler\" (||this is a spoiler||) by clicking them, in order from top to bottom.");

                    let top_members = top_with_rewards
                        .iter()
                        .take(3)
                        .enumerate()
                        .zip([
                            ":first_place:",
                            ":second_place:",
                            ":third_place:"
                        ])
                        .rev();

                    for ((place, ((uid, study_time), rewards)), emoji) in top_members {
                        b.push("#".repeat(place + 1));
                        b.push(" ");
                        b.push(emoji);
                        b.push(" ||");
                        b.user(uid);
                        b.push_line("||");

                        b.push("* ||Studied ");
                        b.push_bold(humantime::format_duration(*study_time).to_string());
                        b.push_line("||");

                        b.push_line(format!(
                                "* ||Rewards: {}||",
                                rewards
                                .iter()
                                .map(|r| format!("**{}**", r.to_string()))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }

                    b.push_line("## Dear Frogs");
                    b.push_line("Congratulations, and thank you, to everyone who participated in The Pond this month! We're proud of you all, and we love your effort!");
                    b.push_line("As such, we want to give you something.");

                    b.push("Everyone who gave more than ");
                    b.push_bold("10 hours");
                    b.push(" of study effort this month have received a ");
                    b.push_bold(community_gift.to_string());
                    b.push_line(" that lasts a month.");

                    b.push_line("## Chart");

                    b.build()
                });

            http.send_message(
                channel_id,
                attachments,
                &message
            ).await.unwrap();
        })
    }).unwrap()
}
