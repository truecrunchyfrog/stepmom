use std::time::{SystemTime, UNIX_EPOCH};

use humantime::format_duration;
use poise::{serenity_prelude::{CreateAllowedMentions, MessageBuilder}, CreateReply};
use tokio::time::Duration;

use crate::{Context, Error};

/// See realtime, studying information.
#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn session(
    ctx: Context<'_>
) -> Result<(), Error> {
    let study_states = ctx.data().study_states.lock().await;

    fn instant_to_timestamp(i: tokio::time::Instant) -> u64 {
        (SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
         - i.elapsed()).as_secs()
    }

    let own_study_info = match study_states.get(&ctx.author().id) {
        Some(own_state) => {
            let mut b = MessageBuilder::new();
            b.push_line(format!(
                    "Session started <t:{}:R>",
                    instant_to_timestamp(own_state.start)));
            b.push_line(format!(
                    "Video streamed for **{}**",
                    format_duration(
                        Duration::from_secs((
                                *own_state.video_sum.lock().await +
                                own_state.video_start.lock().await.map_or(Duration::ZERO, |i| i.elapsed())
                        ).as_secs())
                    ).to_string()
            ));
            b
        }
        None => MessageBuilder::new()
            .push_line("You are not currently studying. Hop into a study room to start!")
            .to_owned()
    }.build();

    let other_study_info = {
        let mut b = MessageBuilder::new();

        for state in study_states.iter().filter(|s| s.0 != &ctx.author().id) {
            b.user(state.0);
            b.push_bold_line(format!(
                    " <t:{}:R>",
                    instant_to_timestamp(state.1.start)
            ));
        }

        b
    }.build();

    ctx.send(CreateReply::default()
        .content(
            format!(
                "## You\n{}\n## Others\n{}",
                own_study_info,
                other_study_info))
        .allowed_mentions(CreateAllowedMentions::new()))
        .await?;

    Ok(())
}
