// hypixel/src/lib.rs
// ------------------
// Crate root: re-exports for the hypixel data-parsing library.

mod guild;
pub mod parsing;

pub use guild::*;

// ---- BedWars Stats ---- //

pub use parsing::bedwars::{
    CosmeticSlot, Cosmetics, GuildInfo, Mode, ModeStats, SlumberInfo, Stats as BedwarsPlayerStats,
    WinstreakModeData, WinstreakSnapshot, calculate_level, combined_mode_name,
    experience_for_level, extract as extract_bedwars_stats, extract_winstreak_snapshot,
    level_progress, ratio,
};
// ---- Player Utilities ---- //

pub use parsing::player::{calculate_network_level, color_code, extract_rank_prefix};

// ---- SkyWars Stats ---- //

pub use parsing::skywars::{
    SkywarsStats, SwMode, SwModeStats, combined_sw_mode_name, extract_skywars_stats,
    skywars_level_from_exp, get_sw_prestige,
};
