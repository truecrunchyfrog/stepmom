use log::info;
use poise::serenity_prelude::{CacheHttp, ChannelId, Context, CreateInteractionResponse, CreateInteractionResponseMessage, Interaction, MessageId, UserId};
use regex::Regex;

use crate::{events::reveal_reward::reveal_reward, Data, Error};

use super::deduct_session::deduct_session;

pub async fn interaction_handler(ctx: &Context, data: &Data, interaction: &Interaction) -> Result<(), Error> {
    match interaction {
        Interaction::Component(component_interaction) => {
            let custom_id = &component_interaction.data.custom_id;
            info!("Component interaction: {}", custom_id);
            match component_interaction.data.custom_id.as_str() {
                "delete_guild_dm" => {
                    let mid = i64::from(component_interaction.message.id);

                    let dm_msgs_set_info = sqlx::query!("
                    SELECT users.uid AS user_id, guild_sent_dm_messages.msg_set_id
                    FROM message_refs
                    JOIN msg_set_items
                        ON message_refs.id = msg_set_items.message_ref_id
                    JOIN guild_sent_dm_messages
                        ON msg_set_items.msg_set_id = guild_sent_dm_messages.msg_set_id
                    JOIN users
                        ON user_id = users.id
                    WHERE message_id = $1
                    ", mid)
                        .fetch_optional(&data.db_pool)
                        .await?;

                    match dm_msgs_set_info {
                        Some(r) if UserId::new(r.user_id as u64) == component_interaction.user.id => {
                            let msgs_to_delete = sqlx::query!("
                            DELETE FROM message_refs
                            WHERE id IN (
                                SELECT message_ref_id
                                FROM msg_set_items
                                WHERE msg_set_id = $1
                            )
                            RETURNING channel_id, message_id
                            ", r.msg_set_id)
                                .fetch_all(&data.db_pool)
                                .await.unwrap();

                            for msg_info in msgs_to_delete {
                                let message = ctx.http().get_message(
                                    ChannelId::new(msg_info.channel_id as u64),
                                    MessageId::new(msg_info.message_id as u64)
                                ).await.unwrap();
                                message.delete(&ctx.http).await.unwrap();
                            }
                        }
                        _ => {
                            component_interaction.create_response(&ctx.http, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                    .ephemeral(true)
                                    .content("You do not own these messages."))).await.unwrap();
                        }
                    }
                }
                id => {
                    if let Some(c) = Regex::new(r"deduct_session_(\d+)").unwrap().captures(id) {
                        deduct_session(
                            ctx, component_interaction, data,
                            c[1].parse::<i64>()?
                        ).await?;
                    } else if let Some(c) = Regex::new(r"reveal_reward_(\d+)").unwrap().captures(id) {
                        reveal_reward(
                            ctx, component_interaction, data,
                            c[1].parse::<i64>()?
                        ).await?;
                    }
                }
            }
        }
        _ => ()
    }

    Ok(())
}
