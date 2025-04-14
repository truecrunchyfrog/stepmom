use anyhow::anyhow;
use poise::{serenity_prelude::{self, ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditMessage, MessageBuilder}, CreateReply};

use crate::{coins::take_coins, shop::ShopItem, Context};

/// Shop boosters, roles, and more!
#[poise::command(slash_command, prefix_command, ephemeral, guild_only)]
pub async fn shop(ctx: Context<'_>) -> anyhow::Result<()> {
    let uuid = ctx.id();

    let shop_items_zipped = ctx.data().config.shop.iter().zip(0..).collect::<Vec<_>>();

    let options = shop_items_zipped.iter()
        .map(|(item, index)| CreateSelectMenuOption::new(item.product.to_string(), index.to_string()))
        .into_iter().collect::<Vec<_>>();

    ctx.send(CreateReply::default()
        .components(vec![
            CreateActionRow::SelectMenu(
                CreateSelectMenu::new(format!("select_{uuid}"), CreateSelectMenuKind::String { options })
                .max_values(1)
                .placeholder("Choose product")),
            CreateActionRow::Buttons(vec![
                CreateButton::new(format!("buy_{uuid}"))
                .label("Buy")
            ])
        ])).await?;

    let mut selected_item: Option<&ShopItem> = None;

    while let Some(mci) = ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(10 * 60))
        .filter(move |mci| mci.data.custom_id.ends_with(&format!("_{uuid}")))
        .await
    {
        match mci.data.kind {
            ComponentInteractionDataKind::StringSelect { ref values } => {
                let selected_item_index = values[0].parse::<u32>()?;
                let item = shop_items_zipped.iter().find(|i| i.1 == selected_item_index)
                    .ok_or(anyhow!("Selected index does not exist among shop items."))?.0;
                selected_item = Some(item);

                mci.message.clone().edit(ctx, EditMessage::new().embed(
                        CreateEmbed::new()
                        .title(item.product.to_string())
                        .field(item.cost.to_string(), "coins", false)
                        .description(item.description.to_owned()))).await?;
            }
            ComponentInteractionDataKind::Button => {
                match selected_item {
                    Some(item) => {
                        let member = mci.member.as_ref().expect("Cannot retrieve member.");
                        let mut tx = ctx.data().db_pool.begin().await?;
                        take_coins(
                            &mut tx,
                            member.user.id,
                            item.cost,
                            format!("buy {}", item.product.to_string()),
                            None).await?;
                        item.product.give_to_member(&mut tx, ctx, &member).await?;
                        tx.commit().await?;
                        mci.create_response(ctx, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content(
                                    MessageBuilder::new()
                                    .push("Bought: ")
                                    .push_bold_line(item.product.to_string())
                                    .push("Cost: ")
                                    .push_bold(item.cost.to_string())
                                    .build()))).await?;
                    }
                    None => mci.create_response(ctx, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content("Select something to buy first!"))).await?
                }
            }
            _ => unreachable!()
        }

        mci.create_response(ctx, serenity_prelude::CreateInteractionResponse::Acknowledge)
            .await?;
    }

    Ok(())
}
