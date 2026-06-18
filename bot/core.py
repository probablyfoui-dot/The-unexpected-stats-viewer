"""
core.py
-------
Api calls
"""

import os
import urllib.request
import aiohttp
import discord
from discord.ext import commands
from dotenv import load_dotenv

load_dotenv()

DISCORD_TOKEN   = os.getenv("DISCORD_TOKEN")
HYPIXEL_API_KEY = os.getenv("HYPIXEL_API_KEY")

MOJANG_API  = "https://api.mojang.com/users/profiles/minecraft/{}"
HYPIXEL_API = "https://api.hypixel.net/v2/player"

FONT_PATH        = os.path.join(os.path.dirname(__file__), "Inter.ttf")
FONT_SYMBOLS_PATH = os.path.join(os.path.dirname(__file__), "NotoSansSymbols2.ttf")


# ---------------------------------------------------------------------------
# FONT
# ---------------------------------------------------------------------------

def download_font():
    if not os.path.exists(FONT_PATH):
        url = "https://github.com/google/fonts/raw/main/ofl/inter/Inter%5Bopsz%2Cwght%5D.ttf"
        urllib.request.urlretrieve(url, FONT_PATH)
    if not os.path.exists(FONT_SYMBOLS_PATH):
        url = (
            "https://github.com/googlefonts/noto-fonts/raw/main/"
            "hinted/ttf/NotoSansSymbols2/NotoSansSymbols2-Regular.ttf"
        )
        urllib.request.urlretrieve(url, FONT_SYMBOLS_PATH)

# ---------------------------------------------------------------------------
# API CALLS
# ---------------------------------------------------------------------------

async def fetch_uuid(session: aiohttp.ClientSession, username: str):
    async with session.get(MOJANG_API.format(username)) as r:
        if r.status != 200:
            return None, None
        data = await r.json()
        return data.get("id"), data.get("name")

async def fetch_player(session: aiohttp.ClientSession, uuid: str):
    async with session.get(HYPIXEL_API, params={"key": HYPIXEL_API_KEY, "uuid": uuid}) as r:
        data = await r.json()
        return data.get("player") if data.get("success") else None


# ---------------------------------------------------------------------------
# BOT SETUP
# ---------------------------------------------------------------------------

def create_bot():
    intents = discord.Intents.default()
    return commands.Bot(command_prefix="!", intents=intents)


async def main():
    bot = create_bot()

    await bot.load_extension("cogs.bedwars")
    await bot.load_extension("cogs.skywars")
    await bot.load_extension("cogs.hypixel")
    await bot.load_extension("cogs.quent")

    @bot.event
    async def on_ready():
        download_font()
        for guild in bot.guilds:
            bot.tree.copy_global_to(guild=guild)
            await bot.tree.sync(guild=guild)
        print(f"Logged in as {bot.user} — synced to {len(bot.guilds)} server(s)")

    await bot.start(DISCORD_TOKEN)


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
