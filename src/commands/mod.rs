use crate::{Data, Error};

pub mod stats;
pub mod star;
pub mod simulate_study_session;
pub mod results;

type ApplicationContext<'a> = poise::ApplicationContext<'a, Data, Error>;
