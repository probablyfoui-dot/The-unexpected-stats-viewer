// crates/stats-bot/src/commands/stats/skywars/cards/overall.rs — SkyWars card renderer

// ---- Imports ---- //

use image::RgbaImage;
use mctext::{MCText, NamedColor};

use hypixel::{SkywarsStats, SwMode, SwModeStats, combined_sw_mode_name, get_sw_prestige};

use render::canvas::{
    Align, BOX_BACKGROUND, CANVAS_BACKGROUND, Canvas, DrawContext, RoundedRect, Shape, TextBlock, TextBox,
};
use render::cards::{color_name_to_named, format_number, format_ratio};

// ---- Card constants ---- //

const W: u32 = 800;
const CR: u32 = 18;
const GAP: u32 = 8;

const HEADER_H: u32 = 148;
const STATS_Y: i32 = HEADER_H as i32 + GAP as i32;
const STATS_H: u32 = 176;
const ROW2_Y: i32 = STATS_Y + STATS_H as i32 + GAP as i32;
const ROW2_H: u32 = 152;
const H: u32 = ROW2_Y as u32 + ROW2_H;

const COL_W: u32 = 256;
const COL_GAP: u32 = 16;
const LEVEL_SCALE: f32 = 2.75;

fn col_x(col: u32) -> i32 {
    (col * (COL_W + COL_GAP)) as i32
}

// ---- render_skywars() ---- //

pub fn render_skywars(stats: &SkywarsStats, modes: &[SwMode]) -> RgbaImage {
    let mode_stats = stats.get_combined_mode_stats(modes);
    let mode_label = combined_sw_mode_name(modes);

    Canvas::new(W, H)
        .background(CANVAS_BACKGROUND)
        .draw(0, 0, &HeaderSection { stats })
        .draw(0, STATS_Y, &StatsSection { stats: &mode_stats })
        .draw(col_x(0), ROW2_Y, &PrestigeBox { stats })
        .draw(col_x(1), ROW2_Y, &mode_label_box(&mode_label))
        .draw(col_x(2), ROW2_Y, &GuildBox { stats })
        .build()
}

// ---- HeaderSection ---- //

// ── Header (name + star/XP bar below) ─────────────────────────────────────────

struct HeaderSection<'a> { stats: &'a SkywarsStats }

impl Shape for HeaderSection<'_> {
    fn draw(&self, ctx: &mut DrawContext) {
        RoundedRect::new(W, HEADER_H).corner_radius(CR).background(BOX_BACKGROUND).draw(ctx);

        // Player name line
        let prefix = self.stats.rank_prefix.as_deref().unwrap_or("§7");
        let guild_tag = match (&self.stats.guild.tag, &self.stats.guild.tag_color) {
            (Some(tag), Some(color)) => format!(" {}[{}]", hypixel::color_code(color), tag),
            (Some(tag), None)        => format!(" §7[{tag}]"),
            _                        => String::new(),
        };
        let name_text = MCText::parse(&format!("{}{}{guild_tag}", prefix, self.stats.display_name));
        let (cw, ch) = ctx.buffer.dimensions();
        ctx.renderer.draw(ctx.buffer.as_mut(), cw, ch,
            (ctx.x + 20) as f32, (ctx.y + 13) as f32, &name_text, LEVEL_SCALE * 16.0, true);

        // Star / XP bar below name
        let scale = LEVEL_SCALE;
        let font_size = scale * 16.0;
        let available = W - 40;
        let bar_top_y = 72i32;

        let prestige = get_sw_prestige(self.stats.star);
        let star_str = format!("{}[{}✫]", prestige.color_code, self.stats.star);
        let next_str = format!("{}[{}✫]", prestige.color_code, self.stats.star + 1);
        let current_text = MCText::parse(&star_str);
        let next_text = MCText::parse(&next_str);

        let filled = (self.stats.progress * 25.0).round() as usize;
        let bar_text = MCText::new()
            .span("[").color(NamedColor::DarkGray)
            .then(&"■".repeat(filled)).color(NamedColor::Aqua)
            .then(&"■".repeat(25 - filled)).color(NamedColor::Gray)
            .then("]").color(NamedColor::DarkGray)
            .build();

        let (cur_w, star_h) = ctx.renderer.measure(&current_text, font_size);
        let (nxt_w, _)      = ctx.renderer.measure(&next_text, font_size);
        let spacing = font_size * 0.3;
        let bar_avail = available as f32 - cur_w - nxt_w - spacing * 2.0;
        let (bar_w, bar_h) = ctx.renderer.measure(&bar_text, font_size);
        let (bar_scale, scaled_w, bar_h) = if bar_w > bar_avail {
            let s = scale * (bar_avail / bar_w);
            let (w, h) = ctx.renderer.measure(&bar_text, s * 16.0);
            (s, w, h)
        } else {
            (scale, bar_w, bar_h)
        };

        let total_w = cur_w + spacing + scaled_w + spacing + nxt_w;
        let start_x = 20.0 + (available as f32 - total_w) / 2.0;
        let star_y  = ctx.y + bar_top_y;
        let bar_y   = ctx.y + bar_top_y + ((star_h - bar_h) / 2.0) as i32;

        ctx.renderer.draw(ctx.buffer.as_mut(), cw, ch,
            ctx.x as f32 + start_x, star_y as f32,
            &current_text, font_size, true);

        let bx = start_x + cur_w + spacing;
        ctx.renderer.draw(ctx.buffer.as_mut(), cw, ch,
            ctx.x as f32 + bx, bar_y as f32,
            &bar_text, bar_scale * 16.0, true);

        let nx = bx + scaled_w + spacing;
        ctx.renderer.draw(ctx.buffer.as_mut(), cw, ch,
            ctx.x as f32 + nx, star_y as f32,
            &next_text, font_size, true);
    }
    fn size(&self) -> (u32, u32) { (W, HEADER_H) }
}

// ---- StatsSection ---- //

// ── Stats section ─────────────────────────────────────────────────────────────

struct StatsSection<'a> { stats: &'a SwModeStats }

impl Shape for StatsSection<'_> {
    fn draw(&self, ctx: &mut DrawContext) {
        RoundedRect::new(W, STATS_H)
            .corner_radius(CR)
            .background(BOX_BACKGROUND)
            .draw(ctx);

        let main_scale = 2.0f32;
        let neg_scale = 1.5f32;
        let main_font = main_scale * 16.0;
        let neg_font = neg_scale * 16.0;
        let padding = 16u32;
        let line_h = (STATS_H - padding * 2) / 3;

        let rows: [(&str, &str, f64, u64, u64); 3] = [
            ("WLR:", "Wins:",   self.stats.wlr(), self.stats.wins,   self.stats.losses),
            ("KDR:", "Kills:",  self.stats.kdr(), self.stats.kills,  self.stats.deaths),
            ("Assists:", "Games:", 0.0, self.stats.assists, self.stats.games()),
        ];

        let mut max_right_w: f32 = 0.0;
        let mut measurements = Vec::new();

        for (ratio_label, pos_label, ratio, positive, negative) in &rows {
            if ratio_label.is_empty() { measurements.push(None); continue; }

            let ratio_color = match *ratio_label {
                "WLR:" => color_for_wlr(*ratio),
                "KDR:" => color_for_kdr(*ratio),
                _ => NamedColor::Aqua,
            };

            let ratio_text = MCText::new()
                .span(*ratio_label)
                .color(NamedColor::Gray)
                .then(" ")
                .then(&format_ratio(*ratio))
                .color(ratio_color)
                .build();
            let (_, main_h) = ctx.renderer.measure(&ratio_text, main_font);

            let pos_text = MCText::new()
                .span(*pos_label)
                .color(NamedColor::Gray)
                .then(" ")
                .then(&format_number(*positive))
                .color(NamedColor::White)
                .build();
            let (pos_w, _) = ctx.renderer.measure(&pos_text, main_font);

            let neg_text = MCText::new()
                .span(" / ")
                .color(NamedColor::DarkGray)
                .then(&format_number(*negative))
                .color(NamedColor::Gray)
                .build();
            let (neg_w, neg_h) = ctx.renderer.measure(&neg_text, neg_font);
            max_right_w = max_right_w.max(pos_w + neg_w);
            measurements.push(Some((ratio_text, pos_text, neg_text, pos_w, main_h, neg_h)));
        }

        let right_edge = W as f32 - padding as f32;
        let col_pos = (W as f32 / 2.0).min(right_edge - max_right_w);

        for (i, m) in measurements.into_iter().enumerate() {
            let Some((ratio_text, pos_text, neg_text, pos_w, main_h, neg_h)) = m else { continue; };
            let y = padding + i as u32 * line_h;
            TextBlock::new().push(ratio_text).scale(main_scale).draw(&mut ctx.at(padding as i32, y as i32));
            TextBlock::new().push(pos_text).scale(main_scale).draw(&mut ctx.at(col_pos as i32, y as i32));
            let neg_y = y as f32 + (main_h - neg_h) * 0.75;
            TextBlock::new().push(neg_text).scale(neg_scale).draw(&mut ctx.at((col_pos + pos_w) as i32, neg_y as i32));
        }
    }
    fn size(&self) -> (u32, u32) { (W, STATS_H) }
}

fn color_for_wlr(v: f64) -> NamedColor {
    match v {
        v if v >= 30.0 => NamedColor::DarkPurple,
        v if v >= 10.0 => NamedColor::LightPurple,
        v if v >= 5.0  => NamedColor::Red,
        v if v >= 3.0  => NamedColor::Gold,
        v if v >= 2.0  => NamedColor::Yellow,
        v if v >= 1.5  => NamedColor::DarkGreen,
        v if v >= 1.0  => NamedColor::Green,
        v if v >= 0.5  => NamedColor::White,
        _              => NamedColor::Gray,
    }
}

fn color_for_kdr(v: f64) -> NamedColor {
    match v {
        v if v >= 20.0 => NamedColor::DarkPurple,
        v if v >= 10.0 => NamedColor::LightPurple,
        v if v >= 6.0  => NamedColor::Red,
        v if v >= 4.0  => NamedColor::Gold,
        v if v >= 2.5  => NamedColor::Yellow,
        v if v >= 1.5  => NamedColor::DarkGreen,
        v if v >= 1.0  => NamedColor::Green,
        v if v >= 0.5  => NamedColor::White,
        _              => NamedColor::Gray,
    }
}

// ── Prestige box ──────────────────────────────────────────────────────────────

struct PrestigeBox<'a> { stats: &'a SkywarsStats }

impl Shape for PrestigeBox<'_> {
    fn draw(&self, ctx: &mut DrawContext) {
        let prestige = get_sw_prestige(self.stats.star);
        let star_text = MCText::parse(&format!(
            "{}[{}✫]", prestige.color_code, self.stats.star
        ));
        let prestige_text = MCText::new()
            .span(prestige.name)
            .color(NamedColor::Gray)
            .build();

        TextBox::new()
            .width(COL_W)
            .height(ROW2_H)
            .corner_radius(CR)
            .padding(12, 12)
            .scale(1.5)
            .align_x(Align::Center)
            .align_y(Align::Center)
            .push(star_text)
            .push(prestige_text)
            .draw(&mut ctx.at(0, 0));
    }
    fn size(&self) -> (u32, u32) { (COL_W, ROW2_H) }
}

// ── Mode label box ────────────────────────────────────────────────────────────

fn mode_label_box(label: &str) -> TextBox {
    TextBox::new()
        .width(COL_W)
        .height(ROW2_H)
        .corner_radius(CR)
        .padding(12, 12)
        .scale(1.5)
        .align_x(Align::Center)
        .align_y(Align::Center)
        .push(MCText::new().span("Mode").color(NamedColor::Gray).build())
        .push(MCText::new().span(label).color(NamedColor::Aqua).build())
}

// ── Guild box ─────────────────────────────────────────────────────────────────

struct GuildBox<'a> { stats: &'a SkywarsStats }

impl Shape for GuildBox<'_> {
    fn draw(&self, ctx: &mut DrawContext) {
        let name  = self.stats.guild.name.as_deref().unwrap_or("-");
        let color = self.stats.guild.tag_color.as_ref()
            .and_then(|c| color_name_to_named(c))
            .unwrap_or(NamedColor::Gray);
        let tag_str = match (&self.stats.guild.tag, &self.stats.guild.tag_color) {
            (Some(tag), _) => format!("[{tag}]"),
            _              => String::new(),
        };

        TextBox::new()
            .width(COL_W)
            .height(ROW2_H)
            .corner_radius(CR)
            .padding(12, 12)
            .scale(1.5)
            .align_x(Align::Center)
            .align_y(Align::Center)
            .push(MCText::new().span(name).color(color).build())
            .push(MCText::new().span(&tag_str).color(color).build())
            .draw(&mut ctx.at(0, 0));
    }
    fn size(&self) -> (u32, u32) { (COL_W, ROW2_H) }
}
