use std::{borrow::{Borrow, BorrowMut}, time::Duration};

use humantime::format_duration;
use poise::serenity_prelude::{self as serenity, futures::lock::Mutex, ChannelId, CreateMessage, FullEvent::{self, *}, MessageBuilder, User, UserId, VoiceState};
use tokio::time::Instant;

use crate::{prelude::{create_user, ActOnUser}, Channels, Data};

fn is_study_vc(channels_config: &Channels, channel_id: ChannelId) -> bool {
    !channels_config.slacking_voice_channels.contains(&u64::from(channel_id))
}

fn is_voice_state_studying(channels_config: &Channels, state: &VoiceState) -> bool {
    state.channel_id.map(|cid| is_study_vc(channels_config, cid))
        .unwrap_or(false)
}

pub struct StudyState {
    start: Instant,

    video_start: Mutex<Option<Instant>>,
    video_sum: Mutex<Duration>
}

pub struct StudyResult {
    user: User,

    session_id: i64,

    length: Duration,
    video_length: Duration,
    leaderboard_place: Option<(u16, Option<u16>)>,

    coins: u64,
}

impl StudyState {
    /// Moves the current progress from video_start to video_prev_total.
    /// Used when video ends, to summarize.
    async fn sum_progress(&self) {
        let mut start = self.video_start.lock().await;
        if let Some(inst) = *start {
            let mut sum = self.video_sum.lock().await;
            *sum += inst.elapsed();
            *start = None;
        }
    }
}

pub async fn voice_state_update(data: &Data, old: Option<&VoiceState>, new: &VoiceState) {
    let study_before = old
        .map(|vs| is_voice_state_studying(&data.config.channels, &vs))
        .unwrap_or(false);
    let study_now = is_voice_state_studying(&data.config.channels, new);

    match (study_before, study_now) {
        (false, true) => begin_studying(data, new.user_id).await,
        (true, false) => end_studying(data, new.user_id).await,
        _ => ()
    }

    video_state_update(data, new);
}

async fn begin_studying(data: &Data, user_id: UserId) {
    let mut study_states = data.study_states.lock().await;

    study_states.insert(user_id, StudyState {
        start: Instant::now(),
        video_start: None.into(),
        video_sum: Duration::ZERO.into()
    });
}

async fn end_studying(data: &Data, user_id: UserId) {
    let mut study_states = data.study_states.lock().await;

    let Some(state) = study_states.remove(&user_id) else { return };
    state.sum_progress();

    let duration = state.start.elapsed();
    let video_sum = state.video_sum.into_inner();

    // TODO
    todo!()
}

async fn result_message(result: StudyResult) -> CreateMessage {
    let content = {
        let mut b = MessageBuilder::new();
        b.push_bold(format_duration(result.length).to_string());
        b.push_line(" studied");

        b.push_bold(format!("+{}", result.coins));
        b.push_line(" coins");

        match result.leaderboard_place {
            Some((current_place, None)) => {
                b.push("Leaderboard: ");
                b.push_bold_line(current_place.to_string());
            }
            Some((current_place, Some(previous_place))) => {
                b.push("Leaderboard climbed: ");
                b.push_bold(previous_place.to_string());
                b.push(" -> ");
                b.push_bold(current_place.to_string());
            }
            _ => ()
        }

        b.push("-# Manage these results with ");
        b.push_mono_line("/studymessages");

        b.push("-# Session ID: ");
        b.push_mono_line(result.session_id.to_string());

        b.build()
    };

    CreateMessage::new()
        .content(content)
}

async fn video_state_update(data: &Data, voice_state: &VoiceState) {
    let study_states = data.study_states.lock().await;

    let Some(state) = study_states
        .get(&voice_state.user_id)
        else { return };
    let mut stream_start = state.video_start.lock().await;

    let now_streaming =
        voice_state.self_video ||
        voice_state.self_stream.unwrap_or(false);

    match (*stream_start, now_streaming) {
        (None, true) => {
            *stream_start = Some(Instant::now());
        }
        (Some(_), false) => {
            state.sum_progress();
        }
        _ => ()
    }
}
