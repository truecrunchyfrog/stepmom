use poise::{serenity_prelude::{MessageBuilder, User}, CreateReply};

use crate::{coins::{add_coins, take_coins}, Context};

/// Pay another user.
#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn pay(
    ctx: Context<'_>,
    #[description = "User to pay"]
    recipient: User,
    #[description = "Payment amount"]
    amount: u64
) -> anyhow::Result<()> {
    if amount < 1 {
        anyhow::bail!("Invalid amount. Be as generous as to give at least 1 coin, please.")
    }

    let mut tx = ctx.data().db_pool.begin().await?;
    take_coins(&mut tx, ctx.author().id, amount, "a payment".to_string(), None).await?;
    add_coins(&mut tx, recipient.id, amount).await?;
    tx.commit().await?;

    ctx.send(CreateReply::default()
        .content(MessageBuilder::new()
            .push("Paid ")
            .push_bold(amount.to_string())
            .push(" coins to ")
            .mention(&recipient)
            .push(".")
            .build()
            )).await?;

    Ok(())
}
