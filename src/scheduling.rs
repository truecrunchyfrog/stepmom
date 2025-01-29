use poise::serenity_prelude::Context;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

use crate::{bumping, leaderboard, Data};

pub async fn create_scheduler() -> Result<JobScheduler, JobSchedulerError> {
    let sched = JobScheduler::new().await?;
    sched.start().await?;
    Ok(sched)
}

pub async fn add_scheduler_items(ctx: &Context, data: &Data) -> Result<(), JobSchedulerError> {
    data.scheduler.add(leaderboard::leaderboard_new_month_job(ctx, data)).await?;
    data.scheduler.add(bumping::bump_reminder_job(ctx, data)).await?;

    Ok(())
}
