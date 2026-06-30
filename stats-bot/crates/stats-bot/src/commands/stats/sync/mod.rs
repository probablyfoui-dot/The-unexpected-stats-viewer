// crates/stats-bot/src/commands/stats/sync/mod.rs — /sync + !sync

// ---- Imports ---- //

use std::sync::Arc;

use anyhow::Result;
use serenity::all::*;

use crate::commands::all_commands;
use crate::framework::BotData;

// ---- register() ---- //

pub fn register() -> CreateCommand<'static> {
    CreateCommand::new("sync")
        .description("Sync global slash commands with Discord immediately")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

// ---- do_sync() ---- //

/// Shared sync logic — registers all commands and returns a result embed.
async fn do_sync(http: &Http) -> CreateEmbed<'_> {
    let cmds = all_commands();
    match Command::set_global_commands(http, &cmds).await {
        Ok(registered) => CreateEmbed::new()
            .title("Commands Synced")
            .description(format!(
                "Successfully registered **{}** command(s) with Discord.\nIt may take a few minutes for the changes to propagate.",
                registered.len()
            ))
            .color(0x55FF55),
        Err(e) => CreateEmbed::new()
            .title("Sync Failed")
            .description(format!("Failed to register commands: {e}"))
            .color(0xFF5555),
    }
}

// ---- run() ---- //

/// Slash command entrypoint: `/sync`
pub async fn run(ctx: &Context, cmd: &CommandInteraction, _data: &Arc<BotData>) -> Result<()> {
    cmd.defer(&ctx.http).await?;
    let embed = do_sync(&ctx.http).await;
    cmd.edit_response(&ctx.http, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}

// ---- run_prefix() ---- //

/// Prefix command entrypoint: `!sync`
pub async fn run_prefix(ctx: &Context, msg: &Message) -> Result<()> {
    let embed = do_sync(&ctx.http).await;
    msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().reference_message(msg).embed(embed))
        .await?;
    Ok(())
}
