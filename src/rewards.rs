use std::time::Duration;

use humantime::format_duration;
use num_format::{Locale, ToFormattedString};
use poise::serenity_prelude::{Mentionable, RoleId};
use rand::Rng;

use crate::prelude::{add_coins, ActOnUser};

#[derive(Clone, Copy)]
pub enum Reward {
    Coins(u64),
    Booster { multiplier: u16, expiration: Duration },
    Role(RoleId)
}

impl Reward {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..100) {
            0..70 => Self::Coins(rng.gen_range(1..8) * 100),
            70..99 => Self::Booster {
                multiplier: rng.gen_range(15..25) * 10,
                expiration: Duration::from_secs(rng.gen_range(1..24 * 8) * 60 * 60)
            },
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
            Reward::Booster { multiplier, expiration } =>
                format!(
                    "{}x booster (expires in {})",
                    *multiplier as f64 / 100.0,
                    format_duration(*expiration)),
            Reward::Role(role_id) =>
                role_id.mention().to_string()
        }
    }
}

pub async fn user_claim_reward(ctx: &ActOnUser<'_>, reward: Reward, reason: String) -> i64 {
    let uid = ctx.uid();
    match reward {
        Reward::Coins(amount) => {
            add_coins(ctx, amount).await;
        }
        Reward::Booster { multiplier, expiration } => {
            let multiplier = multiplier as i64;
            let expiration = expiration.as_secs() as i64;
            sqlx::query!("
            INSERT INTO boosters
            VALUES (NULL, (SELECT id FROM users WHERE uid = $1), $2, UNIXEPOCH() + $3)
            ", uid, multiplier, expiration)
                .execute(ctx.0)
                .await.unwrap();
        }
        Reward::Role(role_id) => {

        }
    }

    let description = reward.to_string();

    sqlx::query!("
    INSERT INTO rewards (user_id, description, reason)
    VALUES ((SELECT id FROM users WHERE uid = $1), $2, $3)
    ", uid, description, reason)
        .execute(ctx.0)
        .await.unwrap()
        .last_insert_rowid()
}
