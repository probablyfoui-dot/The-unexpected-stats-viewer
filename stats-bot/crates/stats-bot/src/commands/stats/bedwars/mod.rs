// crates/stats-bot/src/commands/stats/bedwars/mod.rs — /bedwars command

// ---- Imports ---- //

pub mod cards;

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use hypixel::{BedwarsPlayerStats, Mode, extract_bedwars_stats};
use serenity::all::*;

use crate::api::{fetch_guild_by_player, fetch_hypixel_player};
use crate::commands::stats::{encode_png, resolve_player_or_link};
use crate::framework::BotData;

use cards::render_bedwars;

// ── Cache entry ───────────────────────────────────────────────────────────────

pub struct BedwarsCache {
    pub stats: BedwarsPlayerStats,
    pub mode: Vec<Mode>,
    pub last_use: Instant,
}

const CACHE_TTL_SECS: u64 = 120;

// ── Mode choices (no Overall — it's produced by selecting all) ────────────────

const MODE_CHOICES: &[(&str, Mode, &str)] = &[
    // Core modes
    ("Solos",         Mode::Solos,         "solos"),
    ("Doubles",       Mode::Doubles,       "doubles"),
    ("Threes",        Mode::Threes,        "threes"),
    ("Fours",         Mode::Fours,         "fours"),
    ("4v4",           Mode::FourVFour,     "4v4"),
    // Dream modes
    ("Rush",          Mode::Rush,          "rush"),
    ("Ultimate",      Mode::Ultimate,      "ultimate"),
    ("Armed",         Mode::Armed,         "armed"),
    ("Lucky Blocks",  Mode::LuckyBlocks,   "lucky"),
    ("Voidless",      Mode::Voidless,      "voidless"),
    ("Swap",          Mode::Swap,          "swap"),
    ("Castle",        Mode::Castle,        "castle"),
    ("Underworld",    Mode::Underworld,    "underworld"),
    ("Totally Normal",Mode::TotallyNormal, "totallynormal"),
];

fn make_dropdown(cache_key: &str, selected: &[Mode]) -> CreateSelectMenu<'static> {
    let options: Vec<CreateSelectMenuOption> = MODE_CHOICES
        .iter()
        .map(|(label, mode, value)| {
            CreateSelectMenuOption::new(*label, format!("{}:{}", value, cache_key))
                .default_selection(selected.contains(mode))
        })
        .collect();

    CreateSelectMenu::new(
        format!("bw_mode:{cache_key}"),
        CreateSelectMenuKind::String { options: options.into() },
    )
    .placeholder("Select mode(s)…")
    .min_values(1)
    .max_values(5) // cap at 5 — Discord select menus have a 25-option display limit
}

fn parse_values(values: &[String]) -> Option<(String, Vec<Mode>)> {
    let mut cache_key = None;
    let mut modes = Vec::new();
    for v in values {
        if let Some((mode_str, key)) = v.split_once(':') {
            cache_key = Some(key.to_string());
            if let Some(m) = Mode::from_str(mode_str) {
                modes.push(m);
            }
        }
    }
    let key = cache_key?;
    if modes.is_empty() { return None; }
    Some((key, modes))
}

// ---- register() ---- //

// ── Command registration ──────────────────────────────────────────────────────

pub fn register() -> CreateCommand<'static> {
    CreateCommand::new("bedwars")
        .description("Hypixel BedWars stats for a player")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "player", "Player name or UUID (leave empty to use linked account)")
                .required(false),
        )
}

// ---- run() ---- //

// ── Command run ───────────────────────────────────────────────────────────────

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

    let stats = match extract_bedwars_stats(&identity.name, &player_json, guild_info) {
        Some(s) => s,
        None => {
            return crate::commands::stats::send_error(
                ctx, cmd,
                &format!("{}'s BedWars Stats", identity.name),
                "This player has no BedWars stats.",
            ).await;
        }
    };

    // Default: all 5 core modes combined
    let mode = vec![
        Mode::Solos, Mode::Doubles, Mode::Threes, Mode::Fours, Mode::FourVFour,
    ];
    let png = encode_png(&render_bedwars(&stats, &mode))?;
    let cache_key = cmd.id.to_string();

    {
        let mut cache = data.bedwars_cache.lock().await;
        cache.insert(cache_key.clone(), BedwarsCache { stats, mode: mode.clone(), last_use: Instant::now() });
    }

    let dropdown = make_dropdown(&cache_key, &mode);

    cmd.edit_response(
        &ctx.http,
        EditInteractionResponse::new()
            .new_attachment(CreateAttachment::bytes(png, "bedwars.png"))
            .components(vec![CreateComponent::ActionRow(CreateActionRow::SelectMenu(dropdown))]),
    )
    .await?;

    Ok(())
}

// ---- handle_mode_switch() ---- //

// ── Mode-switch handler ───────────────────────────────────────────────────────

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
        let mut cache = data.bedwars_cache.lock().await;
        let entry = match cache.get_mut(&cache_key) {
            Some(e) => e,
            None => {
                comp.create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("This card has expired — please run `/bedwars` again."),
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
                        .content("This card has expired — please run `/bedwars` again."),
                ),
            ).await?;
            return Ok(());
        }
        entry.last_use = Instant::now();
        entry.mode = modes.clone();
        encode_png(&render_bedwars(&entry.stats, &modes))?
    };

    let dropdown = make_dropdown(&cache_key, &modes);

    comp.create_response(
        &ctx.http,
        CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new()
                .add_file(CreateAttachment::bytes(png, "bedwars.png"))
                .components(vec![CreateComponent::ActionRow(CreateActionRow::SelectMenu(dropdown))]),
        ),
    ).await?;

    Ok(())
}
