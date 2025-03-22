use poise::ChoiceParameter;

use crate::{study::{user_results_mode, ResultsMode}, Context, Error};

/// Set or view study result destination.
#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn results(
    ctx: Context<'_>,
    #[description = "Result messages mode"]
    mode: Option<ResultsMode>
) -> Result<()> {
    match mode {
        Some(m) => {
            let uid = i64::from(ctx.author().id);
            let mode_repr = m as u8;
            sqlx::query!("
            INSERT OR REPLACE INTO study_result_preferences
            VALUES ((SELECT id FROM users WHERE uid = $1), $2)
            ", uid, mode_repr)
                .execute(&ctx.data().db_pool)
                .await?;
            ctx.reply(format!("Changed result location to: **{}**", m.name())).await?;
        }
        None => {
            let mode = user_results_mode(&mut ctx.data().db_pool.acquire().await?).await;
            ctx.reply(format!("Study results location: **{}**", mode.name())).await?;
        }
    }
    Ok(())
}
