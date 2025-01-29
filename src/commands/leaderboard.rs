use crate::{leaderboard::{self, real_leaderboard_start_datetime}, Context, Error};

#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn leaderboard(
    ctx: Context<'_>
) -> Result<(), Error> {
    leaderboard::fetch_leaderboard(
        &ctx.data().db_pool,
        real_leaderboard_start_datetime(),
        None
    ).await;

    // TODO

    Ok(())
}
