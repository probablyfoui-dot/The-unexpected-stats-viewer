// crates/stats-bot/src/commands/mod.rs — Command dispatch

// ---- Module declarations ---- //

pub mod stats;

// ---- Imports ---- //

use std::sync::Arc;

use anyhow::Result;
use serenity::all::*;

use crate::framework::BotData;

// ---- all_commands() ---- //

pub fn all_commands() -> Vec<CreateCommand<'static>> {
    vec![
        stats::bedwars::register(),
        stats::skywars::register(),
        stats::hypixel::register(),
        stats::quent::register(),
        stats::link::register(),
        stats::link::unlink::register(),
        stats::sync::register(),
    ]
}

// ---- dispatch_command() ---- //

pub async fn dispatch_command(
    ctx: &Context,
    cmd: &CommandInteraction,
    data: &Arc<BotData>,
) -> Result<()> {
    match cmd.data.name.as_str() {
        "bedwars" => stats::bedwars::run(ctx, cmd, data).await,
        "skywars" => stats::skywars::run(ctx, cmd, data).await,
        "hypixel" => stats::hypixel::run(ctx, cmd, data).await,
        "quent"   => stats::quent::run(ctx, cmd, data).await,
        "link"    => stats::link::run(ctx, cmd, data).await,
        "unlink"  => stats::link::unlink::run(ctx, cmd, data).await,
        "sync"    => stats::sync::run(ctx, cmd, data).await,
        _         => Ok(()),
    }
}

// ---- dispatch_component() ---- //

pub async fn dispatch_component(
    ctx: &Context,
    comp: &ComponentInteraction,
    data: &Arc<BotData>,
) -> Result<()> {
    let id = comp.data.custom_id.as_str();
    match id {
        id if id.starts_with("bw_mode:") => stats::bedwars::handle_mode_switch(ctx, comp, data).await,
        id if id.starts_with("sw_mode:") => stats::skywars::handle_mode_switch(ctx, comp, data).await,
        _ => Ok(()),
    }
}

// ---- dispatch_autocomplete() ---- //

pub async fn dispatch_autocomplete(
    ctx: &Context,
    aut: &CommandInteraction,
) -> Result<()> {
    match aut.data.name.as_str() {
        "quent"   => stats::quent::autocomplete(ctx, aut).await,
        _         => Ok(()),
    }
}
