use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct User {
    pub id: i64,
    pub uid: i64,
    pub coins: i32
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct MessageRef {
    pub channel_id: i64,
    pub message_id: i64
}
