use std::time::Duration;

use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
pub struct Booster {
    pub multiplier: u16,
    pub expiration: Duration
}
