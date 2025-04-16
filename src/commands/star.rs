use crate::{coins::take_coins, messaging::create_message_ref, StarringConfig};
use poise::{serenity_prelude::{self as serenity, futures::future::join_all, ChannelId, CreateAllowedMentions, CreateAttachment, CreateMessage, Mentionable, MessageId}, Modal};

use super::ApplicationContext;

#[derive(Modal, Debug)]
#[name = "Star message"]
struct StarModal {
    #[name = "Cost"]
    cost: String
}

fn message_starring_cost(starring_config: &StarringConfig, message: &serenity::Message) -> u64 {
    let content_length = message.content.len();
    let attachments_length = message.attachments.len();

    starring_config.base +
        (content_length as f64 * starring_config.per_character) as u64 +
        attachments_length as u64 * starring_config.per_attachment
}

#[poise::command(context_menu_command = "Star!", guild_only)]
pub async fn star(
    ctx: ApplicationContext<'_>,
    message: serenity::Message
) -> anyhow::Result<()> {
    let msg_id = i64::from(message.id);

    let existing_star_entry = sqlx::query!("
            SELECT
                repost_msg.message_id AS repost_mid,
                repost_msg.channel_id AS repost_cid
            FROM starred_messages
            JOIN message_refs AS source_msg ON source_id = source_msg.id
            JOIN message_refs AS repost_msg ON repost_id = repost_msg.id
            WHERE source_msg.message_id = $1
        ", msg_id)
        .fetch_optional(&ctx.data.db_pool).await?;

    if let Some(starred_message) = existing_star_entry {
        let starboard_message = {
            ctx.http()
                .get_message(
                    ChannelId::new(starred_message.repost_cid as u64),
                    MessageId::new(starred_message.repost_mid as u64))
                .await
                .or(Err(anyhow::anyhow!("Already starred, but cannot find message.")))?
                .to_owned()
        };

        anyhow::bail!("This message has already been starred: {}",
            starboard_message.link())
    }

    let starboard_channel = ctx.http().get_channel(
        ctx.data.config.channels.starboard).await?
        .guild().unwrap();

    anyhow::ensure!(message.channel_id == starboard_channel.id, "Messages cannot be starred in this channel.");

    let cost = message_starring_cost(&ctx.data.config.star_cost, &message);

    if let Some(data) = poise::modal::execute_modal(ctx, Some(StarModal { cost: cost.to_string() }), None).await? {
        anyhow::ensure!(data.cost.parse() == Ok(cost), "Cost did not match, and was probably changed by user. Canceled.");

        let mut tx = ctx.data.db_pool.begin().await?;

        take_coins(&mut tx, ctx.author().id, cost, "message starring".to_string(), None).await?;

        let repost = starboard_channel.send_message(ctx.http(), CreateMessage::new()
            .content(format!("
-# originally posted by {} in {} <t:{}:R>
-# starred by {}\n
{}
                ",
                message.author.mention(),
                message.link(),
                message.edited_timestamp.unwrap_or(message.timestamp).unix_timestamp(),
                ctx.author().mention(),
                message.content
            ))

            .files(join_all(
                message.attachments.iter()
                    .map(|a| CreateAttachment::url(ctx.http(), &a.url))
            ).await.into_iter().collect::<Result<Vec<_>, _>>()?)

            .allowed_mentions(CreateAllowedMentions::new())
        ).await?;

        let source_id = create_message_ref(&mut tx, &message).await?;
        let repost_id = create_message_ref(&mut tx, &repost).await?;

        let user_id = i64::from(ctx.author().id);

        sqlx::query!("
        INSERT INTO starred_messages
        VALUES ($2, $3, (SELECT id FROM users WHERE uid = $1), $4)
        ", user_id, source_id, repost_id, message.content)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
    }

    Ok(())
}
