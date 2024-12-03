use crate::{prelude::{create_message_ref, take_coins, ActOnUser}, Error, StarCost};
use poise::{serenity_prelude::{self as serenity, futures::future::join_all, CacheHttp, ChannelId, CreateAllowedMentions, CreateAttachment, CreateMessage, FutureExt, Mentionable, MessageId}, Modal};

use super::ApplicationContext;

#[derive(Modal, Debug)]
#[name = "Star message"]
struct StarModal {
    #[name = "Cost"]
    cost: String
}

fn message_starring_cost(star_cost_config: &StarCost, message: &serenity::Message) -> u64 {
    let content_length = message.content.len();
    let attachments_length = message.attachments.len();

    star_cost_config.base +
        (content_length as f64 * star_cost_config.per_character) as u64 +
        attachments_length as u64 * star_cost_config.per_attachment
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
            WHERE source_msg.message_id = $1
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

    let starboard_channel = ctx.http().get_channel(
        ChannelId::new(ctx.data.config.channels.starboard_channel)).await?
        .guild().unwrap();

    if message.channel_id == starboard_channel.id {
        return Err(Error::from("Messages cannot be starred in this channel."))
    }

    let cost = message_starring_cost(&ctx.data.config.star_cost, &message);

    if let Some(data) = poise::modal::execute_modal(ctx, Some(StarModal { cost: cost.to_string() }), None).await? {
        if data.cost.parse::<u64>().unwrap_or(0) != cost {
            return Err(Error::from("Cost did not match, and was probably changed by user. Canceled."))
        }

        take_coins(
            &ActOnUser(&ctx.data.db_pool, ctx.author().id),
            cost as u64,
            "message starring",
            None).await?;

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

            /*.embeds(message.embeds.iter().map(|e| {
                let ce = CreateEmbed::new();

                if let Some(a) = &e.author {
                    let ca = CreateEmbedAuthor::new(&a.name);

                    if let Some(url) = &a.url { &ca.url(url); }
                    if let Some(icon_url) = &a.icon_url { &ca.icon_url(icon_url); }

                    ce.author(ca);
                }

                if let Some(f) = &e.footer {
                    let cf = CreateEmbedFooter::new(&f.text);

                    if let Some(icon_url) = &f.icon_url { &cf.icon_url(icon_url); }

                    ce.footer(cf);
                }

                if let Some(description) = &e.description { &ce.description(description); }

                if let Some(color) = &e.colour { &ce.color(*color); }

                if let Some(url) = &e.url { &ce.url(url); }

                if let Some(title) = &e.title { &ce.title(title); }

                if let Some(thumbnail) = &e.thumbnail { &ce.thumbnail(thumbnail.url); }

                if let Some(timestamp) = &e.timestamp { &ce.timestamp(timestamp); }

                for field in &e.fields {
                    ce.field(&field.name, &field.value, field.inline);
                }

                if let Some(image) = &e.image { ce.image(image.url); }

                ce
            }).collect())*/

            .files(join_all(message.attachments.iter().map(|a| {
                CreateAttachment::url(ctx.http(), &a.url)
            })).await.into_iter().flatten())

            .allowed_mentions(CreateAllowedMentions::new())
        ).await?;

        let source_id = create_message_ref(&ctx.data.db_pool, &message).await;
        let repost_id = create_message_ref(&ctx.data.db_pool, &repost).await;

        let user_id = i64::from(ctx.author().id);

        sqlx::query!("
            INSERT INTO starred_messages
            VALUES ($2, $3, (SELECT id FROM users WHERE uid = $1))
            ", user_id, source_id, repost_id)
            .execute(&ctx.data.db_pool)
            .await?;
    }

    Ok(())
}
