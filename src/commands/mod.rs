use crate::{Data, Error};

pub mod stats;
pub mod star;
pub mod simulate_study_session;
pub mod results;
pub mod leaderboard;

type ApplicationContext<'a> = poise::ApplicationContext<'a, Data, Error>;
