use std::time::Duration;

#[derive(Clone, Copy)]
pub struct Booster {
    pub multiplier: u16,
    pub expiration: Duration
}
