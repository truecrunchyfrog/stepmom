use std::time::Duration;

use humantime::format_duration;
use num_format::{Locale, ToFormattedString};
use poise::serenity_prelude::{Mentionable, RoleId, UserId};
use rand::Rng;

use crate::{booster::Booster, coins::add_coins, DbConn};

#[derive(Clone, Copy)]
pub enum Reward {
    Coins(u64),
    Booster(Booster),
    Role(RoleId)
}

impl Reward {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..100) {
            0..70 => Self::Coins(rng.gen_range(1..8) * 100),
            70..99 => Self::Booster(Booster {
                multiplier: rng.gen_range(15..25) * 10,
                expiration: Duration::from_secs(rng.gen_range(1..24 * 8) * 60 * 60)
            }),
            _ => unreachable!()
        }
    }
}

impl ToString for Reward {
    fn to_string(&self) -> String {
        match self {
            Reward::Coins(amount) =>
                format!(
                    "{} coins",
                    amount.to_formatted_string(&Locale::en)),
            Reward::Booster(Booster { multiplier, expiration }) =>
                format!(
                    "{}x booster (expires in {})",
                    *multiplier as f64 / 100.0,
                    format_duration(*expiration)),
            Reward::Role(role_id) =>
                role_id.mention().to_string()
        }
    }
}

pub async fn user_claim_reward(conn: DbConn<'_>, uid: UserId, reward: Reward, reason: String) -> Result<i64> {
    match reward {
        Reward::Coins(amount) => {
            add_coins(conn, uid, amount).await;
        }
        Reward::Booster(Booster { multiplier, expiration }) => {
            let multiplier = multiplier as i64;
            let expiration = expiration.as_secs() as i64;
            let uid = i64::from(uid);
            sqlx::query!("
            INSERT INTO boosters
            VALUES (NULL, (SELECT id FROM users WHERE uid = $1), $2, UNIXEPOCH() + $3)
            ", uid, multiplier, expiration)
                .execute(conn)
                .await?;
        }
        Reward::Role(role_id) => {
            // TODO
            unimplemented!()
        }
    }

    let description = reward.to_string();
    let uid = i64::from(uid);

    Ok(sqlx::query!("
    INSERT INTO rewards (user_id, description, reason)
    VALUES ((SELECT id FROM users WHERE uid = $1), $2, $3)
    ", uid, description, reason)
        .execute(conn)
        .await?
        .last_insert_rowid())
}
