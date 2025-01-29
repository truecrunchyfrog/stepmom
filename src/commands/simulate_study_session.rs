use std::time::Duration;

use poise::serenity_prelude::User;
use tokio::time::Instant;

use crate::{study::{finish_session, StudyState}, Context, Error};

/// Simulate a study session on a user.
#[poise::command(slash_command, prefix_command, required_permissions = "ADMINISTRATOR", ephemeral = true)]
pub async fn simulate_study_session(
    ctx: Context<'_>,
    #[description = "User to simulate the study session on"]
    user: User,
    #[description = "Total study length"]
    length: String,
    #[description = "Length studied with video"]
    video_length: Option<String>,
    #[description = "Alert the user with the result"]
    alert: bool
) -> Result<(), Error> {
    let length = humantime::parse_duration(&length)?;
    let video_length = video_length.map(|s| humantime::parse_duration(&s)).transpose()?.unwrap_or(Duration::ZERO);

    if video_length > length {
        return Err(Error::from("Total length must be greater than or equals to the video length."))
    }

    let study_state = StudyState {
        start: Instant::now() - length,
        video_start: None.into(),
        video_sum: video_length.into(),
        break_start: None.into(),
        break_sum: Duration::ZERO.into()
    };

    finish_session(
        ctx.serenity_context(),
        ctx.data(),
        user.id,
        study_state,
        alert
    ).await;

    let _ = ctx.reply("Session simulated.").await;

    Ok(())
}
