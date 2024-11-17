use crate::{Context, Error, Data};
use log::info;
use poise::{serenity_prelude::{self as serenity, futures::future::join_all, CacheHttp, ChannelId, GuildChannel, Http, Mention, Message, MessageBuilder, MessageId}, Modal};
use crate::prelude::*;

type ApplicationContext<'a> = poise::ApplicationContext<'a, Data, Error>;

#[derive(Modal, Debug)]
#[name = "Star message"]
struct StarModal {
    #[name = "Cost"]
    cost: String
}

fn message_starring_cost(message: serenity::Message) -> usize {
    let content_length = message.content.len();
    let attachments_length = message.attachments.len();

    300 +
        (content_length as f64 * 1.0/3.0) as usize +
        attachments_length * 100
}

#[poise::command(context_menu_command = "Star!", guild_only)]
pub async fn star(
    ctx: ApplicationContext<'_>,
    message: serenity::Message
) -> Result<(), Error> {
    struct StarredMessage {
        repost_mid: i64,
        repost_cid: i64,
    }

    let msg_id = i64::from(message.id);
    let existing_star_entry = sqlx::query_as!(
        StarredMessage,
        r#"
            SELECT
                repost_msg.message_id AS repost_mid,
                repost_msg.channel_id AS repost_cid
            FROM starred_messages
            JOIN message_refs AS source_msg ON source_id = source_msg.id
            JOIN message_refs AS repost_msg ON repost_id = repost_msg.id
            WHERE source_id = $1
        "#,
        msg_id)
        .fetch_optional(&ctx.data.db_pool).await?;
    if let Some(starred_message) = existing_star_entry {
        let starboard_message = {
            ctx.http()
                .get_message(
                    ChannelId::new(starred_message.repost_cid as u64),
                    MessageId::new(starred_message.repost_mid as u64))
                .await
                .or(Err(Error::from("Already starred, but cannot find message.")))?
                .to_owned()
        };

        return Err(Error::from(
                format!(
                    "This message has already been starred: {}",
                    starboard_message.link()
                )))
    }

    let cost = message_starring_cost(message);

    if let Some(data) = poise::modal::execute_modal(ctx, Some(StarModal { cost: cost.to_string() }), None).await? {
        if data.cost.parse::<usize>().unwrap_or(0) != cost {
            return Err(Error::from("Cost did not match, and was probably changed by user. Canceled."))
        }

        info!("star message here!");
    }

    Ok(())
}
