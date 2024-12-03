use std::time::Duration;

use humantime::{format_duration, parse_duration};
use log::info;
use poise::{serenity_prelude::{CacheHttp, ComponentInteraction, Context, EditMessage, UserId}, Modal};
use poise::modal;

use crate::{Data, Error};

#[derive(poise::Modal)]
#[name = "Session time deduction"]
struct DeductionModal {
    /// Humantime duration string representing the session length to keep.
    /// Less than or equals to current length.
    /// If zero, session is deleted.
    #[name = "Total time (leave empty to delete session)"]
    #[placeholder = "New total session time (e.g. \"1h 2m 3s\")"]
    #[max_length = 20]
    keep_length: Option<String>,
    /// Humantime duration string representing the session video length to keep.
    /// Less than or equals to current video length AND less than or equals to new total length.
    /// If zero, video length will be set to zero.
    #[name = "Video time"]
    #[placeholder = "New video session time (e.g. \"1h 2m 3s\")"]
    #[max_length = 20]
    keep_video_length: Option<String>
}

pub async fn deduct_session(ctx: &Context, interaction: &ComponentInteraction, data: &Data, session_id: i64) -> Result<(), Error> {
    info!("Session penalty deduction on session {} for user {}", session_id, interaction.user);

    let uid = i64::from(interaction.user.id);
    let session_data = sqlx::query!("
    SELECT length, video_length, uid FROM study_sessions
    JOIN users ON user_id = users.id
    WHERE
        study_sessions.id = $1 AND
        uid = $2
    ", session_id, uid)
        .fetch_optional(&data.db_pool)
        .await?
        .ok_or(Error::from("Cannot find that study session."))?;

    let old_length = Duration::from_secs(session_data.length as u64);
    let old_video_length = Duration::from_secs(session_data.video_length as u64);

    let deduction = modal::execute_modal_on_component_interaction(
        Box::new(ctx.clone()),
        interaction.clone(),
        Some(DeductionModal {
            keep_length:
                format_duration(Duration::from_secs(session_data.length as u64)).to_string().into(),
            keep_video_length:
                format_duration(Duration::from_secs(session_data.video_length as u64)).to_string().into()
        }),
        Some(Duration::from_secs(5 * 60))
    ).await?.ok_or(Error::from("Failure retrieving modal data."))?;

    let new_length =
        parse_duration(&deduction.keep_length.unwrap_or(String::new()))
        .unwrap_or(Duration::ZERO);
    let new_video_length =
        parse_duration(&deduction.keep_video_length.unwrap_or(String::new()))
        .unwrap_or(Duration::ZERO);

    let delete_session = new_length.is_zero();

    if !delete_session &&
        (new_video_length > old_video_length ||
        new_video_length > new_length) {
        Err(Error::from("New video length can neither be greater than the old video length, or the new total length."))?
    }

    if new_length > old_length {
        Err(Error::from("New length cannot be greater than the old length."))?
    }

    let content_prepended_message = if !delete_session {
        {
            let new_length = new_length.as_secs() as i64;
            let new_video_length = new_video_length.as_secs() as i64;

            sqlx::query!("
            UPDATE study_sessions
            SET
                length = $2,
                video_length = $3
            WHERE id = $1
            ", session_id, new_length, new_video_length)
                .execute(&data.db_pool)
                .await?;
        }

        format!(
            "**Session deduction (original study times are outdated):**\nNew time: **{}** (**{}** removed)\nNew video time: **{}** (**{}** removed)\n\n",
            format_duration(new_length),
            format_duration(old_length - new_length),
            format_duration(new_video_length),
            format_duration(old_video_length - new_video_length)
        )
    } else {
        sqlx::query!("DELETE FROM study_sessions WHERE id = $1", session_id)
            .execute(&data.db_pool)
            .await?;

        "**This session has been deleted!**\n\n".to_string()
    };

    let message = &interaction.message;

    let edited_message = EditMessage::new()
        .content(content_prepended_message + &message.content);

    let edited_message = {
        if !delete_session {
            edited_message
        } else {
            edited_message
                .components(Vec::new())
        }
    };

    message.clone().edit(
        &ctx.http(),
        edited_message
    ).await?;

    Ok(())
}
