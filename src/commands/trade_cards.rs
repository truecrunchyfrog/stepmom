use crate::{trade_cards::{fetch_cards, pick_random_card}, Context, Error};

#[poise::command(slash_command, prefix_command, subcommands("buy", "sell", "list", "trade"))]
pub async fn trade_cards(_ctx: Context<'_>) -> Result<(), Error> { Ok(()) }

#[poise::command(slash_command, prefix_command)]
pub async fn buy(ctx: Context<'_>) -> Result<(), Error> {
    // TODO No, cards should not be bought here, but in a separate, centralized shop.

    let cards = fetch_cards(&ctx.data().db_pool).await?;

    let receive_card = pick_random_card(&cards)?;

    unimplemented!()
}

#[poise::command(slash_command, prefix_command)]
pub async fn sell(ctx: Context<'_>) -> Result<(), Error> {


    unimplemented!()
}

#[poise::command(slash_command, prefix_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    unimplemented!()
}

#[poise::command(slash_command, prefix_command)]
pub async fn trade(ctx: Context<'_>) -> Result<(), Error> {
    unimplemented!()
}
