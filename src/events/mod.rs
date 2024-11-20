pub mod study;


use poise::serenity_prelude::FullEvent::{self, *};
use study::voice_state_update;

use crate::{prelude::{create_user, ActOnUser}, Data};

pub async fn event_handler(data: &Data, event: &FullEvent) {
    match event {
        GuildMemberAddition { new_member } =>
            create_user(&ActOnUser(&data.db_pool, new_member.user.id)).await,
        VoiceStateUpdate { old, new } =>
            voice_state_update(data, old.as_ref(), &new).await,
        _ => ()
    }
}
