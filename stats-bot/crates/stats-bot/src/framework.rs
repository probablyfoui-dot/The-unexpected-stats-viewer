// crates/stats-bot/src/framework.rs — Discord bot framework

// ---- Imports ---- //

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serenity::all::*;
use serenity::async_trait;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::commands;

const LINKS_FILE: &str = "links.json";

// ── Link storage ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEntry {
    pub minecraft_name: String,
    pub minecraft_uuid: String,
}

fn load_links_from_disk() -> HashMap<String, LinkEntry> {
    std::fs::read_to_string(LINKS_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_links_to_disk(links: &HashMap<String, LinkEntry>) {
    if let Ok(json) = serde_json::to_string_pretty(links) {
        let _ = std::fs::write(LINKS_FILE, &json);
    }
}

// ---- BotData ---- //

// ── Shared bot state ──────────────────────────────────────────────────────────

pub struct BotData {
    pub bedwars_cache: Mutex<HashMap<String, crate::commands::stats::bedwars::BedwarsCache>>,
    pub skywars_cache: Mutex<HashMap<String, crate::commands::stats::skywars::SkywarsCache>>,
    pub links: Mutex<HashMap<String, LinkEntry>>,
}

impl BotData {
    pub fn new() -> Self {
        let links = load_links_from_disk();
        info!("Loaded {} linked account(s)", links.len());
        Self {
            bedwars_cache: Mutex::new(HashMap::new()),
            skywars_cache: Mutex::new(HashMap::new()),
            links: Mutex::new(links),
        }
    }

    pub async fn set_link(&self, discord_id: &str, entry: LinkEntry) {
        let mut links = self.links.lock().await;
        links.insert(discord_id.to_string(), entry);
        save_links_to_disk(&links);
    }

    pub async fn remove_link(&self, discord_id: &str) -> bool {
        let mut links = self.links.lock().await;
        let removed = links.remove(discord_id).is_some();
        if removed {
            save_links_to_disk(&links);
        }
        removed
    }

    pub async fn get_link(&self, discord_id: &str) -> Option<LinkEntry> {
        self.links.lock().await.get(discord_id).cloned()
    }
}

// ---- Handler impl ---- //

// ── Event handler ─────────────────────────────────────────────────────────────

pub struct Handler {
    pub data: Arc<BotData>,
}

impl Handler {
    pub fn new(data: Arc<BotData>) -> Self {
        Self { data }
    }

    // ---- handle_interaction() ---- //

    async fn handle_interaction(&self, ctx: &Context, interaction: Interaction) {
        let result = match &interaction {
            Interaction::Command(cmd) => {
                commands::dispatch_command(ctx, cmd, &self.data).await
            }
            Interaction::Component(comp) => {
                commands::dispatch_component(ctx, comp, &self.data).await
            }
            Interaction::Autocomplete(aut) => {
                commands::dispatch_autocomplete(ctx, aut).await
            }
            _ => return,
        };

        if let Err(e) = result {
            error!("Interaction error: {e}");
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn dispatch(&self, ctx: &Context, event: &FullEvent) {
        match event {
            // ---- Ready handler ---- //

            FullEvent::Ready { data_about_bot, .. } => {
                info!("Logged in as {}", data_about_bot.user.name);

                let commands = commands::all_commands();
                info!("Sending {} commands to Discord", commands.len());

                match Command::set_global_commands(&ctx.http, &commands).await {
                    Ok(cmds) => info!("Registered {} global commands", cmds.len()),
                    Err(e)   => error!("Failed to register global commands: {e}"),
                }

                for guild in &data_about_bot.guilds {
                    match ctx.http.create_guild_commands(guild.id, &commands).await {
                        Ok(_) => info!("Synced commands to guild {}", guild.id),
                        Err(e) => error!("Failed to sync commands to guild {}: {e}", guild.id),
                    }
                }
                info!("Synced commands to {} guild(s)", data_about_bot.guilds.len());
            }
            FullEvent::InteractionCreate { interaction, .. } => {
                self.handle_interaction(ctx, interaction.clone()).await;
            }
            // ---- Message handler ---- //

            FullEvent::Message { new_message } => {
                let msg = new_message;
                if msg.author.bot() || msg.content != "!sync" {
                    return;
                }
                if let Err(e) = commands::stats::sync::run_prefix(ctx, msg).await {
                    error!("!sync error: {e}");
                }
            }
            _ => {}
        }
    }
}
