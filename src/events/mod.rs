mod interactions;
mod deduct_session;
mod reveal_reward;

use interactions::interaction_handler;
use log::info;
use poise::serenity_prelude::{CacheHttp, Context, FullEvent::{self, *}, MessageCreateEvent};
use crate::{add_scheduler_items, bumping::check_bump, study::voice_state_update, Error};

use crate::{prelude::{create_user, ActOnUser}, Data};

pub async fn event_handler(ctx: &Context, event: &FullEvent, data: &Data) -> Result<(), Error> {
    info!("Event handler: {:?}", event.snake_case_name());

    match event {
        Ready { data_about_bot } => {
            add_scheduler_items(ctx, data);
            Ok(())
        }
        Message { new_message } => {
            check_bump(ctx, new_message, data).await;
            Ok(())
        }
        InteractionCreate { interaction } =>
            interaction_handler(ctx, data, interaction).await,
        GuildMemberAddition { new_member } =>
        {
            create_user(&ActOnUser(&data.db_pool, new_member.user.id)).await;
            Ok(())
        }
        VoiceStateUpdate { old, new } =>
            voice_state_update(ctx, data, old.as_ref(), &new).await,
        _ => Ok(())
    }
}
