// crates/stats-bot/src/commands/stats/mod.rs — Stats command utilities

// ---- Module declarations ---- //

pub mod bedwars;
pub mod hypixel;
pub mod link;
pub mod quent;
pub mod skywars;
pub mod sync;

// ---- Imports ---- //

use std::io::Cursor;

use anyhow::{Result, anyhow};
use image::RgbaImage;

use crate::api::{PlayerIdentity, resolve_player};
use crate::framework::BotData;

// ---- encode_png() ---- //

pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)?;
    Ok(buf.into_inner())
}

// ---- resolve_player_or_link() ---- //

/// Resolve a player from an optional input string. If None, use the author's
/// linked account. Returns Err if neither exists.
pub async fn resolve_player_or_link(
    input: Option<String>,
    author_id: &str,
    data: &BotData,
) -> Result<PlayerIdentity> {
    match input {
        Some(name) => resolve_player(&name).await,
        None => {
            let link = data.get_link(author_id).await
                .ok_or_else(|| anyhow!("No player specified and no linked account.\nUse `/link <player>` to link one, or provide a name."))?;
            Ok(PlayerIdentity {
                uuid: link.minecraft_uuid,
                name: link.minecraft_name,
            })
        }
    }
}

// ---- send_error() ---- //

pub async fn send_error(
    ctx: &serenity::all::Context,
    cmd: &serenity::all::CommandInteraction,
    title: &str,
    msg: &str,
) -> Result<()> {
    use serenity::all::*;
    cmd.edit_response(
        &ctx.http,
        EditInteractionResponse::new().embed(
            CreateEmbed::new()
                .title(title)
                .description(msg)
                .color(0xFF4040u32),
        ),
    )
    .await?;
    Ok(())
}
