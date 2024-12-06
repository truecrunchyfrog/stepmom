use std::time::Duration;

use poise::serenity_prelude::{CacheHttp, ChannelId, Context, CreateMessage, Http, Message, UserId};
use tokio_cron_scheduler::Job;

use crate::{Config, Data};

pub async fn check_bump(ctx: &Context, message: &Message, data: &Data) {
    if !(message.author.bot && message.author.id == UserId::new(data.config.bump_bot_id)) {
        return
    }

    let replied_to = message.referenced_message.clone().unwrap();
    message.delete(ctx.http()).await.unwrap();

    data.scheduler.add(bump_reminder_job(ctx, data));

    println!("{}", replied_to.author);
}

pub fn bump_reminder_job(ctx: &Context, data: &Data) -> Job {
    let http = ctx.http.clone();
    let channel_id = ChannelId::new(data.config.channels.bump_reminder_channel);

    Job::new_one_shot_async(Duration::from_secs(data.config.bump_reminder_delay), move |_, _| {
        let http = http.clone();

        Box::pin(async move {
            http.send_message(
                channel_id,
                Vec::new(),
                &CreateMessage::new()
                .content("Bump reminder goes here")
            ).await.unwrap();
            println!("TODO: send bump reminder here...");
        })
    }).unwrap()
}
