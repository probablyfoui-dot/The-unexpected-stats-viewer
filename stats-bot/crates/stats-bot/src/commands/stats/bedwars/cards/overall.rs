// crates/stats-bot/src/commands/stats/bedwars/cards/overall.rs — BedWars card renderer

// ---- Imports ---- //

use image::RgbaImage;
use mctext::{MCText, NamedColor};

use hypixel::{BedwarsPlayerStats, Mode, ModeStats, combined_mode_name, level_progress};

use super::prestiges::{render_prestige, resolve_cosmetics};
use render::canvas::{
    Align, BOX_BACKGROUND, CANVAS_BACKGROUND, Canvas, DrawContext, RoundedRect, Shape, TextBlock, TextBox,
};
use render::cards::{
    BAR_COLOR, ModeGames, VerticalGamesBox, bedwars_colors as colors,
    color_name_to_named, draw_progress_bar, format_number, format_percent, format_ratio,
    format_timestamp,
};

// ---- Card constants ---- //

// ── Layout ────────────────────────────────────────────────────────────────────

const W: u32 = 800;
const CR: u32 = 18;
const GAP: u32 = 16;
const COL_W: u32 = 256;

const HEADER_Y: i32 = 0;
// Header now contains both name and star/XP bar
const HEADER_H: u32 = 148;
const STATS_Y: i32 = HEADER_H as i32 + GAP as i32;
const STATS_H: u32 = 176;
const ROW2_Y: i32 = STATS_Y + STATS_H as i32 + GAP as i32;
const ROW2_H: u32 = 152;
const ROW3_Y: i32 = ROW2_Y + ROW2_H as i32 + GAP as i32;
const ROW3_H: u32 = 88;
const H: u32 = (ROW3_Y + ROW3_H as i32) as u32;

fn col_x(col: u32) -> i32 { (col * (COL_W + GAP)) as i32 }

// ---- render_bedwars() ---- //

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn render_bedwars(stats: &BedwarsPlayerStats, modes: &[Mode]) -> RgbaImage {
    let mode_stats = stats.get_combined_mode_stats(modes);
    let mode_label = combined_mode_name(modes);

    let is_single = modes.len() == 1;
    let is_overall = is_single && modes[0] == Mode::Overall;

    let mode_games = ModeGames {
        solos:       stats.solos.wins + stats.solos.losses,
        doubles:     stats.doubles.wins + stats.doubles.losses,
        threes:      stats.threes.wins + stats.threes.losses,
        fours:       stats.fours.wins + stats.fours.losses,
        four_v_four: stats.four_v_four.wins + stats.four_v_four.losses,
    };

    let share_box  = ModeShareBox  { mode: &mode_stats, overall: &stats.overall };
    let games_box   = VerticalGamesBox::new(&mode_games, COL_W, ROW2_H);
    let label_box   = mode_label_box(&mode_label, COL_W, ROW2_H);
    let guild_box_s = GuildBox { stats, h: ROW2_H };
    let lbox        = login_box(stats, COL_W, ROW3_H);
    let nbox        = network_box(stats, COL_W, ROW3_H);
    let sbox        = slumber_box(stats, COL_W, ROW3_H);

    let base = Canvas::new(W, H)
        .background(CANVAS_BACKGROUND)
        .draw(0, HEADER_Y, &HeaderSection { stats })

        .draw(0, STATS_Y,  &StatsSection { stats: &mode_stats })
        .draw(col_x(1), ROW2_Y, &label_box)
        .draw(col_x(2), ROW2_Y, &guild_box_s)
        .draw(col_x(0), ROW3_Y, &lbox)
        .draw(col_x(1), ROW3_Y, &nbox)
        .draw(col_x(2), ROW3_Y, &sbox);

    if is_single && !is_overall {
        base.draw(col_x(0), ROW2_Y, &share_box).build()
    } else {
        base.draw(col_x(0), ROW2_Y, &games_box).build()
    }
}

// ---- HeaderSection ---- //

// ── Header ────────────────────────────────────────────────────────────────────

struct HeaderSection<'a> { stats: &'a BedwarsPlayerStats }

impl Shape for HeaderSection<'_> {
    fn draw(&self, ctx: &mut DrawContext) {
        RoundedRect::new(W, HEADER_H).corner_radius(CR).background(BOX_BACKGROUND).draw(ctx);

        // ── Player name line ──────────────────────────────────────────────
        let prefix = self.stats.rank_prefix.as_deref().unwrap_or("§7");
        let guild_tag = match (&self.stats.guild.tag, &self.stats.guild.tag_color) {
            (Some(tag), Some(color)) => format!(" {}[{tag}]", hypixel::color_code(color)),
            (Some(tag), None)        => format!(" §7[{tag}]"),
            _                        => String::new(),
        };
        let name_text = MCText::parse(&format!("{prefix}{}{guild_tag}", self.stats.display_name));
        let (cw, ch) = ctx.buffer.dimensions();
        ctx.renderer.draw(ctx.buffer.as_mut(), cw, ch,
            (ctx.x + 20) as f32, (ctx.y + 13) as f32, &name_text, 2.75 * 16.0, true);

        // ── Star / XP bar (inside header, below name) ─────────────────────
        let scale = 2.75f32;
        let font_size = scale * 16.0;
        let available = W - 40;
        let bar_top_y = 72i32; // y inside header box

        let cosmetics = resolve_cosmetics(&self.stats.cosmetics);
        let current_text = render_prestige(self.stats.level, &cosmetics);
        let next_text    = render_prestige(self.stats.level + 1, &cosmetics);

        let progress = level_progress(self.stats.experience);
        let filled = (progress * 25.0).round() as usize;
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

// ── Main stats table (full width) ────────────────────────────────────────────

struct StatsSection<'a> { stats: &'a ModeStats }

impl Shape for StatsSection<'_> {
    fn draw(&self, ctx: &mut DrawContext) {
        RoundedRect::new(W, STATS_H).corner_radius(CR).background(BOX_BACKGROUND).draw(ctx);

        let main_s  = 2.0f32;
        let neg_s   = 1.5f32;
        let main_f  = main_s * 16.0;
        let neg_f   = neg_s * 16.0;
        let pad     = 16u32;
        let line_h  = (STATS_H - pad * 2) / 4;

        struct Row {
            ratio_label: &'static str,
            pos_label:   &'static str,
            ratio:       f64,
            positive:    u64,
            negative:    u64,
            ratio_color: NamedColor,
            pos_color:   NamedColor,
        }

        let rows = [
            Row { ratio_label: "WLR:",  pos_label: "Wins:",   ratio: self.stats.wlr(),  positive: self.stats.wins,        negative: self.stats.losses,       ratio_color: colors::wlr(self.stats.wlr()),   pos_color: colors::wins(self.stats.wins)   },
            Row { ratio_label: "FKDR:", pos_label: "Finals:", ratio: self.stats.fkdr(), positive: self.stats.final_kills,  negative: self.stats.final_deaths, ratio_color: colors::fkdr(self.stats.fkdr()), pos_color: colors::final_kills(self.stats.final_kills) },
            Row { ratio_label: "KDR:",  pos_label: "Kills:",  ratio: self.stats.kdr(),  positive: self.stats.kills,        negative: self.stats.deaths,       ratio_color: colors::kdr(self.stats.kdr()),   pos_color: colors::kills(self.stats.kills)  },
            Row { ratio_label: "BBLR:", pos_label: "Beds:",   ratio: self.stats.bblr(), positive: self.stats.beds_broken,  negative: self.stats.beds_lost,    ratio_color: colors::bblr(self.stats.bblr()), pos_color: colors::beds_broken(self.stats.beds_broken) },
        ];

        let mut max_right_w = 0.0f32;
        let mut measures: Vec<(MCText, MCText, MCText, f32, f32, f32)> = Vec::new();

        for row in &rows {
            let r_text = MCText::new()
                .span(row.ratio_label).color(NamedColor::Gray)
                .then(" ").then(&format_ratio(row.ratio)).color(row.ratio_color)
                .build();
            let (_, mh) = ctx.renderer.measure(&r_text, main_f);

            let p_text = MCText::new()
                .span(row.pos_label).color(NamedColor::Gray)
                .then(" ").then(&format_number(row.positive)).color(row.pos_color)
                .build();
            let (pw, _) = ctx.renderer.measure(&p_text, main_f);

            let n_text = MCText::new()
                .span(" / ").color(NamedColor::DarkGray)
                .then(&format_number(row.negative)).color(NamedColor::Gray)
                .build();
            let (nw, nh) = ctx.renderer.measure(&n_text, neg_f);
            max_right_w = max_right_w.max(pw + nw);
            measures.push((r_text, p_text, n_text, pw, mh, nh));
        }

        let col_pos = (W as f32 / 2.0).min(W as f32 - pad as f32 - max_right_w);

        for (i, (r_text, p_text, n_text, pw, mh, nh)) in measures.into_iter().enumerate() {
            let y = (pad + i as u32 * line_h) as i32;
            TextBlock::new().push(r_text).scale(main_s).draw(&mut ctx.at(pad as i32, y));
            TextBlock::new().push(p_text).scale(main_s).draw(&mut ctx.at(col_pos as i32, y));
            let ny = y as f32 + (mh - nh) * 0.75;
            TextBlock::new().push(n_text).scale(neg_s).draw(&mut ctx.at((col_pos + pw) as i32, ny as i32));
        }
    }
    fn size(&self) -> (u32, u32) { (W, STATS_H) }
}

// ── Mode share box (single non-overall mode only) ─────────────────────────────

struct ModeShareBox<'a> { mode: &'a ModeStats, overall: &'a ModeStats }

impl Shape for ModeShareBox<'_> {
    fn draw(&self, ctx: &mut DrawContext) {
        RoundedRect::new(COL_W, ROW2_H).corner_radius(CR).background(BOX_BACKGROUND).draw(ctx);

        let pad = 14u32;
        let bar_h = 26u32;
        let scale = 1.5f32;
        let font = scale * 16.0;
        let bar_w = COL_W - pad * 2;
        let gap = (ROW2_H - pad * 2 - bar_h * 4) / 3;
        let (cw, ch) = ctx.buffer.dimensions();

        let pct = |mv: u64, ov: u64| if ov == 0 { 0.0 } else { mv as f64 / ov as f64 * 100.0 };
        let rows = [
            ("Wins",   pct(self.mode.wins,        self.overall.wins)),
            ("Finals", pct(self.mode.final_kills,  self.overall.final_kills)),
            ("Kills",  pct(self.mode.kills,        self.overall.kills)),
            ("Beds",   pct(self.mode.beds_broken,  self.overall.beds_broken)),
        ];

        for (i, (label, p)) in rows.iter().enumerate() {
            let bx = pad;
            let by = pad + i as u32 * (bar_h + gap);
            let fw = (p / 100.0 * bar_w as f64) as u32;
            draw_progress_bar(ctx, bx, by, bar_w, bar_h, 0, 0.0, BAR_COLOR, BAR_COLOR);
            if fw > 0 {
                draw_progress_bar(ctx, bx, by, fw, bar_h, 0, 1.0, BAR_COLOR, BAR_COLOR);
            }
            let txt = MCText::new()
                .span(&format_percent(*p)).color(NamedColor::Green)
                .then(&format!(" of {label}")).color(NamedColor::Gray)
                .build();
            let (tw, th) = ctx.renderer.measure(&txt, font);
            ctx.renderer.draw(ctx.buffer.as_mut(), cw, ch,
                ctx.x as f32 + bx as f32 + (bar_w as f32 - tw) / 2.0,
                ctx.y as f32 + by as f32 + (bar_h as f32 - th) / 2.0,
                &txt, font, true);
        }
    }
    fn size(&self) -> (u32, u32) { (COL_W, ROW2_H) }
}

// ── Mode label box ────────────────────────────────────────────────────────────

fn mode_label_box(label: &str, w: u32, h: u32) -> TextBox {
    TextBox::new()
        .width(w).height(h).corner_radius(CR).padding(12, 12).scale(1.5)
        .align_x(Align::Center).align_y(Align::Center)
        .push(MCText::new().span("Mode").color(NamedColor::Gray).build())
        .push(MCText::new().span(label).color(NamedColor::Aqua).build())
}

// ── Guild box ─────────────────────────────────────────────────────────────────

struct GuildBox<'a> { stats: &'a BedwarsPlayerStats, h: u32 }

impl Shape for GuildBox<'_> {
    fn draw(&self, ctx: &mut DrawContext) {
        let name  = self.stats.guild.name.as_deref().unwrap_or("—");
        let color = self.stats.guild.tag_color.as_ref()
            .and_then(|c| color_name_to_named(c))
            .unwrap_or(NamedColor::Gray);
        let tag_str = match (&self.stats.guild.tag, &self.stats.guild.tag_color) {
            (Some(tag), _) => format!("[{tag}]"),
            _              => String::new(),
        };
        TextBox::new()
            .width(COL_W).height(self.h).corner_radius(CR).padding(12, 12).scale(1.5)
            .align_x(Align::Center).align_y(Align::Center)
            .push(MCText::new().span(name).color(color).build())
            .push(MCText::new().span(&tag_str).color(color).build())
            .draw(&mut ctx.at(0, 0));
    }
    fn size(&self) -> (u32, u32) { (COL_W, self.h) }
}

// ── Bottom row boxes ──────────────────────────────────────────────────────────

fn login_box(stats: &BedwarsPlayerStats, w: u32, h: u32) -> TextBox {
    let fl = stats.first_login.map(format_timestamp).unwrap_or_else(|| "N/A".into());
    TextBox::new()
        .width(w).height(h).corner_radius(CR).padding(12, 12).scale(1.5)
        .align_x(Align::Center).align_y(Align::Center)
        .push(MCText::new().span("First Login: ").color(NamedColor::Gray)
            .then(&fl).color(NamedColor::White).build())
        .push(MCText::new().span("Achiev. Pts: ").color(NamedColor::Gray)
            .then(&format_number(stats.achievement_points)).color(NamedColor::Yellow).build())
}

fn network_box(stats: &BedwarsPlayerStats, w: u32, h: u32) -> TextBox {
    let lvl = format!("{:.0}", stats.network_level);
    TextBox::new()
        .width(w).height(h).corner_radius(CR).padding(12, 12).scale(1.5)
        .align_x(Align::Center).align_y(Align::Center)
        .push(MCText::new().span("Network Lvl: ").color(NamedColor::Gray)
            .then(&lvl).color(NamedColor::Yellow).build())
        .push(MCText::new().span("Ranks Gifted: ").color(NamedColor::Gray)
            .then(&format_number(stats.ranks_gifted)).color(NamedColor::Gold).build())
}

fn slumber_box(stats: &BedwarsPlayerStats, w: u32, h: u32) -> TextBox {
    TextBox::new()
        .width(w).height(h).corner_radius(CR).padding(12, 12).scale(1.5)
        .align_x(Align::Center).align_y(Align::Center)
        .push(MCText::new().span("Tickets: ").color(NamedColor::Gray)
            .then(&format_number(stats.slumber.tickets)).color(NamedColor::Aqua).build())
        .push(MCText::new().span("Lifetime Tkts: ").color(NamedColor::Gray)
            .then(&format_number(stats.slumber.total_tickets_earned)).color(NamedColor::DarkAqua).build())
        .push(MCText::new().span("XP Doublers: ").color(NamedColor::Gray)
            .then(&format_number(stats.slumber.doublers)).color(NamedColor::Green).build())
}
