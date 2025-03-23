use std::fmt;

use poise::serenity_prelude::{User, UserId};

use crate::DbConn;

pub struct InsufficientFundsError {
    /// None if this is the "You", otherwise provide Some user.
    pub second_user: Option<String>,
    /// The user's current balance.
    pub balance: u64,

    /// The name of the product.
    pub product: String,
    /// The cost of the product.
    pub cost: u64
}

impl std::error::Error for InsufficientFundsError {}

impl fmt::Display for InsufficientFundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "{}n't have enough coins to pay for {}. Cost: **{}**\nBalance: **{}** (need **{}** more!)",
            self.second_user.as_deref().map_or("You do".to_string(), |u| format!("{} does", u)),
            self.product,
            self.cost,
            self.balance,
            self.cost - self.balance)
    }
}

impl fmt::Debug for InsufficientFundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Insufficient funds while trying to pay for `{}`. Have {}. Need {}.", self.product, self.balance, self.cost)
    }
}

pub async fn user_balance(conn: DbConn<'_>, uid: UserId) -> anyhow::Result<u64> {
    let uid = i64::from(uid);
    Ok(sqlx::query!("
    SELECT COALESCE(SUM(coins_diff), 0) AS balance FROM users
    JOIN coin_transactions ON users.id = coin_transactions.user_id
    WHERE uid = $1
    ", uid)
        .fetch_one(conn)
        .await?
        .balance as u64)
}

pub async fn coin_transaction(conn: DbConn<'_>, uid: UserId, balance_diff: i64) -> anyhow::Result<()> {
    let uid = i64::from(uid);

    match sqlx::query!("
    INSERT INTO coin_transactions (user_id, coins_diff)
    SELECT users.id, $2 FROM users
    JOIN coin_transactions t ON users.id = t.user_id
    GROUP BY users.id
    HAVING uid = $1 AND COALESCE(SUM(coins_diff), 0) + $2 >= 0
    ", uid, balance_diff)
        .execute(conn)
        .await?
        .rows_affected() {
            0 => Err(anyhow::anyhow!("Cannot create coin transaction.")),
            _ => Ok(())
        }
}

pub async fn add_coins(conn: DbConn<'_>, uid: UserId, coins: u64) -> anyhow::Result<()> {
    coin_transaction(conn, uid, coins as i64).await
}

pub async fn sub_coins(conn: DbConn<'_>, uid: UserId, coins: u64) -> anyhow::Result<()> {
    coin_transaction(conn, uid, -(coins as i64)).await
}

pub async fn take_coins<'a>(
    conn: DbConn<'_>,
    uid: UserId,
    cost: u64,
    product: String,
    second_user: Option<&'a User>
) -> anyhow::Result<()> {
    match sub_coins(conn, uid, cost).await {
        Err(_) => Err(InsufficientFundsError {
            second_user: second_user.map(|u| u.display_name().to_string()),
            balance: user_balance(conn, uid).await?,
            product,
            cost
        }.into()),
        otherwise => otherwise
    }
}
