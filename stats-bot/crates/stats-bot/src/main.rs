// crates/stats-bot/src/main.rs — Binary entrypoint

// ---- Imports ---- //

use anyhow::Result;
use dotenvy::dotenv;
use serenity::all::*;
use std::sync::Arc;
use tracing::info;

mod api;
mod commands;
mod framework;

use framework::{BotData, Handler};

// ---- Main function ---- //

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    render::init_canvas();

    let token = Token::from_env("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");
    let intents = GatewayIntents::GUILDS | GatewayIntents::MESSAGE_CONTENT;

    let data = Arc::new(BotData::new());
    let handler = Arc::new(Handler::new(data));

    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .await?;

    info!("Starting stats-bot…");
    client.start().await?;
    Ok(())
}
