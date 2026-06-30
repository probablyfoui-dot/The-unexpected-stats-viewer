// crates/stats-bot/src/commands/stats/link/mod.rs — /link command

// ---- Module declarations ---- //

pub mod unlink;

// ---- Imports ---- //

use std::sync::Arc;

use anyhow::Result;
use serenity::all::*;

use crate::api::resolve_player;
use crate::framework::{BotData, LinkEntry};

// ---- Links storage ---- //

// ---- register() ---- //

pub fn register() -> CreateCommand<'static> {
    CreateCommand::new("link")
        .description("Link your Discord account to a Minecraft player")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "player", "Minecraft player to link")
                .required(false),
        )
}

// ---- run() ---- //

pub async fn run(ctx: &Context, cmd: &CommandInteraction, data: &Arc<BotData>) -> Result<()> {
    let author_id = cmd.user.id.to_string();

    cmd.defer(&ctx.http).await?;

    let input = cmd
        .data
        .options
        .first()
        .and_then(|o| o.value.as_str())
        .map(String::from);

    let embed = match input {
        Some(name) => {
            let identity = resolve_player(&name).await?;

            let entry = LinkEntry {
                minecraft_name: identity.name.clone(),
                minecraft_uuid: identity.uuid.clone(),
            };
            data.set_link(&author_id, entry).await;

            CreateEmbed::new()
                .title("Account Linked")
                .description(format!("Linked <@{}> to **{}**", author_id, identity.name))
                .color(0x55FF55)
        }
        None => {
            match data.get_link(&author_id).await {
                Some(link) => {
                    let msg = format!(
                        "You are linked to **{}** (`{}`).\nUse `/link <player>` to change, or `/unlink` to remove.",
                        link.minecraft_name, link.minecraft_uuid
                    );
                    CreateEmbed::new()
                        .title("Your Link")
                        .description(msg)
                        .color(0x55FFFF)
                }
                None => {
                    CreateEmbed::new()
                        .title("Not Linked")
                        .description("You don't have a linked account. Use `/link <player>` to link one.")
                        .color(0xFF5555)
                }
            }
        }
    };

    cmd.edit_response(&ctx.http, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}
