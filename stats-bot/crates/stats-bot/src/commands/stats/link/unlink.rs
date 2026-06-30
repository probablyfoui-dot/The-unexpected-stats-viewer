// crates/stats-bot/src/commands/stats/link/unlink.rs — /unlink command

// ---- Imports ---- //

use std::sync::Arc;

use anyhow::Result;
use serenity::all::*;

use crate::framework::BotData;

// ---- register() ---- //

pub fn register() -> CreateCommand<'static> {
    CreateCommand::new("unlink")
        .description("Remove your Discord-Minecraft account link")
}

// ---- run() ---- //

pub async fn run(ctx: &Context, cmd: &CommandInteraction, data: &Arc<BotData>) -> Result<()> {
    let author_id = cmd.user.id.to_string();

    let removed = data.remove_link(&author_id).await;

    let embed = if removed {
        CreateEmbed::new()
            .title("Account Unlinked")
            .description("Your link has been removed.")
            .color(0x55FF55)
    } else {
        CreateEmbed::new()
            .title("Not Linked")
            .description("You don't have a linked account.")
            .color(0xFF5555)
    };

    cmd.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().embed(embed),
        ),
    ).await?;
    Ok(())
}
