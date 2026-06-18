"""
cogs/skywars.py
---------------
/skywars command
"""

import io
import aiohttp
import discord
from discord import app_commands
from discord.ext import commands
from PIL import Image, ImageDraw, ImageFont

from core import fetch_uuid, fetch_player, FONT_PATH, FONT_SYMBOLS_PATH
from cogs.hypixel import get_rank_display, draw_segments, segments_width


# ---------------------------------------------------------------------------
# FONT
# ---------------------------------------------------------------------------

import os

def load_font(size=10):
    try:
        return ImageFont.truetype(FONT_PATH, size)
    except Exception:
        return ImageFont.load_default()

def load_symbol_font(size=14):
    try:
        return ImageFont.truetype(FONT_SYMBOLS_PATH, size)
    except Exception:
        return ImageFont.load_default()

def px_star(draw, x, y, text, main_font, star_font, color, anchor="la"):
    """Draw text where star glyphs (✫✪⚝✥) use star_font, everything else uses main_font."""
    STAR_CHARS = set("✫✪⚝✥")
    if anchor == "ra":
        total = sum(
            int(draw.textlength(ch, font=(star_font if ch in STAR_CHARS else main_font)))
            for ch in text
        )
        x -= total
    for ch in text:
        font = star_font if ch in STAR_CHARS else main_font
        draw.text((x, y), ch, font=font, fill=color)
        x += int(draw.textlength(ch, font=font))


# ---------------------------------------------------------------------------
# STAR / LEVEL FORMULA
# ---------------------------------------------------------------------------

SW_TOTAL_XP = [
    0, 10, 35, 75, 125, 250, 500, 1000, 1750, 2750,
    4000, 5550, 7300, 9300, 11800, 14800, 18300, 22300, 26800, 31800,
]
SW_XP_PER_LEVEL_AFTER_20 = 5000

def skywars_level_from_exp(exp: int) -> tuple[int, float]:
    """Exact formula matching the Swift implementation."""
    exp = max(0, int(exp))
    if exp < SW_TOTAL_XP[-1]:
        for i in range(len(SW_TOTAL_XP) - 1):
            this_total = SW_TOTAL_XP[i]
            next_total = SW_TOTAL_XP[i + 1]
            if exp < next_total:
                progress = (exp - this_total) / (next_total - this_total)
                return i + 1, max(0.0, min(1.0, progress))
        return 20, 1.0
    rem = exp - SW_TOTAL_XP[-1]
    extra = rem // SW_XP_PER_LEVEL_AFTER_20
    progress = (rem % SW_XP_PER_LEVEL_AFTER_20) / SW_XP_PER_LEVEL_AFTER_20
    return 20 + extra, progress


# ---------------------------------------------------------------------------
# PRESTIGE COLOURS
# ---------------------------------------------------------------------------

STAR_PRESTIGE = [
    (  0, (150, 150, 150), "Stone"),
    ( 10, (255, 255, 255), "Iron"),
    ( 20, (255, 170,   0), "Gold"),
    ( 30, ( 85, 255, 255), "Diamond"),
    ( 40, ( 85, 255,  85), "Emerald"),
    ( 50, (  0, 170, 170), "Sapphire"),
    ( 60, (255,  85,  85), "Ruby"),
    ( 70, (255, 170, 255), "Crystal"),
    ( 80, (170, 255, 255), "Opal"),
    ( 90, (170,   0, 170), "Amethyst"),
    (100, (255, 170,   0), "Rainbow"),
    (110, (255, 255, 255), "Iron Prime"),
    (120, (255, 170,   0), "Gold Prime"),
    (130, ( 85, 255, 255), "Diamond Prime"),
    (140, ( 85, 255,  85), "Emerald Prime"),
    (150, (  0, 170, 170), "Sapphire Prime"),
    (160, (255,  85,  85), "Ruby Prime"),
    (170, (255, 170, 255), "Crystal Prime"),
    (180, (170, 255, 255), "Opal Prime"),
    (190, (170,   0, 170), "Amethyst Prime"),
    (200, (255, 255, 255), "Mirror"),
    (210, (255, 255, 255), "Light"),
    (220, (255, 170, 100), "Dawn"),
    (230, (170,   0, 170), "Dusk"),
    (240, (255, 255, 255), "Air"),
    (250, ( 85, 255, 255), "Wind"),
    (260, (170,   0, 170), "Nebula"),
    (270, (255, 255,  85), "Thunder"),
    (280, ( 85, 255,  85), "Earth"),
    (290, ( 85, 170, 255), "Water"),
    (300, (255,  85,  85), "Fire"),
    (310, (255, 200,  50), "Sunrise"),
    (320, (100,   0, 200), "Eclipse"),
    (330, ( 85, 255,  85), "Gamma"),
    (340, (200, 100, 255), "Majestic"),
    (350, ( 85, 170, 255), "Adesine"),
    (360, (  0, 170, 170), "Marine"),
    (370, ( 85, 255,  85), "Element"),
    (380, (100,  50, 200), "Galaxy"),
    (390, (255, 170,   0), "Atomic"),
    (400, (255, 100,  50), "Sunset"),
    (410, (255, 255, 255), "Time"),
    (420, ( 85, 255, 255), "Winter"),
    (430, ( 50,  50,  50), "Obsidian"),
    (440, ( 85, 255,  85), "Spring"),
    (450, (170, 255, 255), "Ice"),
    (460, (255, 200,  50), "Summer"),
    (470, (255,  85, 170), "Spinel"),
    (480, (255, 140,  50), "Autumn"),
    (490, (170,   0, 170), "Mystic"),
    (500, (255, 170,   0), "Eternal"),
]

def get_sw_prestige(star: int):
    color, name = (150, 150, 150), "Stone"
    for threshold, col, nm in STAR_PRESTIGE:
        if star >= threshold:
            color, name = col, nm
        else:
            break
    return color, name


# ---------------------------------------------------------------------------
# MODES
# ---------------------------------------------------------------------------

MODE_KEYS = {
    "all":         None,
    "solo":        "eight_one",
    "doubles":     "eight_two",
    "mega":        "mega",
    "ranked":      "ranked",
    "lab":         "lab",
    "lab_doubles": "lab_team",
}

MODE_LABELS = {
    "all":         "Overall",
    "solo":        "Solo",
    "doubles":     "Doubles",
    "mega":        "Mega",
    "ranked":      "Ranked",
    "lab":         "Lab (Solo)",
    "lab_doubles": "Lab (Doubles)",
}


# ---------------------------------------------------------------------------
# STAT HELPERS
# ---------------------------------------------------------------------------

def ratio(a, b):
    return round(a / b, 2) if b else float(a)

def get_sw_stats(sw: dict, mode: str) -> dict:
    if mode == "all":
        w   = sw.get("wins",    0)
        l   = sw.get("losses",  0)
        k   = sw.get("kills",   0)
        d   = sw.get("deaths",  0)
        ass = sw.get("assists", 0)
    else:
        pfx = MODE_KEYS.get(mode, "")
        def g(key):
            return sw.get(f"{pfx}_{key}" if pfx else key, 0)
        w = g("wins"); l = g("losses"); k = g("kills")
        d = g("deaths"); ass = g("assists")

    return dict(
        wins=w, losses=l, wlr=ratio(w, l),
        kills=k, deaths=d, kdr=ratio(k, d),
        assists=ass, games=w + l,
    )


# ---------------------------------------------------------------------------
# DRAW HELPERS
# ---------------------------------------------------------------------------

C_BG       = (12,  12,  20)
C_PANEL    = (22,  24,  38)
C_BORDER   = (45,  50,  75)
C_GREEN    = (80,  230, 120)
C_CYAN     = (0,   200, 255)
C_GOLD     = (255, 200,  50)
C_RED      = (255,  80,  80)
C_WHITE    = (230, 235, 245)
C_GRAY     = (120, 130, 150)
C_DARKGRAY = (60,   65,  85)

def panel(draw, x, y, w, h, color=C_PANEL, border=C_BORDER, radius=6):
    draw.rounded_rectangle([x, y, x+w, y+h], radius=radius, fill=color, outline=border, width=1)

def px(draw, x, y, text, font, color=C_WHITE, anchor="la"):
    draw.text((x, y), text, font=font, fill=color, anchor=anchor)


# ---------------------------------------------------------------------------
# IMAGE GENERATOR
# ---------------------------------------------------------------------------

W, H = 860, 490

async def generate_sw_image(ign, star, progress, prestige_color, prestige_name,
                             stats, mode, rank_segments, rank_base_color):
    img = Image.new("RGB", (W, H), C_BG)
    d   = ImageDraw.Draw(img)

    f10  = load_font(10)
    f12  = load_font(12)
    f22  = load_font(22)
    f16  = load_font(16)
    fs16 = load_symbol_font(16)
    fs10 = load_symbol_font(10)

    PAD = 16
    RX  = PAD
    RW  = W - PAD * 2

    
    d.rectangle([0, 0, W, 4], fill=prestige_color)

    
    STAR = "✫"
    badge_str = f"[{star}{STAR}]"
    badge_w = sum(
        int(d.textlength(ch, font=(fs16 if ch in "✫✪⚝✥" else f16)))
        for ch in badge_str
    )
    bar_y   = 8; bar_h = 22

    seg_w        = segments_width(d, rank_segments, f16) if rank_segments else 0
    ign_w        = int(d.textlength(ign, font=f16))
    name_block_w = seg_w + (8 if rank_segments else 0) + ign_w + 12

    bar_x = RX + badge_w + 8 + name_block_w
    bar_w = RW - (bar_x - RX)

    px_star(d, RX, bar_y + 2, badge_str, f16, fs16, prestige_color)
    draw_segments(d, RX + badge_w + 8, bar_y + 2, rank_segments, f16)
    px(d, RX + badge_w + 8 + seg_w + (8 if rank_segments else 0), bar_y + 2, ign, f16, rank_base_color)

    d.rounded_rectangle([bar_x, bar_y, bar_x+bar_w, bar_y+bar_h], radius=3, fill=C_DARKGRAY)
    if progress > 0:
        d.rounded_rectangle([bar_x, bar_y, bar_x+int(bar_w*progress), bar_y+bar_h],
                            radius=3, fill=prestige_color)
    px_star(d, bar_x+4,       bar_y+5, f"{star}{STAR}",   f10, fs10, C_BG)
    px_star(d, bar_x+bar_w-4, bar_y+5, f"{star+1}{STAR}", f10, fs10, C_BG, anchor="ra")
    px(d, bar_x+bar_w,   bar_y+bar_h+6, MODE_LABELS.get(mode, mode), f12, C_GRAY, anchor="ra")

    sep_y = bar_y + bar_h + 16
    d.line([(RX, sep_y), (W-PAD, sep_y)], fill=C_BORDER, width=1)

    GY  = sep_y + 8; BH = 90; GAP = 8
    C0W = 190; C1W = (RW - C0W - GAP*2) // 2
    C0X = RX;  C1X = RX + C0W + GAP;  C2X = C1X + C1W + GAP

    ratio_stats = [
        ("WLR",     stats["wlr"],     C_GREEN),
        ("KDR",     stats["kdr"],     C_CYAN),
        ("Games",   stats["games"],   C_WHITE),
        ("Assists", stats["assists"], C_GOLD),
    ]
    pos_stats = [
        ("Wins",     stats["wins"],   C_GREEN),
        ("Kills",    stats["kills"],  C_CYAN),
        ("✫ Star",   star,        C_GOLD),
        ("Prestige", prestige_name,   C_WHITE),
    ]
    neg_stats = [
        ("Losses", stats["losses"], C_RED),
        ("Deaths", stats["deaths"], C_WHITE),
        ("Mode",   MODE_LABELS.get(mode, mode), C_GRAY),
        ("",       "",              C_WHITE),
    ]

    for i in range(4):
        by = GY + i * (BH + GAP)

        panel(d, C0X, by, C0W, BH)
        lbl, val, col = ratio_stats[i]
        px(d, C0X+12, by+10, lbl,      f12, C_GRAY)
        px(d, C0X+12, by+32, str(val), f22, col)

        panel(d, C1X, by, C1W, BH)
        lbl, val, col = pos_stats[i]
        px_star(d, C1X+12, by+10, lbl, f12, fs10, C_GRAY)
        px(d, C1X+12, by+32, f"{val:,}" if isinstance(val, int) else str(val), f22, col)

        panel(d, C2X, by, C1W, BH)
        lbl, val, col = neg_stats[i]
        px(d, C2X+12, by+10, lbl, f12, C_GRAY)
        if val:
            px(d, C2X+12, by+32, f"{val:,}" if isinstance(val, int) else str(val), f22, col)

    buf = io.BytesIO()
    img.save(buf, "PNG")
    buf.seek(0)
    return buf


# ---------------------------------------------------------------------------
# MODE SWITCHER UI
# ---------------------------------------------------------------------------

class SWModeSelect(discord.ui.Select):
    def __init__(self, ign, player_data):
        self.ign = ign
        self.player_data = player_data
        options = [discord.SelectOption(label=v, value=k) for k, v in MODE_LABELS.items()]
        super().__init__(placeholder="Switch mode...", options=options, min_values=1, max_values=1)

    async def callback(self, interaction: discord.Interaction):
        await interaction.response.defer()
        mode        = self.values[0]
        sw          = self.player_data.get("stats", {}).get("SkyWars", {})
        star, prog  = skywars_level_from_exp(sw.get("skywars_experience", 0))
        pc, pn      = get_sw_prestige(star)
        stats       = get_sw_stats(sw, mode)
        rank_segs, rank_base = get_rank_display(self.player_data)
        buf         = await generate_sw_image(self.ign, star, prog, pc, pn, stats, mode, rank_segs, rank_base)
        await interaction.followup.edit_message(
            interaction.message.id,
            attachments=[discord.File(buf, "skywars.png")],
            view=self.view,
        )

class SWModeView(discord.ui.View):
    def __init__(self, ign, player_data):
        super().__init__(timeout=180)
        self.add_item(SWModeSelect(ign, player_data))


# ---------------------------------------------------------------------------
# COG
# ---------------------------------------------------------------------------

class SkyWarsCog(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @app_commands.command(name="skywars", description="Hypixel SkyWars stats for a player")
    @app_commands.describe(mc_ign="Minecraft username")
    async def skywars(self, interaction: discord.Interaction, mc_ign: str):
        await interaction.response.defer()
        async with aiohttp.ClientSession() as session:
            uuid, name = await fetch_uuid(session, mc_ign)
            if not uuid:
                await interaction.followup.send(f"Player not found: `{mc_ign}`.")
                return
            player = await fetch_player(session, uuid)
            if not player:
                await interaction.followup.send(f"No Hypixel data for `{name}`.")
                return

            sw          = player.get("stats", {}).get("SkyWars", {})
            star, prog  = skywars_level_from_exp(sw.get("skywars_experience", 0))
            pc, pn      = get_sw_prestige(star)
            stats       = get_sw_stats(sw, "all")
            rank_segs, rank_base = get_rank_display(player)
            buf         = await generate_sw_image(name, star, prog, pc, pn, stats, "all", rank_segs, rank_base)

        await interaction.followup.send(
            file=discord.File(buf, "skywars.png"),
            view=SWModeView(name, player),
        )


async def setup(bot):
    await bot.add_cog(SkyWarsCog(bot))
