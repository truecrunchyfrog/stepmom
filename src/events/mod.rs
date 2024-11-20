pub mod study;

use std::{borrow::{Borrow, BorrowMut}, time::Duration};

use poise::serenity_prelude::{self as serenity, futures::lock::Mutex, ChannelId, FullEvent::{self, *}, UserId, VoiceState};
use study::voice_state_update;
use tokio::time::Instant;

use crate::{prelude::{create_user, ActOnUser}, Channels, Data};

pub async fn event_handler(data: &Data, event: &FullEvent) {
    match event {
        GuildMemberAddition { new_member } =>
            create_user(&ActOnUser(&data.db_pool, new_member.user.id)).await,
        VoiceStateUpdate { old, new } =>
            voice_state_update(data, old.as_ref(), &new).await,
        _ => ()
    }
}
