// crates/stats-bot/src/api.rs — Hypixel API client

// ---- Imports ---- //

use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::Value;

const MOJANG_API: &str = "https://api.mojang.com/users/profiles/minecraft/";
const HYPIXEL_API: &str = "https://api.hypixel.net/v2/player";
const HYPIXEL_GUILD_API: &str = "https://api.hypixel.net/v2/guild";

const MAX_RETRIES: u32 = 3;

// ---- HTTP client ---- //

fn http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("stats-bot/0.1")
            .build()
            .expect("Failed to create HTTP client")
    })
}

fn api_key() -> Result<String> {
    std::env::var("HYPIXEL_API_KEY").map_err(|_| anyhow!("HYPIXEL_API_KEY must be set"))
}

pub struct PlayerIdentity {
    pub uuid: String,
    pub name: String,
}

pub async fn resolve_player(input: &str) -> Result<PlayerIdentity> {
    let trimmed = input.replace('-', "");
    if trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(PlayerIdentity { uuid: trimmed, name: input.to_string() });
    }

    if input.len() > 16 || !input.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(anyhow!("Invalid player name: `{input}`"));
    }

    let url = format!("{MOJANG_API}{input}");
    let body = retry(|| async {
        let resp = http_client().get(&url).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(anyhow!("Player not found: {input}"));
        }
        resp.json::<Value>().await.map_err(Into::into)
    })
    .await?;

    let uuid = body["id"].as_str()
        .ok_or_else(|| anyhow!("No UUID in Mojang response"))?.to_string();
    let name = body["name"].as_str().unwrap_or(input).to_string();
    Ok(PlayerIdentity { uuid, name })
}

// ---- fetch_hypixel_player() ---- //

pub async fn fetch_hypixel_player(uuid: &str) -> Result<Value> {
    let key = api_key()?;
    let mut body = retry(|| async {
        http_client()
            .get(HYPIXEL_API)
            .query(&[("key", key.as_str()), ("uuid", uuid)])
            .send()
            .await?
            .json::<Value>()
            .await
            .map_err(Into::into)
    })
    .await?;

    if body["success"].as_bool() != Some(true) {
        return Err(anyhow!("Hypixel API error: {}", body["cause"].as_str().unwrap_or("unknown")));
    }

    let player = match body.get_mut("player") {
        Some(v) => std::mem::take(v),
        None => Value::Null,
    };
    if player.is_null() {
        Err(anyhow!("Player has no data"))
    } else {
        Ok(player)
    }
}

// ---- fetch_guild_by_player() ---- //

pub async fn fetch_guild_by_player(uuid: &str) -> Option<hypixel::GuildInfo> {
    let key = api_key().ok()?;
    let body: Value = http_client()
        .get(HYPIXEL_GUILD_API)
        .query(&[("key", key.as_str()), ("player", uuid)])
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    if body["success"].as_bool() != Some(true) { return None; }
    let guild_val = body.get("guild")?;
    if guild_val.is_null() { return None; }
    Some(hypixel::GuildInfo::from_guild(guild_val, uuid))
}

async fn retry<F, Fut, T>(f: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_err = None;
    for attempt in 0..MAX_RETRIES {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if attempt + 1 < MAX_RETRIES {
                    tokio::time::sleep(Duration::from_millis(500 * 2u64.pow(attempt))).await;
                }
                last_err = Some(e);
            }
        }
    }
    Err(anyhow!("Request failed after {MAX_RETRIES} retries: {:?}", last_err.unwrap()))
}
