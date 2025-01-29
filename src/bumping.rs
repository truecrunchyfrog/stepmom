use std::time::Duration;

use poise::serenity_prelude::{CacheHttp, ChannelId, Context, CreateMessage, EmbedMessageBuilding, Message, MessageBuilder, UserId};
use tokio_cron_scheduler::Job;

use crate::Data;

pub async fn check_bump(ctx: &Context, message: &Message, data: &Data) {
    if !(message.author.bot && message.author.id == UserId::new(data.config.bump_bot_id)) {
        return
    }

    let replied_to = message.referenced_message.clone().unwrap();
    let bumper = replied_to.author;

    message.delete(&ctx.http).await.unwrap();
    if let Some(message) = data.last_bump_reminder.lock().await.as_ref() {
        message.delete(&ctx.http).await.unwrap();
    }

    data.scheduler.add(bump_reminder_job(ctx, data)).await.unwrap();

    let uid = i64::from(bumper.id);
    sqlx::query!("
    INSERT INTO bumps
    VALUES ($1, NULL)
    ", uid)
        .execute(&data.db_pool)
        .await.unwrap();

    ctx.http().send_message(
        message.channel_id,
        Vec::new(),
        &CreateMessage::new()
        .content(MessageBuilder::new()
            .push(":face_holding_back_tears: ")
            .push_named_link(
                "THANK YOU",
                "https://cataas.com/cat/gif.gif"
            )
            .push(" for your service, ")
            .mention(&bumper)
            .push_line("!")
            .build()
        )
    ).await.unwrap();
}

pub fn bump_reminder_job(ctx: &Context, data: &Data) -> Job {
    let http = ctx.http.clone();
    let channel_id = ChannelId::new(data.config.channels.bump_reminder_channel);
    let bump_command_id = data.config.bump_command_id;
    let last_bump_reminder = data.last_bump_reminder.clone();

    Job::new_one_shot_async(Duration::from_secs(data.config.bump_reminder_delay), move |_, _| {
        let http = http.clone();
        let last_bump_reminder = last_bump_reminder.clone();

        Box::pin(async move {
            let reminder_msg = http.send_message(
                channel_id,
                Vec::new(),
                &CreateMessage::new()
                .content(format!(
                        "-# :beaver: Hey, listen! Help the pond grow:\n## </bump:{}>",
                        bump_command_id
                ))
            ).await.unwrap();

            *last_bump_reminder.lock().await = Some(reminder_msg);
        })
    }).unwrap()
}
