//! Session configuration for game sessions

use crate::session::TimeMode;
use serde::{Deserialize, Serialize};

/// Session configuration with all game parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionConfig {
    // ===== World Generation =====
    /// World size in tiles (default: 64x64)
    pub world_size: (u32, u32),

    /// Random seed for world generation (None = random)
    pub seed: Option<u64>,

    /// Chunk size for spatial partitioning (default: 12x12)
    pub chunk_size: (u32, u32),

    // ===== Entity Density Modifiers =====
    /// Tree density multiplier (base: 0.2 on grass with noise > 0)
    pub tree_density: f32,

    /// Coal spawn probability multiplier (base: 0.15 in mountain)
    pub coal_density: f32,

    /// Iron spawn probability multiplier (base: 0.25 in mountain)
    pub iron_density: f32,

    /// Diamond spawn probability multiplier (base: 0.006 in deep mountain)
    pub diamond_density: f32,

    /// Initial cow spawn probability multiplier (base: 0.015)
    pub cow_density: f32,

    /// Initial zombie spawn probability multiplier (base: 0.007)
    pub zombie_density: f32,

    /// Skeleton spawn probability in tunnels multiplier (base: 0.05)
    pub skeleton_density: f32,

    // ===== Mob Spawn/Despawn Balancing =====
    /// Zombie spawn rate during night (default: 0.3)
    pub zombie_spawn_rate: f32,

    /// Zombie despawn rate (default: 0.4)
    pub zombie_despawn_rate: f32,

    /// Cow spawn rate (default: 0.01)
    pub cow_spawn_rate: f32,

    /// Cow despawn rate when >30 tiles away (default: 0.01, per tick)
    pub cow_despawn_rate: f32,

    // ===== Game Mechanics =====
    /// Episode length in steps (default: 10000, None = infinite)
    pub max_steps: Option<u32>,

    /// Enable day/night cycle (default: true)
    pub day_night_cycle: bool,

    /// Day cycle period in steps (default: 300)
    pub day_cycle_period: u32,

    /// Enable hunger mechanic (default: true)
    pub hunger_enabled: bool,

    /// Hunger rate: steps per food decrement (default: 25)
    pub hunger_rate: u32,

    /// Enable thirst mechanic (default: true)
    pub thirst_enabled: bool,

    /// Thirst rate: steps per drink decrement (default: 20)
    pub thirst_rate: u32,

    /// Enable fatigue/energy mechanic (default: true)
    pub fatigue_enabled: bool,

    // ===== Combat Modifiers =====
    /// Zombie damage multiplier (base: 2, sleeping: 7)
    pub zombie_damage_mult: f32,

    /// Skeleton arrow damage multiplier (base: 2)
    pub arrow_damage_mult: f32,

    /// Player melee damage multiplier (affects sword damage)
    pub player_damage_mult: f32,

    // ===== Mob Health =====
    /// Cow health (default: 3)
    pub cow_health: u8,

    /// Zombie health (default: 5)
    pub zombie_health: u8,

    /// Skeleton health (default: 3)
    pub skeleton_health: u8,

    // ===== Player View =====
    /// Player view radius in tiles (default: 4 = 9x9 grid)
    pub view_radius: u32,

    /// Include full world state vs local view only
    pub full_world_state: bool,

    // ===== Timing =====
    /// Time mode for this session
    pub time_mode: TimeMode,

    /// Default ticks per second for real-time mode (default: 10.0)
    pub default_ticks_per_second: f32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            world_size: (64, 64),
            seed: None,
            chunk_size: (12, 12),
            tree_density: 1.0,
            coal_density: 1.0,
            iron_density: 1.0,
            diamond_density: 1.0,
            cow_density: 1.0,
            zombie_density: 1.0,
            skeleton_density: 1.0,
            zombie_spawn_rate: 0.3,
            zombie_despawn_rate: 0.4,
            cow_spawn_rate: 0.01,
            cow_despawn_rate: 0.01,
            max_steps: Some(10000),
            day_night_cycle: true,
            day_cycle_period: 300,
            hunger_enabled: true,
            hunger_rate: 25,
            thirst_enabled: true,
            thirst_rate: 20,
            fatigue_enabled: true,
            zombie_damage_mult: 1.0,
            arrow_damage_mult: 1.0,
            player_damage_mult: 1.0,
            cow_health: 3,
            zombie_health: 5,
            skeleton_health: 3,
            view_radius: 4,
            full_world_state: false,
            time_mode: TimeMode::Logical,
            default_ticks_per_second: 10.0,
        }
    }
}

impl SessionConfig {
    /// Create a config suitable for fast agent training
    pub fn fast_training() -> Self {
        Self {
            max_steps: Some(1000),
            day_night_cycle: false,
            hunger_enabled: false,
            thirst_enabled: false,
            fatigue_enabled: false,
            time_mode: TimeMode::Logical,
            ..Default::default()
        }
    }

    /// Create a config suitable for human TUI play
    pub fn human_play() -> Self {
        Self {
            time_mode: TimeMode::RealTime {
                ticks_per_second: 10.0,
                pause_on_disconnect: true,
            },
            full_world_state: true,
            ..Default::default()
        }
    }

    /// Create an easy mode config
    pub fn easy() -> Self {
        Self {
            zombie_density: 0.5,
            skeleton_density: 0.5,
            zombie_damage_mult: 0.5,
            arrow_damage_mult: 0.5,
            hunger_rate: 50,
            thirst_rate: 40,
            ..Default::default()
        }
    }

    /// Create a hard mode config
    pub fn hard() -> Self {
        Self {
            zombie_density: 2.0,
            skeleton_density: 2.0,
            zombie_damage_mult: 1.5,
            arrow_damage_mult: 1.5,
            hunger_rate: 15,
            thirst_rate: 12,
            diamond_density: 0.5,
            ..Default::default()
        }
    }
}
