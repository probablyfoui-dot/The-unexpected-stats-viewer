"""
cogs/hypixel.py
---------------
/hypixel command
"""

import io
import math
import aiohttp
import discord
from discord import app_commands
from discord.ext import commands
from PIL import Image, ImageDraw, ImageFont

from core import fetch_uuid, fetch_player, FONT_PATH, FONT_SYMBOLS_PATH

HYPIXEL_GUILD_API = "https://api.hypixel.net/v2/guild"


# ---------------------------------------------------------------------------
# FONT
# ---------------------------------------------------------------------------

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


# ---------------------------------------------------------------------------
# NETWORK LEVEL
# ---------------------------------------------------------------------------

def network_level_from_exp(exp: float) -> tuple[int, float]:
    """
    Hypixel network XP → (level, progress).
      lvl = (sqrt(exp + 15312.5) - 125 / sqrt(2)) / (25 * sqrt(2))
    """
    exp = max(0.0, float(exp))
    lvl = (math.sqrt(exp + 15312.5) - 125.0 / math.sqrt(2.0)) / (25.0 * math.sqrt(2.0))
    floor_lvl = math.floor(lvl)
    return max(1, int(floor_lvl)), lvl - floor_lvl


# ---------------------------------------------------------------------------
# RANK COLOURS
# ---------------------------------------------------------------------------

MC_COLORS = {
    "BLACK":        (  0,   0,   0),
    "DARK_BLUE":    (  0,   0, 170),
    "DARK_GREEN":   (  0, 170,   0),
    "DARK_AQUA":    (  0, 170, 170),
    "DARK_RED":     (170,   0,   0),
    "DARK_PURPLE":  (170,   0, 170),
    "GOLD":         (255, 170,   0),
    "GRAY":         (170, 170, 170),
    "DARK_GRAY":    ( 85,  85,  85),
    "BLUE":         ( 85,  85, 255),
    "GREEN":        ( 85, 255,  85),
    "AQUA":         ( 85, 255, 255),
    "RED":          (255,  85,  85),
    "LIGHT_PURPLE": (255,  85, 255),
    "YELLOW":       (255, 255,  85),
    "WHITE":        (255, 255, 255),
}

C_AQUA = (85, 255, 255)
C_GOLD_MC = (255, 170, 0)

def get_rank_display(player: dict):
    """
    Returns (segments, base_color) where segments is a list of (text, color)
    to be drawn left-to-right, and base_color is used for the accent bar.

    Examples:
      MVP++  → [("[", gold), ("MVP", gold), ("++", black), ("]", gold)]
      MVP+   → [("[", aqua), ("MVP+", aqua), ("+", red), ("]", aqua)]
      MVP    → [("[MVP]", aqua)]
      VIP+   → [("[VIP+]", gold)]
      VIP    → [("[VIP]", green)]
      None   → []
    """
    special = player.get("rank", "NORMAL")
    if special not in ("NORMAL", "", None):
        # Staff / YouTuber ranks
        colors = {
            "ዞ":     (255,  85,  85),
            "YOUTUBE":  (255,  85,  85),
        }
        col = colors.get(special, (150, 150, 150))
        return [("[" + special + "]", col)], col

    monthly = player.get("monthlyPackageRank", "")
    if monthly == "SUPERSTAR":
        # MVP++ : brackets+MVP colored by monthlyRankColor, ++ colored by rankPlusColor
        bracket_col = MC_COLORS.get(player.get("monthlyRankColor", "GOLD"), C_GOLD_MC)
        plus_col    = MC_COLORS.get(player.get("rankPlusColor",    "BLACK"), (0, 0, 0))
        segs = [
            ("[MVP", bracket_col),
            ("++", plus_col),
            ("]",   bracket_col),
        ]
        return segs, bracket_col

    rank = player.get("newPackageRank") or player.get("packageRank") or ""
    if rank == "MVP_PLUS":
        # MVP+ : bracket+MVP+] all aqua, + colored by rankPlusColor
        plus_col = MC_COLORS.get(player.get("rankPlusColor", "RED"), (255, 85, 85))
        segs = [
            ("[MVP", C_AQUA),
            ("+",    plus_col),
            ("]",    C_AQUA),
        ]
        return segs, C_AQUA

    if rank == "MVP":
        return [("[MVP]", C_AQUA)], C_AQUA
    if rank == "VIP_PLUS":
        return [("[VIP+]", C_GOLD_MC)], C_GOLD_MC
    if rank == "VIP":
        col = (85, 255, 85)
        return [("[VIP]", col)], col

    return [], (150, 150, 150)


# ---------------------------------------------------------------------------
# GUILD FETCH
# ---------------------------------------------------------------------------

async def fetch_guild(session: aiohttp.ClientSession, uuid: str, api_key: str) -> str:
    import os
    key = api_key or os.getenv("HYPIXEL_API_KEY", "")
    try:
        async with session.get(
            HYPIXEL_GUILD_API,
            params={"key": key, "player": uuid},
        ) as r:
            data = await r.json()
            guild = data.get("guild")
            if guild:
                return guild.get("name", "None")
    except Exception:
        pass
    return "None"


# ---------------------------------------------------------------------------
# DRAW HELPERS  (identical palette)
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

def draw_segments(d, x, y, segments, font):
    """Draw colored text segments left-to-right, return final x position."""
    for text, color in segments:
        d.text((x, y), text, font=font, fill=color, anchor="la")
        x += int(d.textlength(text, font=font))
    return x

def segments_width(d, segments, font):
    return sum(int(d.textlength(t, font=font)) for t, _ in segments)

W, H = 860, 360

def generate_hypixel_image(ign: str, level: int, progress: float,
                            rank_segments: list, base_color: tuple,
                            achiev: int, karma: int, guild: str,
                            first_login: str) -> io.BytesIO:
    img = Image.new("RGB", (W, H), C_BG)
    d   = ImageDraw.Draw(img)

    f10 = load_font(10)
    f12 = load_font(12)
    f16 = load_font(16)
    f22 = load_font(22)

    PAD = 16
    RX  = PAD
    RW  = W - PAD * 2

    # Top accent bar
    d.rectangle([0, 0, W, 4], fill=base_color)

    bar_y = 8; bar_h = 22
    seg_w = segments_width(d, rank_segments, f16)
    ign_w = int(d.textlength(ign, f16))
    name_end = RX + seg_w + (12 if rank_segments else 0) + ign_w + 20
    bar_x = name_end
    bar_w = RW - (bar_x - RX)

    draw_segments(d, RX, bar_y + 2, rank_segments, f16)
    px(d, RX + seg_w + (12 if rank_segments else 0), bar_y + 2, ign, f16, base_color)

    d.rounded_rectangle([bar_x, bar_y, bar_x+bar_w, bar_y+bar_h], radius=3, fill=C_DARKGRAY)
    if progress > 0:
        d.rounded_rectangle([bar_x, bar_y, bar_x+int(bar_w*progress), bar_y+bar_h],
                            radius=3, fill=base_color)
    px(d, bar_x+4,       bar_y+6, f"Lvl {level}",   f10, C_BG)
    px(d, bar_x+bar_w-4, bar_y+6, f"Lvl {level+1}", f10, C_BG, anchor="ra")

    sep_y = bar_y + bar_h + 14
    d.line([(RX, sep_y), (W-PAD, sep_y)], fill=C_BORDER, width=1)

    # Rank label string for display in panel
    rank_str = "".join(t for t, _ in rank_segments) if rank_segments else "Non-Ranked"

    GY = sep_y + 8; BH = 100; GAP = 8
    CW = (RW - GAP*2) // 3

    cells = [
        ("Network Level", str(level),    C_GOLD),
        ("Rank",          rank_str,       base_color),
        ("Guild",         guild,          C_CYAN),
        ("Achiev. Points",f"{achiev:,}", C_GREEN),
        ("Karma",         f"{karma:,}",  (255, 85, 255)),
        ("First Login",   first_login,   C_GRAY),
    ]

    for i, (lbl, val, col) in enumerate(cells):
        row, col_idx = divmod(i, 3)
        bx = RX + col_idx * (CW + GAP)
        by = GY + row * (BH + GAP)
        panel(d, bx, by, CW, BH)
        px(d, bx+12, by+10, lbl, f12, C_GRAY)
        if i == 1 and rank_segments:
            draw_segments(d, bx+12, by+36, rank_segments, f16)
        else:
            px(d, bx+12, by+36, val, f22, col)

    buf = io.BytesIO()
    img.save(buf, "PNG")
    buf.seek(0)
    return buf


# ---------------------------------------------------------------------------
# COG
# ---------------------------------------------------------------------------

def _fmt_ts(ms) -> str:
    if not ms:
        return "N/A"
    import datetime
    return datetime.datetime.fromtimestamp(int(ms) / 1000).strftime("%Y-%m-%d")


class HypixelCog(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @app_commands.command(name="hypixel", description="General Hypixel profile stats")
    @app_commands.describe(mc_ign="Minecraft username")
    async def hypixel(self, interaction: discord.Interaction, mc_ign: str):
        await interaction.response.defer()

        import os
        api_key = os.getenv("HYPIXEL_API_KEY", "")

        async with aiohttp.ClientSession() as session:
            uuid, name = await fetch_uuid(session, mc_ign)
            if not uuid:
                await interaction.followup.send(f"Player not found: `{mc_ign}`.")
                return
            player = await fetch_player(session, uuid)
            if not player:
                await interaction.followup.send(f"No Hypixel data for `{name}`.")
                return

            guild = await fetch_guild(session, uuid, api_key)

        level, progress = network_level_from_exp(player.get("networkExp", 0))
        rank_segments, base_color = get_rank_display(player)
        achiev  = player.get("achievementPoints", 0)
        karma   = player.get("karma", 0)
        first   = _fmt_ts(player.get("firstLogin"))

        buf = generate_hypixel_image(
            name, level, progress, rank_segments, base_color,
            achiev, karma, guild, first,
        )

        await interaction.followup.send(file=discord.File(buf, "hypixel.png"))


async def setup(bot):
    await bot.add_cog(HypixelCog(bot))
