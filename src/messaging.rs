use poise::serenity_prelude::{ButtonStyle, CacheHttp, CreateButton, CreateMessage, Mentionable, Message, User};

use crate::{Data, DbConn};

pub async fn create_message_ref(conn: DbConn<'_>, message: &Message) -> anyhow::Result<i64> {
    let cid = i64::from(message.channel_id);
    let mid = i64::from(message.id);
    Ok(sqlx::query!("
            INSERT INTO message_refs (channel_id, message_id)
            VALUES ($1, $2)",
            cid, mid)
        .execute(conn)
        .await?
        .last_insert_rowid())
}

pub async fn try_dm_or_in_guild(conn: DbConn<'_>, data: &Data, http: impl CacheHttp, user: &User, builder: CreateMessage) -> anyhow::Result<Message> {
    let dm_message = user.dm(&http, builder.clone()).await;

    match dm_message {
        Ok(sent_dm_msg) => Ok(sent_dm_msg),
        Err(_) => {
            let msg_set_id = sqlx::query!("INSERT INTO msg_sets VALUES (NULL)")
                .execute(&mut *conn)
                .await?
                .last_insert_rowid();

            let uid = i64::from(user.id);

            sqlx::query!("
            INSERT INTO guild_sent_dm_messages
            VALUES ((SELECT id FROM users WHERE uid = $1), $2)
                ", uid, msg_set_id)
                .execute(&mut *conn)
                .await?
                .last_insert_rowid();

            let channel = data.config.channels.dm_backup;

            let sent_guild_msg = channel
                .send_message(&http, builder)
                .await?;

            let info_msg = channel.send_message(&http, CreateMessage::new()
                .content(format!("{}\n-# We tried to send this message straight to you, but it could not be delivered!\n-# Enable **Direct Messages** under **Privacy Settings** to get these directly to you next time, privately.\n-# * This message is public, click `Delete` to delete it.", user.mention()))
                .button(
                    CreateButton::new("delete_guild_dm")
                    .label("Delete")
                    .style(ButtonStyle::Danger)
                )
                .reference_message(&sent_guild_msg)
            ).await?;

            for msg in vec![&sent_guild_msg, &info_msg] {
                let msg_ref_id = create_message_ref(&mut *conn, &msg).await?;
                sqlx::query!("
                INSERT INTO msg_set_items
                VALUES ($1, $2)
                ", msg_set_id, msg_ref_id)
                    .execute(&mut *conn)
                    .await?;
            }

            Ok(sent_guild_msg)
        }
    }
}
