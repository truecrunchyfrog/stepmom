use humantime::format_duration;
use poise::{serenity_prelude::{CreateAllowedMentions, CreateMessage, MessageBuilder}, CreateReply};

use crate::{leaderboard::{self, real_leaderboard_start_datetime}, Context, Error};

const USERS_PER_PAGE: usize = 10;

/// See this month's current standings.
#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Leaderboard page"]
    page: Option<usize>
) -> Result<(), Error> {
    let leaderboard = leaderboard::fetch_leaderboard(
        &ctx.data().db_pool,
        real_leaderboard_start_datetime(),
        None
    ).await;

    let default_pos = leaderboard.iter()
        .position(|l| l.0 == ctx.author().id)
        .map_or(0, |pos| pos / USERS_PER_PAGE * USERS_PER_PAGE);

    let start_pos = page.map_or(
        default_pos,
        |p| p.checked_sub(1).unwrap_or(0) * USERS_PER_PAGE
    );

    if start_pos >= leaderboard.len() {
        return Err(Error::from(
                format!("Out of range! Max pages available: {}.",
                    (leaderboard.len() - 1) / USERS_PER_PAGE + 1)))
    }

    let end_pos = (start_pos + USERS_PER_PAGE).min(leaderboard.len());

    let display_places = &leaderboard[start_pos..end_pos];

    ctx.send(CreateReply::default()
        .content({
            let mut b = MessageBuilder::new();

            b.push_line("## Leaderboard");
            b.push("-# Page ");
            b.push_line((start_pos / USERS_PER_PAGE + 1).to_string());

            for ((user, duration), place) in display_places.iter().zip(start_pos + 1..) {
                if place <= 3 {
                    b.push("#".repeat(place));
                    b.push(" ");
                }
                b.push_bold(format!("{}. ", place));
                b.user(user);
                b.push(" ");
                b.push_bold_line(format_duration(*duration).to_string());
            }

            b.build()
        })
        .allowed_mentions(CreateAllowedMentions::new())
    ).await.unwrap();

    Ok(())
}
