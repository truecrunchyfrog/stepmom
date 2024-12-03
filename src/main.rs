mod prelude;
mod commands;
mod events;
mod leaderboard;
mod study;
mod rewards;

use core::panic;
use std::collections::HashMap;
use std::fs;
use dotenv::dotenv;
use events::event_handler;
use prelude::create_user;
use prelude::ActOnUser;
use crate::study::StudyState;
use log::{error, info};
use poise::serenity_prelude::futures::lock::Mutex;
use poise::serenity_prelude::CacheHttp;
use poise::serenity_prelude::UserId;
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use sqlx::sqlite::SqlitePoolOptions;
use std::{sync::Arc, time::Duration};

#[derive(Deserialize)]
pub struct Config {
    study_earnings: StudyEarnings,
    channels: Channels,
    star_cost: StarCost,
    temp_charts_dir: String
}

#[derive(Deserialize)]
pub struct StudyEarnings {
    coins_per_minute: u64
}

#[derive(Deserialize)]
pub struct Channels {
    dm_backup_channel: u64,
    starboard_channel: u64,
    slacking_voice_channels: Vec<u64>
}

#[derive(Deserialize)]
pub struct StarCost {
    base: u64,
    per_character: f64,
    per_attachment: u64
}


pub struct Data {
    config: Config,
    db_pool: sqlx::SqlitePool,
    study_states: Mutex<HashMap<UserId, StudyState>>
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!("Error in command: `{}`: {:?}", ctx.command().name, error);

            ctx.send(
                ctx.reply_builder(CreateReply::default().ephemeral(true).content(format!(
                    "-# Command *{}* failed\n>>> {}",
                    ctx.command().name,
                    error
                )))
            )
            .await.unwrap();
        }
        poise::FrameworkError::EventHandler { error, ctx, event, framework, .. } => {
            error!("Error in event handler: {:?}", error);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenv().ok();

    env_logger::init();

    let config_filename = std::env::var("config").unwrap_or(String::from("config.toml"));

    let config: Config = toml::from_str(&fs::read_to_string(config_filename)
        .unwrap()).unwrap();

    let db_pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("SQLITE_CONNSTR")
            .expect("Missing SQLITE_CONNSTR environment variable"))
        .await?;

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::stats::stats(),
            commands::star::star(),
            commands::simulate_study_session::simulate_study_session(),
            commands::results::results()
        ],

        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("mom ".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            additional_prefixes: vec![poise::Prefix::Literal("stepmom ")],
            ..Default::default()
        },
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| {
            Box::pin(async move {
                info!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                info!("Executed command {}", ctx.command().qualified_name);
            })
        },
        command_check: Some(|_ctx| Box::pin(async move { Ok(true) })),
        event_handler: |ctx, event, _framework, data| {
            Box::pin(async move {
                event_handler(ctx, event, data).await
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                info!("Logged in as {}", _ready.user.name);

                info!("Creating users in database.");

                let guilds = ctx.http().get_guilds(None, None)
                    .await?;

                for guild in guilds {
                    let members = ctx.http()
                        .get_guild_members(guild.id, None, None)
                        .await?;
                    for member in members {
                        if !member.user.bot {
                            create_user(&ActOnUser(&db_pool, member.user.id)).await;
                        }
                    }
                }

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                ctx.set_presence(
                    Some(serenity::ActivityData::watching("you")),
                    serenity::OnlineStatus::Idle,
                );

                Ok(Data {
                    config,
                    db_pool,
                    study_states: HashMap::new().into()
                })
            })
        })
        .options(options)
        .build();

    let token =
        std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN environment variable");

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILD_VOICE_STATES
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await.expect("Error creating client");

    info!("Starting Discord client...");
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }

    Ok(())
}
