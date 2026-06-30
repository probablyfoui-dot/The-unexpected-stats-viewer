// crates/stats-bot/src/commands/stats/skywars/mod.rs — /skywars command

// ---- Imports ---- //

pub mod cards;

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use hypixel::{SkywarsStats, SwMode, extract_skywars_stats};
use serenity::all::*;

use crate::api::{fetch_guild_by_player, fetch_hypixel_player};
use crate::commands::stats::{encode_png, resolve_player_or_link};
use crate::framework::BotData;

use cards::render_skywars;

pub struct SkywarsCache {
    pub stats: SkywarsStats,
    pub modes: Vec<SwMode>,
    pub last_use: Instant,
}

const CACHE_TTL_SECS: u64 = 120;

const MODE_CHOICES: &[(&str, SwMode, &str)] = &[
    ("Solos",   SwMode::Solos,   "solos"),
    ("Doubles", SwMode::Doubles, "doubles"),
    ("Mega",    SwMode::Mega,    "mega"),
    ("Labs",    SwMode::Labs,    "labs"),
];

fn make_dropdown(cache_key: &str, selected: &[SwMode]) -> CreateSelectMenu<'static> {
    let options: Vec<CreateSelectMenuOption> = MODE_CHOICES
        .iter()
        .map(|(label, mode, value)| {
            CreateSelectMenuOption::new(*label, format!("{}:{}", value, cache_key))
                .default_selection(selected.contains(mode))
        })
        .collect();

    CreateSelectMenu::new(
        format!("sw_mode:{cache_key}"),
        CreateSelectMenuKind::String { options: options.into() },
    )
    .placeholder("Select mode(s)…")
    .min_values(1)
    .max_values(4)
}

fn parse_values(values: &[String]) -> Option<(String, Vec<SwMode>)> {
    let mut cache_key = None;
    let mut modes = Vec::new();
    for v in values {
        if let Some((mode_str, key)) = v.split_once(':') {
            cache_key = Some(key.to_string());
            if let Some(m) = SwMode::from_str(mode_str) {
                modes.push(m);
            }
        }
    }
    let key = cache_key?;
    if modes.is_empty() { return None; }
    Some((key, modes))
}

// ---- register() ---- //

pub fn register() -> CreateCommand<'static> {
    CreateCommand::new("skywars")
        .description("Hypixel SkyWars stats for a player")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "player", "Player name or UUID (leave empty to use linked account)")
                .required(false),
        )
}

// ---- run() ---- //

pub async fn run(ctx: &Context, cmd: &CommandInteraction, data: &Arc<BotData>) -> Result<()> {
    cmd.defer(&ctx.http).await?;

    let input = cmd.data.options.first().and_then(|o| o.value.as_str()).map(String::from);
    let author_id = cmd.user.id.to_string();
    let identity = match resolve_player_or_link(input, &author_id, data.as_ref()).await {
        Ok(id) => id,
        Err(e) => {
            return crate::commands::stats::send_error(
                ctx, cmd, "Player Not Found", &e.to_string(),
            ).await;
        }
    };

    let (player_json, guild_info) = tokio::join!(
        fetch_hypixel_player(&identity.uuid),
        fetch_guild_by_player(&identity.uuid),
    );

    let player_json = match player_json {
        Ok(p) => p,
        Err(_) => {
            return crate::commands::stats::send_error(
                ctx, cmd, "API Error",
                "Could not fetch Hypixel data. Please try again later.",
            ).await;
        }
    };

    let stats = match extract_skywars_stats(&identity.name, &player_json, guild_info) {
        Some(s) => s,
        None => {
            return crate::commands::stats::send_error(
                ctx, cmd,
                &format!("{}'s SkyWars Stats", identity.name),
                "This player has no SkyWars stats.",
            ).await;
        }
    };

    let modes = vec![
        SwMode::Solos, SwMode::Doubles, SwMode::Mega, SwMode::Labs,
    ];
    let png = encode_png(&render_skywars(&stats, &modes))?;
    let cache_key = cmd.id.to_string();

    {
        let mut cache = data.skywars_cache.lock().await;
        cache.insert(cache_key.clone(), SkywarsCache { stats, modes: modes.clone(), last_use: Instant::now() });
    }

    let dropdown = make_dropdown(&cache_key, &modes);

    cmd.edit_response(
        &ctx.http,
        EditInteractionResponse::new()
            .new_attachment(CreateAttachment::bytes(png, "skywars.png"))
            .components(vec![CreateComponent::ActionRow(CreateActionRow::SelectMenu(dropdown))]),
    ).await?;

    Ok(())
}

// ---- handle_mode_switch() ---- //

pub async fn handle_mode_switch(
    ctx: &Context,
    comp: &ComponentInteraction,
    data: &Arc<BotData>,
) -> Result<()> {
    let values = match &comp.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values.clone(),
        _ => return Ok(()),
    };

    let (cache_key, modes) = match parse_values(&values) {
        Some(v) => v,
        None => {
            comp.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("Invalid selection."),
                ),
            ).await?;
            return Ok(());
        }
    };

    let png = {
        let mut cache = data.skywars_cache.lock().await;
        let entry = match cache.get_mut(&cache_key) {
            Some(e) => e,
            None => {
                comp.create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("This card has expired — please run `/skywars` again."),
                    ),
                ).await?;
                return Ok(());
            }
        };
        if entry.last_use.elapsed().as_secs() >= CACHE_TTL_SECS {
            cache.remove(&cache_key);
            comp.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("This card has expired — please run `/skywars` again."),
                ),
            ).await?;
            return Ok(());
        }
        entry.last_use = Instant::now();
        entry.modes = modes.clone();
        encode_png(&render_skywars(&entry.stats, &modes))?
    };

    let dropdown = make_dropdown(&cache_key, &modes);

    comp.create_response(
        &ctx.http,
        CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .add_file(CreateAttachment::bytes(png, "skywars.png"))
                .components(vec![CreateComponent::ActionRow(CreateActionRow::SelectMenu(dropdown))]),
        ),
    ).await?;

    Ok(())
}
