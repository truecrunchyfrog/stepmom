use std::time::Duration;

use poise::serenity_prelude::{CacheHttp, ChannelId, Context, CreateMessage, EmbedMessageBuilding, Message, MessageBuilder, UserId};
use serde::Deserialize;
use tokio_cron_scheduler::Job;

use crate::Data;

#[derive(Deserialize)]
pub struct BumpingConfig {
    reminder_delay: Duration,
    bot_id: UserId,
    command_id: u64
}

fn is_bump_response(message: &Message, bumping_config: &BumpingConfig) -> bool {
    message.author.bot &&
        message.author.id == bumping_config.bot_id &&
        message.referenced_message.is_some()
}

pub async fn check_bump(ctx: &Context, message: &Message, data: &Data) -> anyhow::Result<bool> {
    if !is_bump_response(message, &data.config.bumping) {
        return Ok(false)
    }

    let bump_command_message =
        message.referenced_message.clone().expect("Bump message should reference other message.");
    let bumper = bump_command_message.author;

    // Delete the bump message.
    message.delete(&ctx.http).await?;

    // Delete last reminder.
    if let Some(last_bump_reminder) = data.last_bump_reminder.lock().await.as_ref() {
        last_bump_reminder.delete(&ctx.http).await?;
    }

    data.scheduler.add(bump_reminder_job(ctx, data)?).await?;

    let uid = i64::from(bumper.id);
    sqlx::query!("
    INSERT INTO bumps
    VALUES ($1, NULL)
    ", uid)
        .execute(&data.db_pool)
        .await?;

    ctx.http.send_message(
        message.channel_id,
        Vec::new(),
        &CreateMessage::new()
        .content(MessageBuilder::new()
            .push(":face_holding_back_tears: ")
            .push_named_link("THANK YOU", "https://cataas.com/cat/gif.gif")
            .push(" for your service, ")
            .mention(&bumper)
            .push_line("!")
            .build()
        )
    ).await?;

    Ok(true)
}

pub fn bump_reminder_job(ctx: &Context, data: &Data) -> anyhow::Result<Job> {
    let http = ctx.http.clone();
    let channel_id = data.config.channels.bump_reminder;
    let bump_command_id = data.config.bumping.command_id;
    let last_bump_reminder = data.last_bump_reminder.clone();

    Ok(Job::new_one_shot_async(data.config.bumping.reminder_delay, move |_, _| {
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
            ).await.expect("Cannot send bump reminder message.");

            *last_bump_reminder.lock().await = Some(reminder_msg);
        })
    })?)
}
