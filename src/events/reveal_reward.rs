use poise::serenity_prelude::{ComponentInteraction, Context, CreateAllowedMentions, CreateInteractionResponse, CreateInteractionResponseMessage, MessageBuilder};

use crate::Data;

pub async fn reveal_reward(ctx: &Context, interaction: &ComponentInteraction, data: &Data, reward_id: i64) -> anyhow::Result<()> {
    let uid = i64::from(interaction.user.id);
    let reward = sqlx::query!("
    SELECT description, reason
    FROM rewards
    WHERE
        id = $1 AND
        user_id IN (SELECT id FROM users WHERE uid = $2)
    ", reward_id, uid)
        .fetch_optional(&data.db_pool)
        .await?
        .ok_or(anyhow::anyhow!("Invalid reward."))?;

    interaction.create_response(&ctx, CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
            .components(Vec::new()) // Remove "Reveal" button
            .allowed_mentions(CreateAllowedMentions::new())
            .content(
                MessageBuilder::new()
                .push_line(format!("-# Congratulations! You deserve this {}.", reward.reason))
                .push("## :partying_face: ")
                .push_line(reward.description)
                .build()))).await?;

    Ok(())
}
