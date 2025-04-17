use num_format::{Locale, ToFormattedString};
use poise::{serenity_prelude::{AutocompleteChoice, Color, ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, Mentionable, MessageBuilder, UserId}, CreateReply};

use crate::{coins::take_coins, shop::ShopItem, Context};

async fn autocomplete_shop_item(
    ctx: Context<'_>,
    _partial: &str
) -> impl Iterator<Item = AutocompleteChoice> {
    ctx.data().config.shop.iter()
        .map(|item| AutocompleteChoice::new(item.product.to_string(), item.product.to_string()))
        .collect::<Vec<_>>()
        .into_iter()
}

/// Shop boosters, roles, and more!
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn shop(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_shop_item"]
    #[rename = "item"]
    #[description = "Shop item"]
    item_name: String
) -> anyhow::Result<()> {
    let uuid = ctx.id();

    let buy_custom_id = format!("buy_{uuid}");
    let gift_target_custom_id = format!("gifttarget_{uuid}");

    let Some(item) = ctx.data().config.shop.iter().find(|i| i.product.to_string() == item_name) else { anyhow::bail!("No product by such name.") };

    let mut message = CreateReply::default()
        .components(vec![
            CreateActionRow::SelectMenu(
                CreateSelectMenu::new(
                    &gift_target_custom_id,
                    CreateSelectMenuKind::User { default_users: None })
                    .placeholder("Gift")
                    .min_values(0)
                    .max_values(1)),
            CreateActionRow::Buttons(vec![
                CreateButton::new(&buy_custom_id).label("Buy")])
        ]);

    let mut create_embed = CreateEmbed::new()
        .color(Color::ROHRKATZE_BLUE)
        .title(format!("{} {}", item.product.emoji(), item.product))
        .field(item.cost.to_formatted_string(&Locale::en), "coins", false)
        .fields(item.product.fields().iter().map(|f| (f.0.to_owned(), f.1.to_owned(), true)))
        .description(item.description.to_owned());

    if let Some(attachment) = item.product.get_attachment() {
        let filename = attachment.filename.clone();
        message = message.attachment(attachment);
        create_embed = create_embed.attachment(filename);
    }

    message = message.embed(create_embed);
    ctx.send(message).await?;

    let mut confirm_buy = false;
    let mut gift_target = None;

    while let Some(mci) = ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(10 * 60))
        .filter(move |mci|
            mci.data.custom_id.split_once('_')
                .map(|(_, id)| id == uuid.to_string())
                .unwrap_or(false))
        .await
    {
        match &mci.data.custom_id {
            id if id == &buy_custom_id && !confirm_buy => {
                confirm_buy = true;

                mci.create_response(ctx, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(
                        MessageBuilder::new()
                            .push_line("# Confirm purchase!")
                            .push_line("Push `Buy` again to confirm your purchase.")
                            .push("What: ")
                            .push_bold_line(item.product.to_string())
                            .push("Cost: ")
                            .push_bold(item.cost.to_formatted_string(&Locale::en))
                            .push_line(" coins")
                            .push("Recipient: ")
                            .push_bold(
                                gift_target.map(|uid: UserId| uid.mention().to_string())
                                .unwrap_or("yourself".to_string()))
                            .build())
                        .allowed_mentions(CreateAllowedMentions::new())
                        .ephemeral(true))).await?;
            }

            id if id == &buy_custom_id => {
                // Buy

                let member = mci.member.as_ref().expect("Cannot retrieve member.");

                let mut tx = ctx.data().db_pool.begin().await?;
                take_coins(&mut tx, member.user.id, item.cost, item.product.to_string(), None).await?;
                item.product.give_to_member(&mut tx, ctx, member).await?;
                tx.commit().await?;

                mci.create_response(ctx, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(
                        MessageBuilder::new()
                            .push("Bought: ")
                            .push_bold_line(item.product.to_string())
                            .push("Cost: ")
                            .push_bold(item.cost.to_formatted_string(&Locale::en))
                            .build())
                        .ephemeral(true))).await?;
            }

            id if id == &gift_target_custom_id => {
                // Gift

                let ComponentInteractionDataKind::UserSelect { ref values } =
                mci.data.kind else { anyhow::bail!("Invalid gift targets data kind.") };

                confirm_buy = false;

                gift_target = match values[..] {
                    [target] => Some(target),
                    [] => None,
                    _ => unreachable!()
                };

                mci.create_response(ctx, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(
                        gift_target.map(|uid| format!("Gifting to {}", uid.mention()))
                            .unwrap_or("Purchasing for yourself".to_string()))
                        .ephemeral(true))).await?;
            }

            _ => unreachable!()
        }
    }

    Ok(())
}
