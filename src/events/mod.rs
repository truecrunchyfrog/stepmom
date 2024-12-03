mod interactions;
mod deduct_session;
mod reveal_reward;

use interactions::interaction_handler;
use log::info;
use poise::serenity_prelude::{Context, FullEvent::{self, *}};
use crate::{study::voice_state_update, Error};

use crate::{prelude::{create_user, ActOnUser}, Data};

pub async fn event_handler(ctx: &Context, event: &FullEvent, data: &Data) -> Result<(), Error> {
    info!("Event handler: {:?}", event.snake_case_name());

    match event {
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
