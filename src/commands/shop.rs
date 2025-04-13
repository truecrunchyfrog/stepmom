use poise::{serenity_prelude::{self, ComponentInteractionCollector, CreateActionRow, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption}, CreateReply};

use crate::Context;

/// Shop boosters, roles, and more!
#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn shop(ctx: Context<'_>) -> anyhow::Result<()> {
    let uuid = ctx.id().to_string();

    ctx.send(CreateReply::default()
        .components(vec![
            CreateActionRow::SelectMenu(
                CreateSelectMenu::new(&uuid, CreateSelectMenuKind::String {
                    options: ctx.data().config.shop
                        .iter().zip(0..)
                        .map(|(item, index)| CreateSelectMenuOption::new(item.product.to_string(), index.to_string()))
                        .into_iter().collect::<Vec<_>>()
                })
                .max_values(1)
                .placeholder("Choose product"))
        ])).await?;

    while let Some(mci) = ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(10 * 60))
        .filter(|mci| &mci.data.custom_id == &uuid)
        .await
    {

        mci.create_response(ctx, serenity_prelude::CreateInteractionResponse::Acknowledge)
            .await?;
    }

    Ok(())
}
