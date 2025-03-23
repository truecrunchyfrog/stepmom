use crate::{trade_cards::{fetch_cards, pick_random_card}, Context};

#[poise::command(slash_command, prefix_command, subcommands("buy", "sell", "list", "trade"))]
pub async fn trade_cards(_ctx: Context<'_>) -> anyhow::Result<()> { Ok(()) }

#[poise::command(slash_command, prefix_command)]
pub async fn buy(ctx: Context<'_>) -> anyhow::Result<()> {
    // TODO No, cards should not be bought here, but in a separate, centralized shop.

    let cards = fetch_cards(&ctx.data().db_pool).await?;

    let receive_card = pick_random_card(&cards)?;

    unimplemented!()
}

#[poise::command(slash_command, prefix_command)]
pub async fn sell(ctx: Context<'_>) -> anyhow::Result<()> {


    unimplemented!()
}

#[poise::command(slash_command, prefix_command)]
pub async fn list(ctx: Context<'_>) -> anyhow::Result<()> {
    unimplemented!()
}

#[poise::command(slash_command, prefix_command)]
pub async fn trade(ctx: Context<'_>) -> anyhow::Result<()> {
    unimplemented!()
}
