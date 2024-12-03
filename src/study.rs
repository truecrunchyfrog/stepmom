use std::time::Duration;

use humantime::format_duration;
use poise::serenity_prelude::{futures::{future::join_all, lock::Mutex}, ButtonStyle, CacheHttp, ChannelId, Context, CreateButton, CreateMessage, FutureExt, Mentionable, MessageBuilder, User, UserId, VoiceState};
use rand::Rng;
use sqlx::types::time::OffsetDateTime;
use tokio::time::Instant;

use crate::{leaderboard::{real_leaderboard_start_datetime, user_place}, prelude::{try_dm_or_in_guild, ActOnUser}, rewards::{user_claim_reward, Reward}, Channels, Data, Error};

fn is_study_vc(channels_config: &Channels, channel_id: ChannelId) -> bool {
    !channels_config.slacking_voice_channels.contains(&u64::from(channel_id))
}

fn is_voice_state_studying(channels_config: &Channels, state: &VoiceState) -> bool {
    state.channel_id.map(|cid| is_study_vc(channels_config, cid))
        .unwrap_or(false)
}

pub struct StudyState {
    pub start: Instant,

    pub video_start: Mutex<Option<Instant>>,
    pub video_sum: Mutex<Duration>,

    pub break_start: Mutex<Option<Instant>>,
    pub break_sum: Mutex<Duration>
}

/// Only for visual representation.
pub struct StudyResult<'a> {
    user: &'a User,

    session_id: i64,

    start: OffsetDateTime,
    end: OffsetDateTime,

    length: Duration,
    video_length: Duration,
    next_video_reward: Option<Duration>,
    breaks: Duration,

    leaderboard_place: Option<(u16, Option<u16>)>,

    coins: u64,

    /// (after, before)
    streak: (u16, u16)
}

impl StudyState {
    /// Moves the current progress from video_start to video_prev_total.
    /// Used when video ends, to summarize.
    async fn sum_video_progress(&self) {
        let mut start = self.video_start.lock().await;
        if let Some(inst) = *start {
            let mut sum = self.video_sum.lock().await;
            *sum += inst.elapsed();
            *start = None;
        }
    }

    async fn sum_break_progress(&self) {
        let mut start = self.break_start.lock().await;
        if let Some(inst) = *start {
            let mut sum = self.break_sum.lock().await;
            *sum += inst.elapsed();
            *start = None;
        }
    }
}

pub async fn voice_state_update(ctx: &Context, data: &Data, old: Option<&VoiceState>, new: &VoiceState) -> Result<(), Error> {
    let study_before = old
        .map(|vs| is_voice_state_studying(&data.config.channels, &vs))
        .unwrap_or(false);
    let study_now = is_voice_state_studying(&data.config.channels, new);

    match (study_before, study_now) {
        (false, true) => begin_studying(ctx, data, new.user_id).await,
        (true, false) => end_studying(ctx, data, new.user_id).await,
        _ => ()
    }

    video_state_update(ctx, data, new).await;

    Ok(())
}

async fn begin_studying(ctx: &Context, data: &Data, user_id: UserId) {
    let mut study_states = data.study_states.lock().await;

    study_states.insert(user_id, StudyState {
        start: Instant::now(),

        video_start: None.into(),
        video_sum: Duration::ZERO.into(),

        break_start: None.into(),
        break_sum: Duration::ZERO.into()
    });
}

async fn end_studying(ctx: &Context, data: &Data, user_id: UserId) {
    let mut study_states = data.study_states.lock().await;

    let Some(state) = study_states.remove(&user_id) else { return };
    finish_session(ctx, data, user_id, state, true).await;
}

pub async fn finish_session(ctx: &Context, data: &Data, user_id: UserId, state: StudyState, alert: bool) {
    state.sum_video_progress().await;
    state.sum_break_progress().await;

    let length = state.start.elapsed();
    let video_length = state.video_sum.into_inner();

    let coins = (length.as_secs() / 60)
        * data.config.study_earnings.coins_per_minute;
    let uid = i64::from(user_id);

    let act_on_user_ctx =
        &ActOnUser(&data.db_pool, user_id);

    let lb_start = real_leaderboard_start_datetime();

    let lb_place_before =
        user_place(act_on_user_ctx, lb_start).await;

    let user = ctx.http().get_user(user_id).await.unwrap();

    let streak_before = user_streak(act_on_user_ctx).await;

    let session_id = {
        let coins = coins as i64;

        let coin_reward_id = sqlx::query!("
        INSERT INTO coin_transactions (user_id, coins_diff)
        SELECT users.id, $2 FROM users WHERE uid = $1
        ", uid, coins)
            .execute(&data.db_pool)
            .await
            .unwrap()
            .last_insert_rowid();

        let length = length.as_secs() as i64;
        let video_length = video_length.as_secs() as i64;

        let ended = OffsetDateTime::now_utc();

        sqlx::query!("
        INSERT INTO study_sessions (user_id, coin_reward_id, length, video_length, ended)
        SELECT users.id, $2, $3, $4, $5 FROM users WHERE uid = $1
        ", uid, coin_reward_id, length, video_length, ended)
            .execute(&data.db_pool)
            .await
            .unwrap()
            .last_insert_rowid()
    };

    let lb_place_after =
        user_place(act_on_user_ctx, lb_start).await;

    let streak_after = user_streak(act_on_user_ctx).await;

    let mut rewards = Vec::new();

    if streak_before != streak_after {
        rewards.push("Daily reward");
    }

    let default_reward_time = random_video_reward_time().as_secs() as i64;

    sqlx::query!("
    INSERT OR IGNORE INTO video_rewards_time_left
    VALUES ((SELECT id FROM users WHERE uid = $1), $2)
    ", uid, default_reward_time)
        .execute(&data.db_pool)
        .await.unwrap();

    let mut depositable_video_length = video_length;

    while !depositable_video_length.is_zero() {
        let video_time_left = Duration::from_secs(sqlx::query!("
        SELECT time_left FROM video_rewards_time_left
        WHERE user_id IN (SELECT id FROM users WHERE uid = $1)
        ", uid)
            .fetch_one(&data.db_pool)
            .await.unwrap()
            .time_left as u64);

        match video_time_left
            .checked_sub(depositable_video_length)
            .map(|d| d.as_secs() as i64) {
            // No overflow - only subtract time.
            Some(new_time_left) =>
            {
                sqlx::query!("
                UPDATE video_rewards_time_left
                SET time_left = $2
                WHERE user_id IN (SELECT id FROM users WHERE uid = $1)
                ", uid, new_time_left)
                    .execute(&data.db_pool)
                    .await.unwrap();
            }
            // Overflow - replace time and continue.
            None =>
            {
                rewards.push("Video reward");

                let new_time = random_video_reward_time().as_secs() as i64;

                sqlx::query!("
                UPDATE video_rewards_time_left
                SET time_left = $2
                WHERE user_id IN (SELECT id FROM users WHERE uid = $1)
                ", uid, new_time)
                    .execute(&data.db_pool)
                    .await.unwrap();
            }
        }

        depositable_video_length =
            depositable_video_length
            .checked_sub(video_time_left)
            .unwrap_or(Duration::ZERO);
    }

    let claimed_rewards = join_all(rewards.iter().map(|reason| {
        user_claim_reward(
            act_on_user_ctx,
            Reward::random(),
            reason.to_string())
            .map(|u| (*reason, u))
    }).collect::<Vec<_>>()).await;

    if alert {
        let now = OffsetDateTime::now_utc();

        let mut messages = Vec::new();

        let next_video_reward = if !video_length.is_zero() {
            Duration::from_secs(
            sqlx::query!("
            SELECT time_left FROM video_rewards_time_left
            WHERE user_id IN (SELECT id FROM users WHERE uid = $1)
            ", uid)
                .fetch_one(&data.db_pool)
                .await.unwrap()
                .time_left as u64).into()
        } else { None };

        messages.push(result_message(StudyResult {
            user: &user,
            session_id,

            start: now - state.start.elapsed(),
            end: now,
            length,
            video_length,
            next_video_reward,
            breaks: state.break_sum.into_inner(),

            leaderboard_place: lb_place_after.map(|after| (after, lb_place_before)),
            coins,
            streak: (streak_after, streak_before)
        }).await);

        messages.extend(claimed_rewards.iter().map(|r| {
            CreateMessage::new()
                .content(
                    MessageBuilder::new()
                    .push_line("-# We've got a present for you!")
                    .push("# :piñata: ")
                    .push(r.0)
                    .build())
                .button(
                    CreateButton::new(format!("reveal_reward_{}", r.1))
                    .label("Reveal")
                    .style(ButtonStyle::Success))
        }));

        match user_results_mode(act_on_user_ctx).await {
            ResultsMode::Off => (),
            ResultsMode::Dm => {
                for msg in messages {
                    try_dm_or_in_guild(ctx, data, ctx.http(), &user, msg).await;
                }
            },
            ResultsMode::Guild => {
                let channel = &ctx.http().get_channel(
                    ChannelId::new(data.config.channels.dm_backup_channel))
                    .await.unwrap()
                    .guild().unwrap();

                for msg in messages {
                    let guild_msg = channel
                        .send_message(ctx.http(), msg)
                        .await.unwrap();

                    guild_msg.reply(ctx.http(), user.mention().to_string()).await.unwrap();
                }
            }
        }
    }
}

fn random_video_reward_time() -> Duration {
    Duration::from_secs(rand::thread_rng().gen_range(1..12) * 30 * 60)
}

pub async fn user_streak(ctx: &ActOnUser<'_>) -> u16 {
    let uid = ctx.uid();

    let study_days = sqlx::query!("
    SELECT DISTINCT JULIANDAY(DATE(ended, 'unixepoch')) AS date
    FROM study_sessions
    WHERE
        user_id IN (SELECT id FROM users WHERE uid = $1) AND
        length > 10 * 60
    ORDER BY ended DESC
    ", uid)
        .fetch_all(ctx.0)
        .await.unwrap();

    study_days
        .iter()
        .flat_map(|r| r.date)
        .map(|r| r as i64)
        .fold((0u16, -1i64), |acc, d| {
            if d == acc.1 - 1 || acc.1 == -1 {
                (acc.0 + 1, d)
            } else {
                (acc.0, 0)
            }
        }).0
}

async fn result_message(result: StudyResult<'_>) -> CreateMessage {
    let content = {
        let mut b = MessageBuilder::new();

        if result.streak.0 != result.streak.1 && result.streak.0 > 1 {
            b.push(":wing: ");
            b.push_bold(result.streak.1.to_string());
            b.push(" → ");
            b.push_bold(result.streak.0.to_string());
            b.push_line(" day streak!");
        }

        b.push(":stopwatch: ");
        b.push_bold(format_duration(Duration::from_secs(result.length.as_secs())).to_string());
        b.push_line(format!(" studied: <t:{}:t> → <t:{}:t>",
            result.start.unix_timestamp(),
            result.end.unix_timestamp()));

        if !result.video_length.is_zero() {
            let ratio = result.video_length.as_secs() as f32 / result.length.as_secs() as f32;
            b.push(":video_camera: ");
            b.push_bold(
                format_duration(Duration::from_secs(result.video_length.as_secs())).to_string());
            b.push_line(format!(" of video streamed ({}%)", (ratio * 100.0) as u64));
        }

        if let Some(time_left) = result.next_video_reward {
            b.push(":gift: ");
            b.push_bold(format_duration(time_left).to_string());
            b.push_line(" of video streaming left until next reward");
        }

        b.push(":purse: ");
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
                b.push(" → ");
                b.push_bold(current_place.to_string());
            }
            _ => ()
        }

        b.push("-# Manage these results with ");
        b.push_line(format!("</{}:{}>", "results", "1311019874134265928"));

        b.push("-# Session ID: ");
        b.push_mono_line(result.session_id.to_string());

        b.build()
    };

    CreateMessage::new()
        .content(content)
        .button(
            CreateButton::new(format!("deduct_session_{}", result.session_id))
            .label("Deduction Penalty")
            .style(poise::serenity_prelude::ButtonStyle::Danger)
        )
}

async fn video_state_update(ctx: &Context, data: &Data, voice_state: &VoiceState) {
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
            drop(stream_start);
            state.sum_video_progress().await;
        }
        _ => ()
    }
}

#[derive(Clone, Copy, poise::ChoiceParameter)]
#[repr(u8)]
pub enum ResultsMode {
    #[name = "Disabled"]
    Off = 0,
    #[name = "DM"]
    Dm = 1,
    #[name = "In server (publicly)"]
    Guild = 2
}

pub async fn user_results_mode(ctx: &ActOnUser<'_>) -> ResultsMode {
    let uid = ctx.uid();
    match sqlx::query!("
    SELECT mode FROM study_result_preferences
    WHERE user_id IN (SELECT id FROM users WHERE uid = $1)
    ", uid)
        .fetch_optional(ctx.0)
        .await.unwrap()
        .map(|r| r.mode)
        .unwrap_or(1) {
        0 => ResultsMode::Off,
        1 => ResultsMode::Dm,
        2 => ResultsMode::Guild,
        _ => unreachable!()
    }
}
