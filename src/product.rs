use humantime::format_duration;
use num_format::{Locale, ToFormattedString};
use poise::serenity_prelude::{CacheHttp, Member, RoleId, UserId};
use serde::Deserialize;

use crate::{booster::Booster, charts::ChartTheme, coins::add_coins, DbConn};

#[derive(Deserialize)]
pub enum Product {
    Coins(u64),
    Booster(Booster),
    Role { name: String, role_id: RoleId },
    ChartTheme(ChartTheme)
}

impl ToString for Product {
    fn to_string(&self) -> String {
        match self {
            Self::Coins(amount) =>
                format!(
                    "{} coins",
                    amount.to_formatted_string(&Locale::en)),
            Self::Booster(Booster { multiplier, expiration }) =>
                format!(
                    "{}x booster (expires in {})",
                    *multiplier as f64 / 100.0,
                    format_duration(*expiration)),
            Self::Role { name, .. } => name.to_string(),
            Self::ChartTheme(theme) => format!("Chart theme {:?}", theme)
        }
    }
}

impl Product {
    pub async fn give_to_member(&self, conn: DbConn<'_>, http: impl CacheHttp, member: &Member) -> anyhow::Result<()> {
        match self {
            Self::Coins(amount) => add_coins(conn, member.user.id, *amount).await?,
            Self::Booster(Booster { multiplier, expiration }) => {
                let multiplier = *multiplier as i64;
                let expiration = expiration.as_secs() as i64;
                let uid = i64::from(member.user.id);
                sqlx::query!("
                INSERT INTO boosters
                VALUES (NULL, (SELECT id FROM users WHERE uid = $1), $2, UNIXEPOCH() + $3)
                ", uid, multiplier, expiration)
                    .execute(&mut *conn)
                    .await?;
            }
            Self::Role { role_id, .. } => member.add_role(http.http(), role_id).await?,
            Self::ChartTheme(theme) => {
                let theme = *theme as i64;
                let uid = i64::from(member.user.id);
                sqlx::query!("
                INSERT INTO owned_chart_themes
                VALUES (NULL, $1, (SELECT id FROM users WHERE uid = $2))
                ", theme, uid);
            }
        }

        Ok(())
    }
}
