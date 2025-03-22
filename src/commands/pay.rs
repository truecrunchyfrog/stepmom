use poise::{serenity_prelude::{MessageBuilder, User}, CreateReply};

use crate::{Context, DbCtx, Error};

/// Pay another user.
#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn pay(
    ctx: Context<'_>,
    #[description = "User to pay"]
    recipient: User,
    #[description = "Payment amount"]
    amount: u64
) -> Result<(), Error> {
    if amount < 1 {
        return Err(Error::from("Invalid amount. Be as generous as to give at least 1 coin, please."))
    }

    let mut tx = ctx.data().db_pool.begin().await?;
    let db_ctx = DbCtx(&mut tx);
    db_ctx.take_coins(ctx.author().id, amount, "a payment", None).await?;
    db_ctx.add_coins(recipient.id, amount).await?;
    tx.commit()?;

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
