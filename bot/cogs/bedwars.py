"""
cogs/bedwars.py
---------------
/bedwars command implementation.
"""

import io
import aiohttp
import discord
from discord import app_commands
from discord.ext import commands
from PIL import Image, ImageDraw, ImageFont

from core import fetch_uuid, fetch_player, FONT_PATH
from cogs.hypixel import get_rank_display, draw_segments, segments_width


# ---------------------------------------------------------------------------
# FONT
# ---------------------------------------------------------------------------

def load_font(size=10):
    try:
        return ImageFont.truetype(FONT_PATH, size)
    except Exception:
        return ImageFont.load_default()


# ---------------------------------------------------------------------------
# PRESTIGE
# ---------------------------------------------------------------------------

# Max current suported prestige is 5000 (Eternal). We'll probably support the up to 10k next update.
# Colors from the Hypixel wiki prestige table.
# Stars are not included yet but will be added in the future.
PRESTIGE_TABLE = [
    (   1, (150, 150, 150), "Stone"),
    ( 100, (255, 255, 255), "Iron"),
    ( 200, (255, 170,   0), "Gold"),
    ( 300, ( 85, 255, 255), "Diamond"),
    ( 400, ( 85, 255,  85), "Emerald"),
    ( 500, (  0, 170, 170), "Sapphire"),
    ( 600, (255,  85,  85), "Ruby"),
    ( 700, (255, 170, 255), "Crystal"),
    ( 800, (170, 255, 255), "Opal"),
    ( 900, (170,   0, 170), "Amethyst"),
    (1000, (255, 170,   0), "Rainbow"),     # animated in-game, couldnt do it.
    (1100, (255, 255, 255), "Iron Prime"),
    (1200, (255, 170,   0), "Gold Prime"),
    (1300, ( 85, 255, 255), "Diamond Prime"),
    (1400, ( 85, 255,  85), "Emerald Prime"),
    (1500, (  0, 170, 170), "Sapphire Prime"),
    (1600, (255,  85,  85), "Ruby Prime"),
    (1700, (255, 170, 255), "Crystal Prime"),
    (1800, (170, 255, 255), "Opal Prime"),
    (1900, (170,   0, 170), "Amethyst Prime"),
    (2000, (255, 255, 255), "Mirror"),
    (2100, (255, 255, 255), "Light"),
    (2200, (255, 170, 100), "Dawn"),
    (2300, (170,   0, 170), "Dusk"),
    (2400, (255, 255, 255), "Air"),
    (2500, ( 85, 255, 255), "Wind"),
    (2600, (170,   0, 170), "Nebula"),
    (2700, (255, 255,  85), "Thunder"),
    (2800, ( 85, 255,  85), "Earth"),
    (2900, ( 85, 170, 255), "Water"),
    (3000, (255,  85,  85), "Fire"),
    (3100, (255, 200,  50), "Sunrise"),
    (3200, (100,   0, 200), "Eclipse"),
    (3300, ( 85, 255,  85), "Gamma"),
    (3400, (200, 100, 255), "Majestic"),
    (3500, ( 85, 170, 255), "Adesine"),
    (3600, (  0, 170, 170), "Marine"),
    (3700, ( 85, 255,  85), "Element"),
    (3800, (100,  50, 200), "Galaxy"),
    (3900, (255, 170,   0), "Atomic"),
    (4000, (255, 100,  50), "Sunset"),
    (4100, (255, 255, 255), "Time"),
    (4200, ( 85, 255, 255), "Winter"),
    (4300, ( 50,  50,  50), "Obsidian"),
    (4400, ( 85, 255,  85), "Spring"),
    (4500, (170, 255, 255), "Ice"),
    (4600, (255, 200,  50), "Summer"),
    (4700, (255,  85, 170), "Spinel"),
    (4800, (255, 140,  50), "Autumn"),
    (4900, (170,   0, 170), "Mystic"),
    (5000, (255, 170,   0), "Eternal"),
]

def get_prestige(level: int):
    """Return (color, name, star_symbol) for a given BedWars level."""
    color, name = (150, 150, 150), "Stone"
    for min_lvl, col, nm in PRESTIGE_TABLE:
        if level >= min_lvl:
            color, name = col, nm
        else:
            break
    star = "*"  # will be adding the different stars in a later version.
    return color, name, star


# ---------------------------------------------------------------------------
# XP → LEVEL
# ---------------------------------------------------------------------------

PRESTIGE_XP = 487000
EASY_XP     = [500, 1000, 2000, 3500]
EASY_TOTAL  = 7000
XP_PER      = 5000

def bedwars_level(exp: int) -> tuple[int, float]:
    """Returns (level, progress) where progress is 0.0–1.0 within the current level."""
    prestige = exp // PRESTIGE_XP
    level    = prestige * 100
    rem      = exp % PRESTIGE_XP

    if rem < EASY_TOTAL:
        cumulative = 0
        for i, xp in enumerate(EASY_XP):
            if rem < cumulative + xp:
                progress = (rem - cumulative) / xp
                return (level, progress)
            cumulative += xp
            level += 1
    else:
        level += 4
        rem   -= EASY_TOTAL
        level += rem // XP_PER
        progress = (rem % XP_PER) / XP_PER
        return (level, progress)

    return (level, 0.0)


# ---------------------------------------------------------------------------
# MODES
# ---------------------------------------------------------------------------

MODE_KEYS = {
    "all":          None,
    "core":         ["eight_one", "eight_two", "four_three", "four_four"],
    "solo":         ["eight_one"],
    "doubles":      ["eight_two"],
    "threes":       ["four_three"],
    "fours":        ["four_four"],
    "4v4":          ["two_four"],
    "rush":         ["eight_two_rush",       "four_four_rush"],
    "ultimate":     ["eight_two_ultimate",   "four_four_ultimate"],
    "armed":        ["eight_two_armed",      "four_four_armed"],
    "lucky":        ["eight_two_lucky",      "four_four_lucky"],
    "voidless":     ["eight_two_voidless",   "four_four_voidless"],
    "swap":         ["eight_two_swap",       "four_four_swap"],
    "castle":       ["castle"],
    "underworld":   ["eight_two_underworld",    "four_four_underworld"],
    "totallynormal":["eight_two_totallynormal", "four_four_totallynormal"],
}

MODE_LABELS = {
    "all":"All Modes","core":"Core","solo":"Solo","doubles":"Doubles",
    "threes":"Threes","fours":"Fours","4v4":"4v4","rush":"Rush",
    "ultimate":"Ultimate","armed":"Armed","lucky":"Lucky Blocks",
    "voidless":"Voidless","swap":"Swap","castle":"Castle",
    "underworld":"Underworld","totallynormal":"Totally Normal",
}


# ---------------------------------------------------------------------------
# STAT HELPERS
# ---------------------------------------------------------------------------

def ratio(a, b):
    return round(a / b, 2) if b else float(a)

def get_stats(bw, mode):
    if mode == "all":
        w  = bw.get("wins_bedwars",        0); l  = bw.get("losses_bedwars",       0)
        fk = bw.get("final_kills_bedwars", 0); fd = bw.get("final_deaths_bedwars", 0)
        k  = bw.get("kills_bedwars",       0); d  = bw.get("deaths_bedwars",       0)
        bb = bw.get("beds_broken_bedwars", 0); bl = bw.get("beds_lost_bedwars",    0)
    else:
        w = l = fk = fd = k = d = bb = bl = 0
        for s in MODE_KEYS.get(mode, []):
            w  += bw.get(f"{s}_wins_bedwars",         0)
            l  += bw.get(f"{s}_losses_bedwars",       0)
            fk += bw.get(f"{s}_final_kills_bedwars",  0)
            fd += bw.get(f"{s}_final_deaths_bedwars", 0)
            k  += bw.get(f"{s}_kills_bedwars",        0)
            d  += bw.get(f"{s}_deaths_bedwars",       0)
            bb += bw.get(f"{s}_beds_broken_bedwars",  0)
            bl += bw.get(f"{s}_beds_lost_bedwars",    0)
    return dict(
        wins=w, losses=l, wlr=ratio(w, l),
        final_kills=fk, final_deaths=fd, fkdr=ratio(fk, fd),
        kills=k, deaths=d, kdr=ratio(k, d),
        beds_broken=bb, beds_lost=bl, bblr=ratio(bb, bl),
        games=w + l,
    )

def get_ticket_info(bw, player):
    gg = player.get("gambler_george", {})
    return dict(
        total_earned    = bw.get("total_tickets_earned", 0),
        tickets_stock   = bw.get("tickets", 0),
        slumber_spent   = bw.get("bedwars_slumber_ticket_master", 0),
        gg_win          = gg.get("gambler_george_win", False),
        gg_gold         = gg.get("blacksmith_golden_ticket", False),
        gg_chess        = gg.get("chess_tickets", False),
        gg_golden_count = gg.get("slumber_item_golden_ticket", 0),
        gg_ticket_tart  = gg.get("slumber_item_ticket_tart", 0),
    )


# ---------------------------------------------------------------------------
# DRAW HELPERS
# ---------------------------------------------------------------------------

C_BG       = (12,  12,  20)
C_PANEL    = (22,  24,  38)
C_BORDER   = (45,  50,  75)
C_GREEN    = (80,  230, 120)
C_CYAN     = (0,   200, 255)
C_GOLD     = (255, 200, 50)
C_RED      = (255, 80,  80)
C_WHITE    = (230, 235, 245)
C_GRAY     = (120, 130, 150)
C_DARKGRAY = (60,  65,  85)

def panel(draw, x, y, w, h, color=C_PANEL, border=C_BORDER, radius=6):
    draw.rounded_rectangle([x, y, x+w, y+h], radius=radius, fill=color, outline=border, width=1)

def px(draw, x, y, text, font, color=C_WHITE, anchor="la"):
    draw.text((x, y), text, font=font, fill=color, anchor=anchor)


# ---------------------------------------------------------------------------
# IMAGE GENERATOR
# ---------------------------------------------------------------------------

W, H = 860, 480

async def generate_stats_image(ign, level, progress, prestige_color, prestige_name, star,
                                stats, ticket_info, mode, uuid, session,
                                rank_segments, rank_base_color):
    img = Image.new("RGB", (W, H), C_BG)
    d   = ImageDraw.Draw(img)

    f8  = load_font(8)
    f10 = load_font(10)
    f14 = load_font(14)
    f16 = load_font(16)

    PAD = 16
    RX  = PAD
    RW  = W - PAD * 2

    d.rectangle([0, 0, W, 4], fill=prestige_color)

    # Single header row: badge, rank+IGN, XP bar
    badge   = f"[{level}{star}]"
    badge_w = int(d.textlength(badge, font=f14))
    bar_y   = 8; bar_h = 18

    seg_w        = segments_width(d, rank_segments, f14) if rank_segments else 0
    ign_w        = int(d.textlength(ign, font=f14))
    name_block_w = seg_w + (8 if rank_segments else 0) + ign_w + 12

    bar_x = RX + badge_w + 8 + name_block_w
    bar_w = RW - (bar_x - RX)

    px(d, RX, bar_y+2, badge, f14, prestige_color)
    draw_segments(d, RX + badge_w + 8, bar_y+2, rank_segments, f14)
    px(d, RX + badge_w + 8 + seg_w + (8 if rank_segments else 0), bar_y+2, ign, f14, rank_base_color)

    d.rounded_rectangle([bar_x, bar_y, bar_x+bar_w, bar_y+bar_h], radius=3, fill=C_DARKGRAY)
    if progress > 0:
        d.rounded_rectangle([bar_x, bar_y, bar_x+int(bar_w*progress), bar_y+bar_h],
                            radius=3, fill=prestige_color)
    px(d, bar_x+4,       bar_y+5, f"{level}{star}",   f8, C_BG)
    px(d, bar_x+bar_w-4, bar_y+5, f"{level+1}{star}", f8, C_BG, anchor="ra")
    px(d, bar_x+bar_w,   bar_y+bar_h+6, MODE_LABELS.get(mode, mode), f8, C_GRAY, anchor="ra")

    sep_y = bar_y + bar_h + 14
    d.line([(RX, sep_y), (W-PAD, sep_y)], fill=C_BORDER, width=1)

    GY  = sep_y + 8; BH = 82; GAP = 8
    C0W = 180; C1W = (RW - C0W - GAP*2) // 2
    C0X = RX;  C1X = RX + C0W + GAP;  C2X = C1X + C1W + GAP

    ratio_stats = [
        ("WLR",  stats["wlr"],  C_GREEN),
        ("FKDR", stats["fkdr"], C_CYAN),
        ("KDR",  stats["kdr"],  C_WHITE),
        ("BBLR", stats["bblr"], C_GOLD),
    ]
    pos_stats = [
        ("Wins",        stats["wins"],        C_GREEN),
        ("Final Kills", stats["final_kills"], C_CYAN),
        ("Kills",       stats["kills"],       C_WHITE),
        ("Beds Broken", stats["beds_broken"], C_GOLD),
    ]
    neg_stats = [
        ("Losses",       stats["losses"],       C_RED),
        ("Final Deaths", stats["final_deaths"], C_WHITE),
        ("Deaths",       stats["deaths"],       C_WHITE),
        ("Beds Lost",    stats["beds_lost"],    C_WHITE),
    ]

    for i in range(4):
        by = GY + i * (BH + GAP)

        panel(d, C0X, by, C0W, BH)
        lbl, val, col = ratio_stats[i]
        px(d, C0X+12, by+12, lbl,      f10, C_GRAY)
        px(d, C0X+12, by+32, str(val), f16, col)

        panel(d, C1X, by, C1W, BH)
        lbl, val, col = pos_stats[i]
        px(d, C1X+12, by+12, lbl,        f10, C_GRAY)
        px(d, C1X+12, by+32, f"{val:,}", f16, col)

        panel(d, C2X, by, C1W, BH)
        lbl, val, col = neg_stats[i]
        px(d, C2X+12, by+12, lbl,        f10, C_GRAY)
        px(d, C2X+12, by+32, f"{val:,}", f16, col)

    ti  = ticket_info
    BY2 = GY + 4*(BH+GAP) + 4
    d.line([(RX, BY2), (W-PAD, BY2)], fill=C_BORDER, width=1)
    BY2 += 10

    panel(d, RX, BY2, RW, H-BY2-PAD)
    t_rows  = [
        ("Total Earned", f"{ti['total_earned']:,}", C_GOLD),
        ("In Stock",     f"{ti['tickets_stock']:,}", C_WHITE),
    ]
    spacing = (RW - 24) // len(t_rows)
    for j, (lbl, val, col) in enumerate(t_rows):
        tx = RX + 12 + j * spacing
        px(d, tx, BY2+8,  lbl, f10, C_GRAY)
        px(d, tx, BY2+26, val, f16, col)

    buf = io.BytesIO()
    img.save(buf, "PNG")
    buf.seek(0)
    return buf


# ---------------------------------------------------------------------------
# MODE SWITCHER UI
# ---------------------------------------------------------------------------

class ModeSelect(discord.ui.Select):
    def __init__(self, ign, player_data, uuid):
        self.ign = ign; self.player_data = player_data; self.uuid = uuid
        options = [discord.SelectOption(label=v, value=k) for k, v in MODE_LABELS.items()]
        super().__init__(placeholder="Switch mode...", options=options, min_values=1, max_values=1)

    async def callback(self, interaction: discord.Interaction):
        await interaction.response.defer()
        mode   = self.values[0]
        bw          = self.player_data.get("stats", {}).get("Bedwars", {})
        lvl, prog   = bedwars_level(bw.get("Experience", 0))
        pc, pn, star = get_prestige(lvl)
        stats       = get_stats(bw, mode)
        ti          = get_ticket_info(bw, self.player_data)
        rank_segs, rank_base = get_rank_display(self.player_data)
        async with aiohttp.ClientSession() as session:
            buf = await generate_stats_image(self.ign, lvl, prog, pc, pn, star, stats, ti, mode, self.uuid, session, rank_segs, rank_base)
        await interaction.followup.edit_message(
            interaction.message.id,
            attachments=[discord.File(buf, "bedwars.png")],
            view=self.view,
        )

class ModeView(discord.ui.View):
    def __init__(self, ign, player_data, uuid):
        super().__init__(timeout=180)
        self.add_item(ModeSelect(ign, player_data, uuid))


# ---------------------------------------------------------------------------
# COG (command implementation)
# ---------------------------------------------------------------------------

class BedWarsCog(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @app_commands.command(name="bedwars", description="Hypixel BedWars stats for a player")
    @app_commands.describe(mc_ign="Minecraft username")
    async def bedwars(self, interaction: discord.Interaction, mc_ign: str):
        await interaction.response.defer()
        async with aiohttp.ClientSession() as session:
            uuid, name = await fetch_uuid(session, mc_ign)
            if not uuid:
                await interaction.followup.send(f"Player not found: `{mc_ign}`.")
                return
            player = await fetch_player(session, uuid)
            if not player:
                await interaction.followup.send(f"No Hypixel data found for `{name}`.")
                return
            bw          = player.get("stats", {}).get("Bedwars", {})
            lvl, prog   = bedwars_level(bw.get("Experience", 0))
            pc, pn, star = get_prestige(lvl)
            stats       = get_stats(bw, "all")
            ti          = get_ticket_info(bw, player)
            rank_segs, rank_base = get_rank_display(player)
            buf         = await generate_stats_image(name, lvl, prog, pc, pn, star, stats, ti, "all", uuid, session, rank_segs, rank_base)
        await interaction.followup.send(
            file=discord.File(buf, "bedwars.png"),
            view=ModeView(name, player, uuid),
        )


async def setup(bot):
    await bot.add_cog(BedWarsCog(bot))
