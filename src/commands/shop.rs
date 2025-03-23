use poise::{serenity_prelude::{CreateActionRow, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption}, CreateReply};

use crate::Context;

const ITEM_SELECT_MENU_ID: &'static str = "shop_select_items_menu";

/// Shop boosters, roles, and more!
#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn shop(ctx: Context<'_>) -> anyhow::Result<()> {
    ctx.send(CreateReply::default()
        .components(vec![
            CreateActionRow::SelectMenu(
                CreateSelectMenu::new(ITEM_SELECT_MENU_ID, CreateSelectMenuKind::String { options: vec![
                    CreateSelectMenuOption::new("", "")
                ] })
                .max_values(1)
                .placeholder("Choose product"))
        ]));

    Ok(())
}
