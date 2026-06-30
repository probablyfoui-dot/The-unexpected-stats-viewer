// hypixel/src/parsing/skywars.rs
// ------------------------------
// SkyWars stat parsing: modes, combined stats, level/prestige.

// ---- Imports ---- //

/// SkyWars stat parsing.
/// Level formula ported from probablyfoui-dot/The-unexpected-stats-viewer (Python).
use serde_json::Value;

use super::bedwars::GuildInfo;
use super::player::{calculate_network_level, extract_rank_prefix};

// ── Level formula ─────────────────────────────────────────────────────────────

// ---- Prestige/Level ---- //

/// Total cumulative XP required to reach levels 1-20.
const SW_TOTAL_XP: [u64; 20] = [
    0, 10, 35, 75, 125, 250, 500, 1000, 1750, 2750,
    4000, 5550, 7300, 9300, 11800, 14800, 18300, 22300, 26800, 31800,
];
const SW_XP_PER_LEVEL_AFTER_20: u64 = 5000;

/// Returns `(star, progress_0_to_1)` for a raw XP value.
pub fn skywars_level_from_exp(exp: u64) -> (u32, f64) {
    let last = *SW_TOTAL_XP.last().unwrap();
    if exp < last {
        for i in 0..SW_TOTAL_XP.len() - 1 {
            let this = SW_TOTAL_XP[i];
            let next = SW_TOTAL_XP[i + 1];
            if exp < next {
                let progress = (exp - this) as f64 / (next - this) as f64;
                return (i as u32 + 1, progress.clamp(0.0, 1.0));
            }
        }
        return (20, 1.0);
    }
    let rem = exp - last;
    let extra = rem / SW_XP_PER_LEVEL_AFTER_20;
    let progress = (rem % SW_XP_PER_LEVEL_AFTER_20) as f64 / SW_XP_PER_LEVEL_AFTER_20 as f64;
    (20 + extra as u32, progress)
}

// ── Prestige colours (mirror of Python STAR_PRESTIGE table) ───────────────────

pub struct PrestigeInfo {
    pub color_code: &'static str,
    pub name: &'static str,
}

static PRESTIGE_TABLE: &[(u32, &str, &str)] = &[
    (0,   "§7", "Stone"),
    (10,  "§f", "Iron"),
    (20,  "§6", "Gold"),
    (30,  "§b", "Diamond"),
    (40,  "§a", "Emerald"),
    (50,  "§3", "Sapphire"),
    (60,  "§c", "Ruby"),
    (70,  "§d", "Crystal"),
    (80,  "§9", "Opal"),
    (90,  "§5", "Amethyst"),
    (100, "§6", "Rainbow"),
    (110, "§f", "Iron Prime"),
    (120, "§6", "Gold Prime"),
    (130, "§b", "Diamond Prime"),
    (140, "§a", "Emerald Prime"),
    (150, "§3", "Sapphire Prime"),
    (160, "§c", "Ruby Prime"),
    (170, "§d", "Crystal Prime"),
    (180, "§9", "Opal Prime"),
    (190, "§5", "Amethyst Prime"),
    (200, "§f", "Mirror"),
    (210, "§f", "Light"),
    (220, "§6", "Dawn"),
    (230, "§5", "Dusk"),
    (240, "§f", "Air"),
    (250, "§b", "Wind"),
    (260, "§5", "Nebula"),
    (270, "§e", "Thunder"),
    (280, "§a", "Earth"),
    (290, "§9", "Water"),
    (300, "§c", "Fire"),
    (310, "§6", "Sunrise"),
    (320, "§5", "Eclipse"),
    (330, "§a", "Gamma"),
    (340, "§d", "Majestic"),
    (350, "§9", "Adesine"),
    (360, "§3", "Marine"),
    (370, "§a", "Element"),
    (380, "§5", "Galaxy"),
    (390, "§6", "Atomic"),
    (400, "§c", "Sunset"),
    (410, "§f", "Time"),
    (420, "§b", "Winter"),
    (430, "§8", "Obsidian"),
    (440, "§a", "Spring"),
    (450, "§9", "Ice"),
    (460, "§6", "Summer"),
    (470, "§d", "Spinel"),
    (480, "§6", "Autumn"),
    (490, "§5", "Mystic"),
    (500, "§6", "Eternal"),
];

pub fn get_sw_prestige(star: u32) -> PrestigeInfo {
    let mut color_code = "§7";
    let mut name = "Stone";
    for (threshold, col, nm) in PRESTIGE_TABLE {
        if star >= *threshold {
            color_code = col;
            name = nm;
        } else {
            break;
        }
    }
    PrestigeInfo { color_code, name }
}

// ── Stats structures ──────────────────────────────────────────────────────────

// ---- SwModeStats ---- //

fn ratio(a: u64, b: u64) -> f64 {
    if b == 0 { a as f64 } else { a as f64 / b as f64 }
}

#[derive(Clone, Default)]
pub struct SwModeStats {
    pub wins: u64,
    pub losses: u64,
    pub kills: u64,
    pub deaths: u64,
    pub assists: u64,
}

impl SwModeStats {
    pub fn wlr(&self) -> f64 { ratio(self.wins, self.losses) }
    pub fn kdr(&self) -> f64 { ratio(self.kills, self.deaths) }
    pub fn games(&self) -> u64 { self.wins + self.losses }
}

#[derive(Clone)]
pub struct SkywarsStats {
    pub username: String,
    pub display_name: String,
    pub rank_prefix: Option<String>,
    pub star: u32,
    pub progress: f64,
    pub network_level: f64,
    pub achievement_points: u64,
    pub first_login: Option<i64>,
    pub guild: GuildInfo,

    pub overall: SwModeStats,
    pub solos: SwModeStats,
    pub doubles: SwModeStats,
    pub mega: SwModeStats,
    pub labs: SwModeStats,
}

impl SkywarsStats {
    pub fn get_mode_stats(&self, mode: SwMode) -> SwModeStats {
        match mode {
            SwMode::Overall => self.overall.clone(),
            SwMode::Solos => self.solos.clone(),
            SwMode::Doubles => self.doubles.clone(),
            SwMode::Mega => self.mega.clone(),
            SwMode::Labs => self.labs.clone(),
        }
    }

    pub fn get_combined_mode_stats(&self, modes: &[SwMode]) -> SwModeStats {
        if modes.len() == 1 && modes[0] == SwMode::Overall {
            return self.overall.clone();
        }
        let mut combined = SwModeStats::default();
        for mode in modes {
            let ms = self.get_mode_stats(*mode);
            combined.wins += ms.wins;
            combined.losses += ms.losses;
            combined.kills += ms.kills;
            combined.deaths += ms.deaths;
            combined.assists += ms.assists;
        }
        combined
    }
}

pub fn combined_sw_mode_name(modes: &[SwMode]) -> String {
    match modes.len() {
        0 => "Overall".into(),
        1 if modes[0] == SwMode::Overall => "Overall".into(),
        1 => modes[0].display_name().to_string(),
        _ => {
            if modes.len() == 4 {
                "Overall".into()
            } else {
                modes.iter().map(|m| m.display_name()).collect::<Vec<_>>().join("/")
            }
        }
    }
}

// ── Mode enum ─────────────────────────────────────────────────────────────────

// ---- SwMode Enum ---- //

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SwMode {
    #[default]
    Overall,
    Solos,
    Doubles,
    Mega,
    Labs,
}

impl SwMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Overall => "Overall",
            Self::Solos => "Solo",
            Self::Doubles => "Doubles",
            Self::Mega => "Mega",
            Self::Labs => "Labs",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "overall" => Some(Self::Overall),
            "solos" | "solo" => Some(Self::Solos),
            "doubles" => Some(Self::Doubles),
            "mega" => Some(Self::Mega),
            "labs" | "lab" => Some(Self::Labs),
            _ => None,
        }
    }

    pub fn all() -> &'static [SwMode] {
        &[Self::Overall, Self::Solos, Self::Doubles, Self::Mega, Self::Labs]
    }
}

// ── Extraction ────────────────────────────────────────────────────────────────

// ---- extract_skywars_stats() ---- //

fn u64_field(sw: &Value, key: &str) -> u64 {
    sw.get(key)
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
        .unwrap_or(0)
}

fn mode_stats(sw: &Value, suffix: &str) -> SwModeStats {
    let suf = if suffix.is_empty() { String::new() } else { format!("_{suffix}") };
    SwModeStats {
        wins:    u64_field(sw, &format!("wins{suf}")),
        losses:  u64_field(sw, &format!("losses{suf}")),
        kills:   u64_field(sw, &format!("kills{suf}")),
        deaths:  u64_field(sw, &format!("deaths{suf}")),
        assists: u64_field(sw, &format!("assists{suf}")),
    }
}

pub fn extract_skywars_stats(
    username: &str,
    player: &Value,
    guild: Option<GuildInfo>,
) -> Option<SkywarsStats> {
    let sw = player.get("stats")?.get("SkyWars")?;

    let exp = u64_field(sw, "skywars_experience");
    let (star, progress) = skywars_level_from_exp(exp);
    let network_exp = player.get("networkExp").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let network_level = calculate_network_level(network_exp);
    let display_name = player.get("displayname").and_then(|v| v.as_str()).unwrap_or(username).to_string();
    let rank_prefix = extract_rank_prefix(player);
    let achievement_points = player.get("achievementPoints").and_then(|v| v.as_u64()).unwrap_or(0);
    let first_login = player.get("firstLogin").and_then(|v| v.as_i64());

    // Overall totals
    let overall = SwModeStats {
        wins:    u64_field(sw, "wins"),
        losses:  u64_field(sw, "losses"),
        kills:   u64_field(sw, "kills"),
        deaths:  u64_field(sw, "deaths"),
        assists: u64_field(sw, "assists"),
    };

    // Per-mode (API uses suffix pattern: wins_solo, kills_team, etc.)
    let solos   = mode_stats(sw, "solo");
    let doubles = mode_stats(sw, "team");
    let mega    = mode_stats(sw, "mega");
    let labs    = mode_stats(sw, "lab");

    Some(SkywarsStats {
        username: username.to_string(),
        display_name,
        rank_prefix,
        star,
        progress,
        network_level,
        achievement_points,
        first_login,
        guild: guild.unwrap_or_default(),
        overall,
        solos,
        doubles,
        mega,
        labs,
    })
}
