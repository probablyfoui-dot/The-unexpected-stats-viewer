"""
Hypixel Discord Bot
----------------------------
Command: /bedwars mc_ign:<username>
"""

import os
import io
import asyncio
import aiohttp
import discord
import urllib.request
from discord import app_commands
from discord.ext import commands
from PIL import Image, ImageDraw, ImageFont, ImageFilter

# ---------------------------------------------------------------------------
# CONFIGURATION
# ---------------------------------------------------------------------------
from dotenv import load_dotenv
load_dotenv()

DISCORD_TOKEN   = os.getenv("DISCORD_TOKEN")
HYPIXEL_API_KEY = os.getenv("HYPIXEL_API_KEY")

MOJANG_API    = "https://api.mojang.com/users/profiles/minecraft/{}"
HYPIXEL_API   = "https://api.hypixel.net/v2/player"
SKIN_BODY_URL = "https://crafatar.com/renders/body/{}?overlay&scale=6"

FONT_PATH = os.path.join(os.path.dirname(__file__), "PressStart2P.ttf")

def download_font():
    if not os.path.exists(FONT_PATH):
        url = "https://github.com/google/fonts/raw/main/ofl/pressstart2p/PressStart2P-Regular.ttf"
        urllib.request.urlretrieve(url, FONT_PATH)

def load_font(size=10):
    try:
        return ImageFont.truetype(FONT_PATH, size)
    except Exception:
        return ImageFont.load_default()

# ---------------------------------------------------------------------------
# PRESTIGE
# ---------------------------------------------------------------------------
PRESTIGE_COLORS = {
    0:(150,150,150), 100:(255,255,255), 200:(255,215,0),
    300:(85,255,255), 400:(85,255,85),  500:(0,200,80),
    600:(200,0,200),  700:(255,85,255), 800:(100,200,255),
    900:(255,85,85),  1000:(255,170,0),
}
PRESTIGE_NAMES = {
    0:"Gray",100:"Iron",200:"Gold",300:"Diamond",400:"Emerald",
    500:"Sapphire",600:"Ruby",700:"Crystal",800:"Opal",900:"Amethyst",1000:"Mythic",
}

def get_prestige(level):
    tier = min((int(level)//100)*100, 1000)
    return PRESTIGE_COLORS.get(tier,(150,150,150)), PRESTIGE_NAMES.get(tier,"?")

# ---------------------------------------------------------------------------
# MODE DEFINITIONS
# ---------------------------------------------------------------------------
MODE_KEYS = {
    "all":None,
    "core":["eight_one","eight_two","four_three","four_four"],
    "solo":["eight_one"],"doubles":["eight_two"],
    "threes":["four_three"],"fours":["four_four"],
    "4v4":["two_four"],
    "rush":["eight_two_rush","four_four_rush"],
    "ultimate":["eight_two_ultimate","four_four_ultimate"],
    "armed":["eight_two_armed","four_four_armed"],
    "lucky":["eight_two_lucky","four_four_lucky"],
    "voidless":["eight_two_voidless","four_four_voidless"],
    "swap":["eight_two_swap","four_four_swap"],
    "castle":["castle"],
    "underworld":["eight_two_underworld","four_four_underworld"],
    "totallynormal":["eight_two_totallynormal","four_four_totallynormal"],
}
MODE_LABELS = {
    "all":"All Modes","core":"Core","solo":"Solo","doubles":"Doubles",
    "threes":"Threes","fours":"Fours","4v4":"4v4","rush":"Rush",
    "ultimate":"Ultimate","armed":"Armed","lucky":"Lucky Blocks",
    "voidless":"Voidless","swap":"Swap","castle":"Castle","underworld":"Underworld","totallynormal":"Totally Normal"
}

# ---------------------------------------------------------------------------
# XP → LEVEL
# ---------------------------------------------------------------------------
EASY_XP  = [500,1000,2000,3500]
EASY_SUM = sum(EASY_XP)
XP_PER   = 5000

def bedwars_level(exp):
    if exp < EASY_SUM:
        total = 0
        for i,c in enumerate(EASY_XP):
            if exp < total+c: return i+(exp-total)/c
            total+=c
    exp -= EASY_SUM
    lv = 4.0
    while True:
        base = (int(lv)//100)*100
        lip  = int(lv)-base
        cost = EASY_XP[lip] if lip<4 else XP_PER
        if exp<cost: return lv+exp/cost
        exp-=cost; lv+=1

def xp_for_next(level):
    lip = int(level) % 100
    if lip < 4: return EASY_XP[lip]
    return XP_PER

def xp_progress(exp, level):
    # XP spent to reach current level
    lv = int(level)
    spent = 0
    if lv <= 3:
        for i in range(lv): spent += EASY_XP[i]
        spent += (exp - sum(EASY_XP[:lv])) if lv < 4 else 0
    else:
        spent = EASY_SUM
        for i in range(4, lv):
            base = (i//100)*100; lip = i-base
            spent += EASY_XP[lip] if lip<4 else XP_PER
    base = (lv//100)*100; lip = lv-base
    needed = EASY_XP[lip] if lip<4 else XP_PER
    current_xp_in_level = exp - spent if lv == 0 else exp - spent
    # fraction = level - int(level)
    frac = level - int(level)
    return frac, needed

# ---------------------------------------------------------------------------
# STAT HELPERS
# ---------------------------------------------------------------------------
def ratio(a,b): return round(a/b,2) if b else float(a)

def get_stats(bw, mode):
    if mode=="all":
        w=bw.get("wins_bedwars",0); l=bw.get("losses_bedwars",0)
        fk=bw.get("final_kills_bedwars",0); fd=bw.get("final_deaths_bedwars",0)
        k=bw.get("kills_bedwars",0); d=bw.get("deaths_bedwars",0)
        bb=bw.get("beds_broken_bedwars",0); bl=bw.get("beds_lost_bedwars",0)
    else:
        w=l=fk=fd=k=d=bb=bl=0
        for s in MODE_KEYS.get(mode,[]):
            w+=bw.get(f"{s}_wins_bedwars",0); l+=bw.get(f"{s}_losses_bedwars",0)
            fk+=bw.get(f"{s}_final_kills_bedwars",0); fd+=bw.get(f"{s}_final_deaths_bedwars",0)
            k+=bw.get(f"{s}_kills_bedwars",0); d+=bw.get(f"{s}_deaths_bedwars",0)
            bb+=bw.get(f"{s}_beds_broken_bedwars",0); bl+=bw.get(f"{s}_beds_lost_bedwars",0)
    games = w+l
    return dict(wins=w,losses=l,wlr=ratio(w,l),
                final_kills=fk,final_deaths=fd,fkdr=ratio(fk,fd),
                kills=k,deaths=d,kdr=ratio(k,d),
                beds_broken=bb,beds_lost=bl,bblr=ratio(bb,bl),games=games)

def get_ticket_info(bw, player):
    gg = player.get("gambler_george",{})
    return dict(
        total_earned=bw.get("total_tickets_earned",0),
        tickets_stock=bw.get("tickets",0),
        slumber_spent=bw.get("bedwars_slumber_ticket_master",0),
        gg_win=gg.get("gambler_george_win",False),
        gg_gold=gg.get("blacksmith_golden_ticket",False),
        gg_chess=gg.get("chess_tickets",False),
        gg_golden_count=gg.get("slumber_item_golden_ticket",0),
        gg_ticket_tart=gg.get("slumber_item_ticket_tart",0),
    )

# ---------------------------------------------------------------------------
# COLOR HELPERS
# ---------------------------------------------------------------------------
C_BG       = (12, 12, 20)
C_PANEL    = (22, 24, 38)
C_BORDER   = (45, 50, 75)
C_GREEN    = (80, 230, 120)
C_CYAN     = (0,  200, 255)
C_GOLD     = (255, 200, 50)
C_RED      = (255, 80,  80)
C_WHITE    = (230, 235, 245)
C_GRAY     = (120, 130, 150)
C_DARKGRAY = (60,  65,  85)

def panel(draw, x, y, w, h, color=C_PANEL, border=C_BORDER, radius=6):
    draw.rounded_rectangle([x,y,x+w,y+h], radius=radius, fill=color, outline=border, width=1)

def px(draw, x, y, text, font, color=C_WHITE, anchor="la"):
    draw.text((x,y), text, font=font, fill=color, anchor=anchor)

# ---------------------------------------------------------------------------
# IMAGE GENERATOR
# ---------------------------------------------------------------------------
W, H = 860, 480

async def generate_stats_image(ign, level, prestige_color, prestige_name,
                                stats, ticket_info, mode, uuid, session):
    img = Image.new("RGB", (W,H), C_BG)
    d   = ImageDraw.Draw(img)

    f8  = load_font(8)
    f14 = load_font(14)
    f18 = load_font(18)

    PAD = 16
    RX  = PAD
    RW  = W - PAD*2

    # ── TOP PRESTIGE BAR ──
    d.rectangle([0,0,W,4], fill=prestige_color)

    # ── HEADER: badge + IGN, XP bar ──
    badge   = f"[{int(level)}\u2605]"
    badge_w = int(d.textlength(badge, font=f14))
    ign_w   = int(d.textlength(ign,   font=f14))
    text_w  = badge_w + 12 + ign_w + 20

    bar_h  = 18
    bar_x  = RX + text_w
    bar_y  = 8
    bar_w  = RW - text_w

    px(d, RX,            bar_y+2, badge, f14, prestige_color)
    px(d, RX+badge_w+12, bar_y+2, ign,   f14, C_WHITE)

    frac = level - int(level)
    d.rounded_rectangle([bar_x,bar_y,bar_x+bar_w,bar_y+bar_h], radius=3, fill=C_DARKGRAY)
    if frac > 0:
        d.rounded_rectangle([bar_x,bar_y,bar_x+int(bar_w*frac),bar_y+bar_h], radius=3, fill=prestige_color)
    px(d, bar_x+4,       bar_y+5, f"{int(level)}\u2605",   f8, C_BG)
    px(d, bar_x+bar_w-4, bar_y+5, f"{int(level)+1}\u2605", f8, C_BG, anchor="ra")

    mode_label = MODE_LABELS.get(mode, mode)
    px(d, bar_x+bar_w, bar_y+bar_h+6, mode_label, f8, C_GRAY, anchor="ra")

    d.line([(RX,40),(W-PAD,40)], fill=C_BORDER, width=1)

    # ── STATS GRID ──
    GY   = 48
    BH   = 82
    GAP  = 8
    C0W  = 180  # ratio col width
    C1W  = (RW - C0W - GAP*2) // 2

    ratio_stats = [
        ("WLR",  stats["wlr"],  C_GREEN),
        ("FKDR", stats["fkdr"], C_CYAN),
        ("KDR",  stats["kdr"],  C_WHITE),
        ("BBLR", stats["bblr"], C_GOLD),
    ]
    pos_stats = [
        ("Wins",        stats["wins"],         C_GREEN),
        ("Final Kills", stats["final_kills"],  C_CYAN),
        ("Kills",       stats["kills"],        C_WHITE),
        ("Beds Broken", stats["beds_broken"],  C_GOLD),
    ]
    neg_stats = [
        ("Losses",       stats["losses"],       C_RED),
        ("Final Deaths", stats["final_deaths"], C_WHITE),
        ("Deaths",       stats["deaths"],       C_WHITE),
        ("Beds Lost",    stats["beds_lost"],    C_WHITE),
    ]

    C0X = RX
    C1X = RX + C0W + GAP
    C2X = C1X + C1W + GAP

    for i in range(4):
        by = GY + i*(BH+GAP)

        # Ratio block
        panel(d, C0X, by, C0W, BH)
        lbl,val,col = ratio_stats[i]
        px(d, C0X+12, by+12, lbl,      f8,  C_GRAY)
        px(d, C0X+12, by+34, str(val), f18, col)

        # Positive block
        panel(d, C1X, by, C1W, BH)
        lbl,val,col = pos_stats[i]
        px(d, C1X+12, by+12, lbl,           f8,  C_GRAY)
        px(d, C1X+12, by+34, f"{val:,}",    f14, col)

        # Negative block
        panel(d, C2X, by, C1W, BH)
        lbl,val,col = neg_stats[i]
        px(d, C2X+12, by+12, lbl,           f8,  C_GRAY)
        px(d, C2X+12, by+34, f"{val:,}",    f14, col)

    # ── TICKETS BAR (bottom) ──
    ti  = ticket_info
    BY2 = GY + 4*(BH+GAP) + 4
    d.line([(RX, BY2),(W-PAD, BY2)], fill=C_BORDER, width=1)
    BY2 += 10

    panel(d, RX, BY2, RW, H-BY2-PAD)
    t_rows = [
        ("Total Earned", f"{ti['total_earned']:,}", C_GOLD),
        ("In Stock",     f"{ti['tickets_stock']:,}", C_WHITE),
    ]
    spacing = (RW - 24) // len(t_rows)
    for j,(lbl,val,col) in enumerate(t_rows):
        tx = RX + 12 + j*spacing
        px(d, tx, BY2+8,  lbl, f8,  C_GRAY)
        px(d, tx, BY2+24, val, f14, col)

    buf = io.BytesIO()
    img.save(buf,"PNG")
    buf.seek(0)
    return buf

# ---------------------------------------------------------------------------
# API CALLS
# ---------------------------------------------------------------------------
async def fetch_uuid(session, username):
    async with session.get(MOJANG_API.format(username)) as r:
        if r.status!=200: return None,None
        d = await r.json()
        return d.get("id"), d.get("name")

async def fetch_player(session, uuid):
    async with session.get(HYPIXEL_API, params={"key":HYPIXEL_API_KEY,"uuid":uuid}) as r:
        d = await r.json()
        return d.get("player") if d.get("success") else None

# ---------------------------------------------------------------------------
# BOT SETUP
# ---------------------------------------------------------------------------
intents = discord.Intents.default()
bot = commands.Bot(command_prefix="!", intents=intents)

class ModeSelect(discord.ui.Select):
    def __init__(self, ign, player_data, uuid):
        self.ign, self.player_data, self.uuid = ign, player_data, uuid
        options = [discord.SelectOption(label=v, value=k) for k,v in MODE_LABELS.items()]
        super().__init__(placeholder="Switch mode...", options=options, min_values=1, max_values=1)

    async def callback(self, interaction: discord.Interaction):
        await interaction.response.defer()
        mode = self.values[0]
        bw   = self.player_data.get("stats",{}).get("Bedwars",{})
        lvl  = bedwars_level(bw.get("Experience",0))
        pc,pn = get_prestige(lvl)
        stats = get_stats(bw, mode)
        ti    = get_ticket_info(bw, self.player_data)
        async with aiohttp.ClientSession() as session:
            buf = await generate_stats_image(self.ign,lvl,pc,pn,stats,ti,mode,self.uuid,session)
        await interaction.followup.edit_message(
            interaction.message.id, attachments=[discord.File(buf,"bedwars.png")], view=self.view)

class ModeView(discord.ui.View):
    def __init__(self, ign, player_data, uuid):
        super().__init__(timeout=180)
        self.add_item(ModeSelect(ign, player_data, uuid))

@bot.tree.command(name="bedwars", description="Hypixel BedWars stats for a player")
@app_commands.describe(mc_ign="Minecraft username")
async def bedwars(interaction: discord.Interaction, mc_ign: str):
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
        bw    = player.get("stats",{}).get("Bedwars",{})
        lvl   = bedwars_level(bw.get("Experience",0))
        pc,pn = get_prestige(lvl)
        stats = get_stats(bw,"all")
        ti    = get_ticket_info(bw, player)
        buf   = await generate_stats_image(name,lvl,pc,pn,stats,ti,"all",uuid,session)
    await interaction.followup.send(
        file=discord.File(buf,"bedwars.png"),
        view=ModeView(name, player, uuid))

@bot.event
async def on_ready():
    download_font()
    await bot.tree.sync()
    print(f"Logged in as {bot.user}")

if __name__ == "__main__":
    bot.run(DISCORD_TOKEN)
