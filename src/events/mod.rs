mod interactions;
mod deduct_session;
mod reveal_reward;

use interactions::interaction_handler;
use log::info;
use poise::serenity_prelude::{Context, FullEvent::{self, *}};
use crate::{scheduling::add_scheduler_items, bumping::check_bump, study::voice_state_update};

use crate::{users::create_user, Data};

pub async fn event_handler(ctx: &Context, event: &FullEvent, data: &Data) -> anyhow::Result<()> {
    info!("Event handler: {:?}", event.snake_case_name());

    match event {
        Ready { data_about_bot: _ } => {
            add_scheduler_items(ctx, data).await?;
            Ok(())
        }
        Message { new_message } => {
            check_bump(ctx, new_message, data).await;
            Ok(())
        }
        InteractionCreate { interaction } =>
            interaction_handler(ctx, data, interaction).await,
        GuildMemberAddition { new_member } => {
            create_user(&mut data.db_pool.acquire().await.unwrap(), new_member.user.id).await?;
            Ok(())
        }
        VoiceStateUpdate { old, new } =>
            voice_state_update(ctx, data, old.as_ref(), &new).await,
        _ => Ok(())
    }
}
