use core::fmt;
use std::error::Error;

use sqlx::SqlitePool;
use poise::serenity_prelude::{self as serenity, CacheHttp, ChannelId, Context, CreateButton, CreateMessage, Mentionable, Message, User, UserId};

use crate::Data;

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
    pub fn uid(&self) -> i64 {
        i64::from(self.1)
    }
}

pub async fn user_balance<'a>(ctx: &ActOnUser<'a>) -> u64 {
    let uid = ctx.uid();

    sqlx::query!("
    SELECT COALESCE(SUM(coins_diff), 0) AS balance FROM users
    JOIN coin_transactions ON users.id = coin_transactions.user_id
    WHERE uid = $1
    ", uid)
        .fetch_one(ctx.0)
        .await
        .unwrap()
        .balance as u64
}

pub async fn coin_transaction<'a>(ctx: &ActOnUser<'a>, balance_diff: i64) -> bool {
    let uid = ctx.uid();

    sqlx::query!("
    INSERT INTO coin_transactions (user_id, coins_diff)
    SELECT users.id, $2 FROM users
    JOIN coin_transactions t ON users.id = t.user_id
    GROUP BY users.id
    HAVING uid = $1 AND COALESCE(SUM(coins_diff), 0) + $2 >= 0
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
        let balance = user_balance(ctx).await;

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

pub async fn create_user(ctx: &ActOnUser<'_>) {
    let user_id = ctx.uid();
    sqlx::query!("INSERT OR IGNORE INTO users (uid) VALUES ($1)", user_id)
        .execute(ctx.0)
        .await
        .unwrap();
}

pub async fn try_dm_or_in_guild(ctx: &Context, data: &Data, cache_http: impl CacheHttp, user: &User, builder: CreateMessage) -> Message {
    let dm_message = user.dm(&cache_http, builder.clone()).await;

    match dm_message {
        Ok(sent_dm_msg) => sent_dm_msg,
        Err(_) => {
            let msg_set_id = sqlx::query!("INSERT INTO msg_sets VALUES (NULL)")
                .execute(&data.db_pool)
                .await.unwrap()
                .last_insert_rowid();

            let user_id = i64::from(user.id);

            sqlx::query!("
            INSERT INTO guild_sent_dm_messages
            VALUES ((SELECT id FROM users WHERE uid = $1), $2)
                ", user_id, msg_set_id)
                .execute(&data.db_pool)
                .await.unwrap()
                .last_insert_rowid();

            let channel = &cache_http.http().get_channel(
                ChannelId::new(data.config.channels.dm_backup_channel))
                .await.unwrap()
                .guild().unwrap();

            let sent_guild_msg = channel
                .send_message(&cache_http, builder)
                .await.unwrap();

            let info_msg = channel.send_message(&cache_http, CreateMessage::new()
                .content(format!("{}\n-# We tried to send this message straight to you, but it could not be delivered!\n-# Enable **Direct Messages** under **Privacy Settings** to get these directly to you next time, privately.\n-# * This message is public, click `Delete` to delete it.", user.mention()))
                .button(
                    CreateButton::new("delete_guild_dm")
                    .label("Delete")
                    .style(serenity::ButtonStyle::Danger)
                )
                .reference_message(&sent_guild_msg)
            ).await.unwrap();

            for msg in vec![&sent_guild_msg, &info_msg] {
                let msg_ref_id = create_message_ref(&data.db_pool, &msg).await;
                sqlx::query!("
                INSERT INTO msg_set_items
                VALUES ($1, $2)
                ", msg_set_id, msg_ref_id)
                    .execute(&data.db_pool)
                    .await.unwrap();
            }

            sent_guild_msg
        }
    }
}
