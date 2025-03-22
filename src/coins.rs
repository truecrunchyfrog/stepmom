use std::fmt;

use poise::serenity_prelude::{User, UserId};
use sqlx::SqliteConnection;

use crate::{Result, DbConn};

pub struct InsufficientFundsError<'a> {
    /// None if this is the "You", otherwise provide Some user.
    pub second_user: Option<&'a User>,
    /// The user's current balance.
    pub balance: u64,

    /// The name of the product.
    pub product: &'static str,
    /// The cost of the product.
    pub cost: u64,
}

impl std::error::Error for InsufficientFundsError<'_> {}

impl fmt::Display for InsufficientFundsError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "{}n't have enough coins to pay for {}. Cost: **{}**\nBalance: **{}** (need **{}** more!)",
            self.second_user.map_or("You do".to_string(), |u| format!("{} does", u)),
            self.product,
            self.cost,
            self.balance,
            self.cost - self.balance)
    }
}

impl fmt::Debug for InsufficientFundsError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Insufficient funds while trying to pay for `{}`. Have {}. Need {}.", self.product, self.balance, self.cost)
    }
}

#[derive(Debug, Clone)]
pub struct AnonymousInsufficientFundsError;

impl std::error::Error for AnonymousInsufficientFundsError {}

impl fmt::Display for AnonymousInsufficientFundsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Insufficient funds!")
    }
}

pub async fn user_balance(conn: DbConn<'_>, uid: UserId) -> u64 {
    let uid = i64::from(uid);
    sqlx::query!("
    SELECT COALESCE(SUM(coins_diff), 0) AS balance FROM users
    JOIN coin_transactions ON users.id = coin_transactions.user_id
    WHERE uid = $1
    ", uid)
        .fetch_one(conn)
        .await
        .unwrap()
        .balance as u64
}

pub async fn coin_transaction(conn: DbConn<'_>, uid: UserId, balance_diff: i64) -> Result<()> {
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
            0 => Err(Box::new(AnonymousInsufficientFundsError)),
            _ => Ok(())
        }
}

pub async fn add_coins(conn: DbConn<'_>, uid: UserId, coins: u64) -> Result<()> {
    coin_transaction(conn, uid, coins as i64).await
}

pub async fn sub_coins(conn: DbConn<'_>, uid: UserId, coins: u64) -> Result<()> {
    coin_transaction(conn, uid, -(coins as i64)).await
}

pub async fn take_coins<'a>(conn: DbConn<'_>, uid: UserId, cost: u64, product: &'static str, second_user: Option<&'a User>) -> Result<()> {
    match sub_coins(conn, uid, cost).await {
        Err(AnonymousInsufficientFundsError) => Err(Box::new(InsufficientFundsError {
            second_user,
            balance: user_balance(conn, uid).await,
            product,
            cost
        })),
        otherwise => otherwise
    }
}
