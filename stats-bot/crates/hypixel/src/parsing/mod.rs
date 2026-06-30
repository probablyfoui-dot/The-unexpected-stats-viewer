// hypixel/src/parsing/mod.rs
// --------------------------
// Module declarations + re-exports for parsing submodules.

// ---- Module Declarations ---- //

pub mod bedwars;
pub mod player;

// ---- BedWars Re-exports ---- //

pub use bedwars::{
    GuildInfo, Mode, ModeStats, Stats, calculate_level, extract, level_progress, ratio,
};
// ---- Player Re-exports ---- //

pub use player::{calculate_network_level, color_code, extract_rank_prefix};

// ---- SkyWars Re-exports ---- //

pub mod skywars;
pub use skywars::{
    SkywarsStats, SwMode, SwModeStats, extract_skywars_stats,
    skywars_level_from_exp, get_sw_prestige,
};
