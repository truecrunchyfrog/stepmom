use poise::serenity_prelude::UserId;

use crate::DbConn;

pub async fn create_user(conn: DbConn<'_>, uid: UserId) -> anyhow::Result<()> {
    let uid = i64::from(uid);
    sqlx::query!("INSERT OR IGNORE INTO users (uid) VALUES ($1)", uid)
        .execute(conn)
        .await?;
    Ok(())
}
