use poise::serenity_prelude::UserId;
use sqlx::{types::time::{OffsetDateTime, Time}, SqlitePool};
use time::Date;

use crate::prelude::ActOnUser;

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
        .await
        .unwrap()
        .map(|r| r.place as u16)
}

pub async fn fetch_leaderboard(pool: &SqlitePool, after: OffsetDateTime, limit: Option<i16>) -> Vec<(UserId, u64)> {
    let limit = limit.unwrap_or(-1);

    sqlx::query!("
    SELECT
        users.id AS uid,
        SUM(length) AS study_amount
    FROM users
    JOIN study_sessions ON users.id = study_sessions.user_id
    LEFT JOIN leaderboard_optout ON users.id = leaderboard_optout.user_id
    WHERE leaderboard_optout.user_id IS NULL AND ended > $1
    GROUP BY users.id
    HAVING SUM(length) IS NOT NULL
    ORDER BY SUM(length) DESC
    LIMIT $2
    ", after, limit)
        .fetch_all(pool)
        .await
        .unwrap()
        .iter()
        .map(|r| (UserId::new(r.uid as u64), r.study_amount as u64))
        .collect()
}
