// crates/stats-bot/src/commands/stats/hypixel/mod.rs — /hypixel command

// ---- Imports ---- //

use std::sync::Arc;
use anyhow::Result;
use image::RgbaImage;
use mctext::{MCText, NamedColor};
use serenity::all::*;
use serde_json::Value;

use render::canvas::{
    Align, BOX_BACKGROUND, CANVAS_BACKGROUND, Canvas, DrawContext, RoundedRect, Shape, TextBox,
};
use render::cards::{color_name_to_named, format_number, format_timestamp};
use render::{OutputType, Pose, Skin, render_skin};

use crate::api::{fetch_guild_by_player, fetch_hypixel_player};
use crate::commands::stats::{encode_png, resolve_player_or_link};
use crate::framework::BotData;
use hypixel::{calculate_network_level, extract_rank_prefix};

// ── Layout ────────────────────────────────────────────────────────────────────

const W: u32 = 800;
const CR: u32 = 18;
const GAP: u32 = 8;

const SKIN_W: u32 = 120;
const HEADER_H: u32 = 200;  // tall enough for skin body

const INFO_W: u32 = W - SKIN_W - GAP;
const COL_W: u32 = (INFO_W - GAP) / 2;
const BOX_H: u32 = 88;

const ROW1_Y: u32 = HEADER_H + GAP;
const ROW2_Y: u32 = ROW1_Y + BOX_H + GAP;
const BOTTOM_H: u32 = 64;
const ROW3_Y: u32 = ROW2_Y + BOX_H + GAP;
const H: u32 = ROW3_Y + BOTTOM_H;

// ---- Data ---- //

// ── Data ──────────────────────────────────────────────────────────────────────

struct ProfileData {
    display_name_mc: String,
    level: u32,
    level_frac: f64,
    guild_name: Option<String>,
    guild_tag: Option<String>,
    guild_tag_color: Option<String>,
    achievement_points: u64,
    karma: u64,
    first_login: Option<i64>,
    skin_bytes: Option<Vec<u8>>,
}

impl ProfileData {
    fn from_json(player: &Value, guild: Option<hypixel::GuildInfo>, skin_bytes: Option<Vec<u8>>) -> Self {
        let exp = player["networkExp"].as_f64().unwrap_or(0.0);
        let level_f = calculate_network_level(exp);
        let level = level_f.floor() as u32;
        let level_frac = level_f.fract();

        let display_name = player["displayname"].as_str().unwrap_or("Unknown");
        let prefix = extract_rank_prefix(player).unwrap_or_else(|| "§7".to_string());
        let display_name_mc = format!("{prefix}{display_name}");

        let (guild_name, guild_tag, guild_tag_color) = match guild {
            Some(g) => (g.name, g.tag, g.tag_color),
            None => (None, None, None),
        };

        Self {
            display_name_mc,
            level,
            level_frac,
            guild_name,
            guild_tag,
            guild_tag_color,
            achievement_points: player["achievementPoints"].as_u64().unwrap_or(0),
            karma: player["karma"].as_u64().unwrap_or(0),
            first_login: player["firstLogin"].as_i64(),
            skin_bytes,
        }
    }
}

// ---- Render ---- //

// ── Render ────────────────────────────────────────────────────────────────────

fn render_profile(data: &ProfileData) -> RgbaImage {
    Canvas::new(W, H)
        .background(CANVAS_BACKGROUND)
        .draw(0, 0,                    &HeaderSection { data })
        .draw(SKIN_W as i32 + GAP as i32, ROW1_Y as i32, &level_box(data))
        .draw((SKIN_W + GAP + COL_W + GAP) as i32, ROW1_Y as i32, &guild_box(data))
        .draw(SKIN_W as i32 + GAP as i32, ROW2_Y as i32, &achiev_box(data))
        .draw((SKIN_W + GAP + COL_W + GAP) as i32, ROW2_Y as i32, &karma_box(data))
        .draw(0, ROW3_Y as i32, &bottom_box(data))
        .build()
}

// ── Header: name + XP bar + skin body ────────────────────────────────────────

struct HeaderSection<'a> { data: &'a ProfileData }

impl Shape for HeaderSection<'_> {
    fn draw(&self, ctx: &mut DrawContext) {
        // Full-width background box
        RoundedRect::new(W, HEADER_H).corner_radius(CR).background(BOX_BACKGROUND).draw(ctx);

        let (cw, ch) = ctx.buffer.dimensions();

        // Skin body on the left
        if let Some(ref bytes) = self.data.skin_bytes {
            if let Ok(skin) = Skin::from_bytes(bytes) {
                let pose = Pose::default();
                let output_type = OutputType::full_body(SKIN_W, HEADER_H - 8);
                if let Ok(rendered) = render_skin(&skin, &pose, output_type) {
                    let skin_img = rendered.into_image();
                    let sx = ctx.x as u32 + 4;
                    let sy = ctx.y as u32 + 4;
                    for py in 0..skin_img.height() {
                        for px in 0..skin_img.width() {
                            let sp = *skin_img.get_pixel(px, py);
                            if sp[3] > 0 {
                                let dx = sx + px;
                                let dy = sy + py;
                                if dx < cw && dy < ch {
                                    ctx.buffer.put_pixel(dx, dy, sp);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Name — to the right of the skin
        let name_text = MCText::parse(&self.data.display_name_mc);
        let name_x = (ctx.x + SKIN_W as i32 + GAP as i32 + 4) as f32;
        ctx.renderer.draw(ctx.buffer.as_mut(), cw, ch,
            name_x, (ctx.y + 14) as f32, &name_text, 2.75 * 16.0, true);

        // XP bar below name
        let bar_x = SKIN_W + GAP + 4;
        let bar_y = 60u32;
        let bar_w = W - bar_x - 8;
        let bar_h = 20u32;
        let filled = (self.data.level_frac * bar_w as f64) as u32;

        use render::cards::{BAR_COLOR, draw_progress_bar};
        draw_progress_bar(ctx, bar_x, bar_y, bar_w, bar_h, 5, 0.0, BAR_COLOR, BAR_COLOR);
        if filled > 0 {
            let fill = image::Rgba([85, 255, 255, 200]);
            draw_progress_bar(ctx, bar_x, bar_y, filled, bar_h, 5, 1.0, fill, fill);
        }
        let lf = 16.0f32;
        let lv = MCText::new().span(&format!("Lv. {}", self.data.level)).color(NamedColor::Aqua).build();
        let nv = MCText::new().span(&format!("Lv. {}", self.data.level + 1)).color(NamedColor::Aqua).build();
        let (nw, _) = ctx.renderer.measure(&nv, lf);
        ctx.renderer.draw(ctx.buffer.as_mut(), cw, ch,
            (ctx.x + bar_x as i32 + 5) as f32, (ctx.y + bar_y as i32 + 2) as f32, &lv, lf, true);
        ctx.renderer.draw(ctx.buffer.as_mut(), cw, ch,
            (ctx.x + bar_x as i32 + bar_w as i32 - nw as i32 - 5) as f32,
            (ctx.y + bar_y as i32 + 2) as f32, &nv, lf, true);
    }
    fn size(&self) -> (u32, u32) { (W, HEADER_H) }
}

// ── Info boxes ────────────────────────────────────────────────────────────────

fn level_box(data: &ProfileData) -> TextBox {
    TextBox::new().width(COL_W).height(BOX_H).corner_radius(CR).padding(12, 12).scale(1.5)
        .align_x(Align::Center).align_y(Align::Center)
        .push(MCText::new().span("Network Level").color(NamedColor::Gray).build())
        .push(MCText::new().span(&data.level.to_string()).color(NamedColor::Yellow).build())
}

fn guild_box(data: &ProfileData) -> TextBox {
    let guild = data.guild_name.as_deref().unwrap_or("None");
    let color = data.guild_tag_color.as_ref()
        .and_then(|c| color_name_to_named(c))
        .unwrap_or(NamedColor::Aqua);
    let tag = match &data.guild_tag {
        Some(t) => format!("[{t}]"),
        None => String::new(),
    };
    let mut b = TextBox::new().width(COL_W).height(BOX_H).corner_radius(CR).padding(12, 12).scale(1.5)
        .align_x(Align::Center).align_y(Align::Center)
        .push(MCText::new().span("Guild").color(NamedColor::Gray).build())
        .push(MCText::new().span(guild).color(color).build());
    if !tag.is_empty() {
        b = b.push(MCText::new().span(&tag).color(color).build());
    }
    b
}

fn achiev_box(data: &ProfileData) -> TextBox {
    TextBox::new().width(COL_W).height(BOX_H).corner_radius(CR).padding(12, 12).scale(1.5)
        .align_x(Align::Center).align_y(Align::Center)
        .push(MCText::new().span("Achievement Points").color(NamedColor::Gray).build())
        .push(MCText::new().span(&format_number(data.achievement_points)).color(NamedColor::Green).build())
}

fn karma_box(data: &ProfileData) -> TextBox {
    TextBox::new().width(COL_W).height(BOX_H).corner_radius(CR).padding(12, 12).scale(1.5)
        .align_x(Align::Center).align_y(Align::Center)
        .push(MCText::new().span("Karma").color(NamedColor::Gray).build())
        .push(MCText::new().span(&format_number(data.karma)).color(NamedColor::LightPurple).build())
}

fn bottom_box(data: &ProfileData) -> TextBox {
    let fl = data.first_login.map(format_timestamp).unwrap_or_else(|| "N/A".into());
    TextBox::new().width(W).height(BOTTOM_H).corner_radius(CR).padding(12, 12).scale(1.5)
        .align_x(Align::Center).align_y(Align::Center)
        .push(MCText::new().span("First Login: ").color(NamedColor::Gray)
            .then(&fl).color(NamedColor::White).build())
}

// ---- register() ---- //

// ── Command registration ──────────────────────────────────────────────────────

pub fn register() -> CreateCommand<'static> {
    CreateCommand::new("hypixel")
        .description("General Hypixel profile stats for a player")
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
        Err(e) => return crate::commands::stats::send_error(
            ctx, cmd, "Player Not Found", &e.to_string(),
        ).await,
    };

    let (player_json, guild_info, skin_bytes) = tokio::join!(
        fetch_hypixel_player(&identity.uuid),
        fetch_guild_by_player(&identity.uuid),
        fetch_skin_bytes(&identity.uuid),
    );

    let player_json = match player_json {
        Ok(p) => p,
        Err(_) => return crate::commands::stats::send_error(
            ctx, cmd, "API Error", "Could not fetch Hypixel data.",
        ).await,
    };

    let data = ProfileData::from_json(&player_json, guild_info, skin_bytes);
    let png = encode_png(&render_profile(&data))?;

    cmd.edit_response(
        &ctx.http,
        EditInteractionResponse::new()
            .new_attachment(CreateAttachment::bytes(png, "hypixel.png")),
    ).await?;

    Ok(())
}

async fn fetch_skin_bytes(uuid: &str) -> Option<Vec<u8>> {
    let url = format!("https://crafatar.com/skins/{}", uuid);
    let client = reqwest::Client::builder()
        .user_agent("stats-bot/0.1")
        .build().ok()?;
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() { return None; }
    resp.bytes().await.ok().map(|b| b.to_vec())
}
