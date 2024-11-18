use core::fmt;
use std::error::Error;

use sqlx::SqlitePool;
use poise::serenity_prelude::{self as serenity, UserId};

pub async fn create_message_ref(pool: &SqlitePool, message: &serenity::Message) -> i64
{
    let cid = i64::from(message.channel_id);
    let mid = i64::from(message.id);
    sqlx::query!("
            INSERT INTO message_refs (channel_id, message_id)
            VALUES ($1, $2)",
            cid, mid)
        .execute(pool)
        .await
        .unwrap()
        .last_insert_rowid()
}

pub struct ActOnUser<'a>(pub &'a SqlitePool, pub UserId);

impl ActOnUser<'_> {
    fn uid(&self) -> i64 {
        i64::from(self.1)
    }
}

pub async fn get_balance<'a>(ctx: &ActOnUser<'a>) -> u64 {
    let uid = ctx.uid();

    sqlx::query!(
        "SELECT coins FROM users WHERE uid = $1",
        uid
    )
        .fetch_one(ctx.0)
        .await
        .unwrap()
        .coins as u64
}

pub async fn coin_transaction<'a>(ctx: &ActOnUser<'a>, balance_diff: i64) -> bool {
    let uid = ctx.uid();

    sqlx::query!("
    UPDATE users
    SET coins = coins + $2
    WHERE uid = $1 AND coins + $2 >= 0
    ", uid, balance_diff)
        .execute(ctx.0)
        .await
        .unwrap()
        .rows_affected() != 0
}

pub async fn add_coins(ctx: &ActOnUser<'_>, coins: u64) -> bool {
    coin_transaction(ctx, coins as i64).await
}

pub async fn sub_coins(ctx: &ActOnUser<'_>, coins: u64) -> bool {
    coin_transaction(ctx, -(coins as i64)).await
}

pub async fn take_coins<'a>(ctx: &ActOnUser<'_>, cost: u64, product: &'static str, third_user: Option<&'a serenity::User>) -> Result<(), InsufficientFundsError<'a>> {
    if sub_coins(ctx, cost).await {
        Ok(())
    } else {
        let balance = get_balance(ctx).await;

        Err(InsufficientFundsError {
            third_user,
            balance,
            product,
            cost
        })
    }
}

pub struct InsufficientFundsError<'a> {
    /// None if this is the "You", otherwise provide Some user.
    pub third_user: Option<&'a serenity::User>,
    /// The user's current balance.
    pub balance: u64,

    /// The name of the product.
    pub product: &'static str,
    /// The cost of the product.
    pub cost: u64,
}

impl Error for InsufficientFundsError<'_> {}

impl fmt::Display for InsufficientFundsError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{}n't have enough coins to pay for {}. Cost: **{}**\nBalance: **{}** (need **{}** more!)",
            self.third_user.map_or("You do".to_string(), |u| format!("{} does", u)),
            self.product,
            self.cost,
            self.balance,
            self.cost - self.balance)
    }
}

impl fmt::Debug for InsufficientFundsError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Insufficient funds while trying to pay for `{}`. Have {}. Need {}.", self.product, self.balance, self.cost)
    }
}
