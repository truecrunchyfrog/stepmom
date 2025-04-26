use charming::{component::{Axis, Title}, series::Line, Chart, ImageRenderer};
use humantime::format_duration;
use num_format::{Locale, ToFormattedString};
use poise::serenity_prelude::{CacheHttp, CreateAttachment, Member, Mentionable, ReactionType, RoleId};
use rand::{thread_rng, RngCore};
use serde::Deserialize;

use crate::{booster::Booster, charts::{render_chart_to_attachment, user_owned_themes, ChartTheme}, coins::add_coins, DbConn};

#[derive(Deserialize)]
pub enum Product {
    Coins(u64),
    Booster(Booster),
    Role { name: String, role_id: RoleId },
    ChartTheme(ChartTheme)
}

impl std::fmt::Display for Product {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}",
            match self {
                Self::Coins(amount) =>
                format!(
                    "{} coins",
                    amount.to_formatted_string(&Locale::en)),
                Self::Booster(Booster { multiplier, expiration }) =>
                format!(
                    "{}x booster (expires in {})",
                    *multiplier as f64 / 100.0,
                    format_duration(*expiration)),
                Self::Role { name, .. } => name.to_string(),
                Self::ChartTheme(theme) => format!("{:?} (theme)", theme)
            }
        )
    }
}

#[derive(PartialEq)]
pub enum ProductMemberStatus {
    Available,
    Unavailable(ProductMemberUnavailableReason)
}

#[derive(PartialEq)]
pub enum ProductMemberUnavailableReason {
    Owned,
    Other(String)
}

impl Product {
    pub async fn member_status(&self, conn: DbConn<'_>, member: &Member) -> anyhow::Result<ProductMemberStatus> {
        use ProductMemberStatus::*;
        use ProductMemberUnavailableReason::*;

        if member.user.bot {
            return Ok(Unavailable(Other("User is a bot.".to_string())));
        }

        Ok(match self {
            Self::Role { role_id, .. } => if member.roles.contains(role_id) { Unavailable(Owned) } else { Available },
            Self::ChartTheme(chart_theme) =>
            if user_owned_themes(conn, member.user.id).await?.contains(chart_theme) { Unavailable(Owned) } else { Available },
            _ => Available
        })
    }

    pub async fn give_to_member(&self, conn: DbConn<'_>, http: impl CacheHttp, member: &Member) -> anyhow::Result<()> {
        if let ProductMemberStatus::Unavailable(reason) = self.member_status(conn, member).await? {
            anyhow::bail!(
                "Unable to give product: {}",
                match reason {
                    ProductMemberUnavailableReason::Owned => "Already owned.".to_string(),
                    ProductMemberUnavailableReason::Other(reason) => reason,
                }
            );
        };

        match self {
            Self::Coins(amount) => add_coins(conn, member.user.id, *amount).await?,
            Self::Booster(Booster { multiplier, expiration }) => {
                let multiplier = *multiplier as i64;
                let expiration = expiration.as_secs() as i64;
                let uid = i64::from(member.user.id);
                sqlx::query!("
                INSERT INTO boosters
                VALUES (NULL, (SELECT id FROM users WHERE uid = $1), $2, UNIXEPOCH() + $3)
                ", uid, multiplier, expiration)
                    .execute(&mut *conn)
                    .await?;
            }
            Self::Role { role_id, .. } => member.add_role(http.http(), role_id).await?,
            Self::ChartTheme(theme) => {
                let theme = *theme as i64;
                let uid = i64::from(member.user.id);
                sqlx::query!("
                INSERT INTO owned_chart_themes
                VALUES (NULL, $1, (SELECT id FROM users WHERE uid = $2))
                ", theme, uid)
                    .execute(&mut *conn)
                    .await?;
            }
        }

        Ok(())
    }

    pub fn emoji(&self) -> ReactionType {
        match self {
            Self::Coins(_) => ReactionType::Unicode("💰".to_string()),
            Self::Booster(_) => ReactionType::Unicode("🚀".to_string()),
            Self::Role { .. } => ReactionType::Unicode("🏷️".to_string()),
            Self::ChartTheme(_) => ReactionType::Unicode("📊".to_string())
        }
    }

    pub fn fields(&self) -> Vec<(String, String)> {
        match self {
            Self::Booster(Booster { multiplier, expiration }) => vec![
                ("Multiplier".to_string(), format!("{}x", *multiplier as f64 / 100.0)),
                ("Expiration".to_string(), format_duration(*expiration).to_string())
            ],
            Self::Role { role_id, .. } => vec![
                ("Role".to_string(), role_id.mention().to_string())
            ],
            _ => Vec::new()
        }
    }

    pub fn get_attachment(&self) -> Option<CreateAttachment> {
        match self {
            Self::ChartTheme(theme) => render_chart_to_attachment(
                &mut ImageRenderer::new(1024, 512).theme((*theme).into()),
                &Chart::new()
                    .title(Title::new().text(self.to_string()))
                    .x_axis(
                        Axis::new()
                            .name("Time")
                            .data(vec!["1", "2", "3", "4", "5"]))
                    .y_axis(
                        Axis::new()
                            .name("Time")
                            .data(vec!["1", "2", "3", "4", "5"])),
                    // .series(Line::new()
                    //     .data(vec![1, 2, 3, 4, 5])),
                Some(&format!("{}.png", thread_rng().next_u32()))).ok(),
            _ => None
        }
    }
}
