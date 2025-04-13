use std::time::Duration;

use poise::serenity_prelude::UserId;
use rand::Rng;

use crate::{booster::Booster, product::Product, DbConn};

impl Product {
    pub fn random_reward() -> Self {
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

    pub async fn register_received_reward(self, conn: DbConn<'_>, uid: UserId, reason: String) -> anyhow::Result<i64> {
        let uid = i64::from(uid);
        let description = self.to_string();

        Ok(sqlx::query!("
        INSERT INTO rewards (user_id, description, reason)
        VALUES ((SELECT id FROM users WHERE uid = $1), $2, $3)
        ", uid, description, reason)
            .execute(&mut *conn)
            .await?
            .last_insert_rowid())
    }
}
