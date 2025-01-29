use poise::ChoiceParameter;

use crate::{prelude::ActOnUser, study::{user_results_mode, ResultsMode}, Context, Error};

/// Set or view study result destination.
#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn results(
    ctx: Context<'_>,
    #[description = "Result messages mode"]
    mode: Option<ResultsMode>
) -> Result<(), Error> {
    let act_on_user_ctx = &ActOnUser(&ctx.data().db_pool, ctx.author().id);
    match mode {
        Some(m) => {
            let uid = act_on_user_ctx.uid();
            let mode_repr = m as u8;
            sqlx::query!("
            INSERT OR REPLACE INTO study_result_preferences
            VALUES ((SELECT id FROM users WHERE uid = $1), $2)
            ", uid, mode_repr)
                .execute(act_on_user_ctx.0)
                .await.unwrap();
            ctx.reply(format!("Changed result location to: **{}**", m.name())).await?;
        }
        None => {
            let mode = user_results_mode(act_on_user_ctx).await;
            ctx.reply(format!("Study results location: **{}**", mode.name())).await?;
        }
    }
    Ok(())
}
