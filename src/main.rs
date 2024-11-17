mod prelude;
mod commands;

use core::panic;
use dotenv::dotenv;
use log::{error, info};
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use sqlx::sqlite::SqlitePoolOptions;
use std::{sync::Arc, time::Duration};

pub struct Data {
    db_pool: sqlx::SqlitePool
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!("Error in command: `{}`: {:?}", ctx.command().name, error);

            ctx.defer_ephemeral().await;
            ctx.send(
                ctx.reply_builder(CreateReply::default().ephemeral(true).content(format!(
                    "Error while running command `{}`:\n>>> {}",
                    ctx.command().name,
                    error
                ))),
            )
            .await;
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

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("SQLITE_CONNSTR").expect("Missing SQLITE_CONNSTR environment variable"))
        .await?;

    let options = poise::FrameworkOptions {
        commands: vec![commands::star::star()],

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
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                info!("Event handler: {:?}", event.snake_case_name());
                Ok(())
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                info!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                ctx.set_presence(
                    Some(serenity::ActivityData::watching("you")),
                    serenity::OnlineStatus::Idle,
                );

                Ok(Data {
                    db_pool: pool
                })
            })
        })
        .options(options)
        .build();

    let token =
        std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN environment variable");

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Error creating client");

    info!("Starting Discord client...");
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }

    Ok(())
}
