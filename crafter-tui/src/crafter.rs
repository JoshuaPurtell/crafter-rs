use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use crafter_core::image_renderer::{ImageRenderer, ImageRendererConfig};
use crafter_core::recording::{Recording, RecordingOptions, RecordingSession, ReplaySession};
use crafter_core::{Achievements, GameObject, Material, SaveData};
use crafter_core::renderer::{Renderer, TextRenderer};
use crafter_core::{Action, SessionConfig};
use opentui_sys as ot;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::process::Command;

pub const APP_ID: &str = "crafter";
pub const NAME: &str = "Crafter";
pub const SHORT_NAME: &str = "Craft";
pub const HEADER: &str = "Crafter";
pub const DESCRIPTION: &str = "Minecraft-like survival game with crafting.";

pub struct CrafterKeyOutcome {
    pub handled: bool,
    pub graphics_mode_update: Option<bool>,
}

pub enum CrafterCommand {
    Start { config: CrafterConfig },
    Stop,
    StopAndDiscard,
    SetPaused(bool),
    Action(Action),
    Reset,
    // Recording/replay commands
    SaveRecording,
    LoadRecording { path: PathBuf },
    StartReplay { path: PathBuf },
    StopReplay,
    ReplayStep,
    SetReplaySpeed(f32),
    BranchFromReplay,
    ListRecordings,
    // Rendering commands
    SetTileSize(u32),
}

pub enum CrafterUpdate {
    Tick { actual_hz: f32 },
    Status { message: String },
    Running { running: bool },
    Paused { paused: bool },
    InputCapture { capture: bool },
    Frame {
        lines: Vec<String>,
        rgba_data: Option<Vec<u8>>,
        rgba_width: u32,
        rgba_height: u32,
        score: u32,
        health: i32,
        food: i32,
        thirst: i32,
        energy: i32,
        tick: u64,
        achievements: Vec<String>,
        visible_mobs: Vec<MobPreview>,
        density_lines: Vec<String>,
        has_adjacent_table: bool,
        has_adjacent_furnace: bool,
        // Inventory
        inventory: InventoryData,
    },
    Event { message: String },
    // Recording/replay updates
    RecordingSaved { path: PathBuf },
    RecordingsList { recordings: Vec<RecordingInfo> },
    ReplayMode {
        active: bool,
        current_step: usize,
        total_steps: usize,
    },
}

/// Inventory data for display
#[derive(Clone, Debug, Default)]
pub struct InventoryData {
    // Resources
    pub sapling: u8,
    pub wood: u8,
    pub stone: u8,
    pub coal: u8,
    pub iron: u8,
    pub diamond: u8,
    pub sapphire: u8,
    pub ruby: u8,
    // Tools
    pub wood_pickaxe: u8,
    pub stone_pickaxe: u8,
    pub iron_pickaxe: u8,
    pub diamond_pickaxe: u8,
    pub wood_sword: u8,
    pub stone_sword: u8,
    pub iron_sword: u8,
    pub diamond_sword: u8,
    pub bow: u8,
    pub arrows: u8,
    // Armor
    pub armor_helmet: u8,
    pub armor_chestplate: u8,
    pub armor_leggings: u8,
    pub armor_boots: u8,
    // Potions
    pub potion_red: u8,
    pub potion_green: u8,
    pub potion_blue: u8,
    pub potion_pink: u8,
    pub potion_cyan: u8,
    pub potion_yellow: u8,
    // Progression
    pub xp: u32,
    pub level: u8,
    pub stat_points: u8,
}

#[derive(Clone, Debug)]
pub struct MobPreview {
    pub label: String,
    pub detail: Option<String>,
    pub rgba: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

impl InventoryData {
    pub fn from_crafter(inv: &crafter_core::Inventory) -> Self {
        Self {
            sapling: inv.sapling,
            wood: inv.wood,
            stone: inv.stone,
            coal: inv.coal,
            iron: inv.iron,
            diamond: inv.diamond,
            sapphire: inv.sapphire,
            ruby: inv.ruby,
            wood_pickaxe: inv.wood_pickaxe,
            stone_pickaxe: inv.stone_pickaxe,
            iron_pickaxe: inv.iron_pickaxe,
            diamond_pickaxe: inv.diamond_pickaxe,
            wood_sword: inv.wood_sword,
            stone_sword: inv.stone_sword,
            iron_sword: inv.iron_sword,
            diamond_sword: inv.diamond_sword,
            bow: inv.bow,
            arrows: inv.arrows,
            armor_helmet: inv.armor_helmet,
            armor_chestplate: inv.armor_chestplate,
            armor_leggings: inv.armor_leggings,
            armor_boots: inv.armor_boots,
            potion_red: inv.potion_red,
            potion_green: inv.potion_green,
            potion_blue: inv.potion_blue,
            potion_pink: inv.potion_pink,
            potion_cyan: inv.potion_cyan,
            potion_yellow: inv.potion_yellow,
            xp: inv.xp,
            level: inv.level,
            stat_points: inv.stat_points,
        }
    }
}

/// Info about a saved recording for display
#[derive(Clone, Debug)]
pub struct RecordingInfo {
    pub path: PathBuf,
    pub name: String,
    pub total_steps: u64,
    pub total_reward: f32,
    pub timestamp: u64,
    pub total_achievements: u32,
    pub unique_achievements: u32,
}

pub struct CrafterState {
    pub running: bool,
    pub paused: bool,
    pub input_capture: bool,
    pub status: String,
    pub frame_lines: Vec<String>,
    // Graphics mode rendering
    pub frame_rgba: Option<Vec<u8>>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub last_tile_size: u32,
    pub score: u32,
    pub health: i32,
    pub food: i32,
    pub thirst: i32,
    pub energy: i32,
    pub tick: u64,
    pub actual_hz: f32,
    pub events: Vec<String>,
    pub achievements: Vec<String>,
    pub visible_mobs: Vec<MobPreview>,
    pub density_lines: Vec<String>,
    pub last_action: Option<Action>,
    pub has_adjacent_table: bool,
    pub has_adjacent_furnace: bool,
    pub inventory: InventoryData,
    // Recording/replay state
    pub recordings: Vec<RecordingInfo>,
    pub selected_recording: usize,
    pub replay_active: bool,
    pub replay_step: usize,
    pub replay_total: usize,
    pub show_recordings: bool,
    pub recordings_search: String,
    pub recordings_search_active: bool,
    // Menus
    pub show_craft_menu: bool,
    pub craft_selection: usize,
    pub show_place_menu: bool,
    pub place_selection: usize,
    // Config menu
    pub show_config_menu: bool,
    pub config_selection: usize,
    pub config: CrafterConfig,
    pub profile_names: Vec<String>,
    pub profile_index: usize,
    pub rule_configs: Vec<RuleConfigEntry>,
    pub rule_config_index: usize,
    pub show_rule_editor: bool,
    pub rule_editor_index: usize,
    pub rule_editor_path: Option<PathBuf>,
    pub rule_editor_doc: Option<RuleConfigDoc>,
    pub rule_editor_config: Option<SessionConfig>,
}

/// Craft menu items
pub const CRAFT_ITEMS: &[(&str, Action, &str)] = &[
    ("Wood Pickaxe", Action::MakeWoodPickaxe, "table + 1 wood"),
    ("Stone Pickaxe", Action::MakeStonePickaxe, "table + 1 wood + 1 stone"),
    ("Iron Pickaxe", Action::MakeIronPickaxe, "table + furnace + wood/coal/iron"),
    ("Diamond Pickaxe", Action::MakeDiamondPickaxe, "table + wood + diamond"),
    ("Wood Sword", Action::MakeWoodSword, "table + 1 wood"),
    ("Stone Sword", Action::MakeStoneSword, "table + 1 wood + 1 stone"),
    ("Iron Sword", Action::MakeIronSword, "table + furnace + wood/coal/iron"),
    ("Diamond Sword", Action::MakeDiamondSword, "table + wood + 2 diamond"),
    ("Iron Armor", Action::MakeIronArmor, "table + furnace + 3 iron + 3 coal"),
    ("Diamond Armor", Action::MakeDiamondArmor, "table + 3 diamond"),
    ("Bow", Action::MakeBow, "table + 2 wood"),
    ("Arrows", Action::MakeArrow, "table + 1 wood + 1 stone"),
];

/// Place menu items
pub const PLACE_ITEMS: &[(&str, Action, &str)] = &[
    ("Crafting Table", Action::PlaceTable, "craft tools here"),
    ("Furnace", Action::PlaceFurnace, "smelt iron ore"),
    ("Stone", Action::PlaceStone, "block path"),
    ("Plant", Action::PlacePlant, "grow food"),
];

/// Game configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct CrafterConfig {
    pub tick_rate: u32,      // Hz (1-30) - only used in real-time mode
    pub world_width: u32,    // 16-64
    pub world_height: u32,   // 16-64
    pub random_seed: bool,   // true = random, false = fixed
    pub seed: u64,           // fixed seed value
    pub graphics_mode: bool, // true = pixel graphics, false = ASCII
    pub logical_time: bool,  // true = step only on input (for AI), false = real-time
    #[serde(default = "default_rule_config_name")]
    pub rule_config: String, // SessionConfig TOML name/path
}

impl Default for CrafterConfig {
    fn default() -> Self {
        Self {
            tick_rate: 10,
            world_width: 64,
            world_height: 64,
            random_seed: true,
            seed: 42,
            graphics_mode: true,
            logical_time: false,
            rule_config: default_rule_config_name(),
        }
    }
}

fn default_rule_config_name() -> String {
    "classic".to_string()
}

/// Config menu items
pub const CONFIG_ITEMS: &[&str] = &[
    "Profile",        // 0: profile name
    "Rule Config",    // 1: SessionConfig profile
    "Time Mode",      // 2: Logical (AI) vs Real-time
    "Tick Rate",      // 3: Hz (only for real-time)
    "World Width",    // 4
    "World Height",   // 5
    "Seed Mode",      // 6
    "Seed Value",     // 7
    "Graphics Mode",  // 8
    "--- Start Game ---",  // 9
];

impl CrafterState {
    pub fn new() -> Self {
        let (profile_names, profile_index, config) = load_initial_profile();
        let rule_configs = list_rule_configs();
        let mut rule_config_index = rule_config_index(&rule_configs, &config.rule_config);
        if rule_configs.is_empty() {
            rule_config_index = 0;
        }
        Self {
            running: false,
            paused: false,
            input_capture: false,
            status: "[S] Settings  [C] Start  [L] Recordings".to_string(),
            frame_lines: Vec::new(),
            frame_rgba: None,
            frame_width: 0,
            frame_height: 0,
            last_tile_size: 10,
            score: 0,
            health: 9,
            food: 9,
            thirst: 9,
            energy: 9,
            tick: 0,
            actual_hz: 0.0,
            events: Vec::new(),
            achievements: Vec::new(),
            visible_mobs: Vec::new(),
            density_lines: Vec::new(),
            last_action: None,
            has_adjacent_table: false,
            has_adjacent_furnace: false,
            inventory: InventoryData::default(),
            recordings: Vec::new(),
            selected_recording: 0,
            replay_active: false,
            replay_step: 0,
            replay_total: 0,
            show_recordings: false,
            recordings_search: String::new(),
            recordings_search_active: false,
            show_craft_menu: false,
            craft_selection: 0,
            show_place_menu: false,
            place_selection: 0,
            show_config_menu: false,
            config_selection: 0,
            config,
            profile_names,
            profile_index,
            rule_configs,
            rule_config_index,
            show_rule_editor: false,
            rule_editor_index: 0,
            rule_editor_path: None,
            rule_editor_doc: None,
            rule_editor_config: None,
        }
    }
}

fn config_dir_path(app_name: &str) -> PathBuf {
    let mut base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.push(".config");
    base.push(app_name);
    base
}

fn config_file_path() -> PathBuf {
    let mut base = config_dir_path("crafter");
    base.push("config.toml");
    base
}

fn profiles_dir() -> PathBuf {
    let mut base = config_dir_path("crafter");
    base.push("profiles");
    base
}

fn profile_path(profile_name: &str) -> PathBuf {
    if profile_name == "default" {
        config_file_path()
    } else {
        let mut base = profiles_dir();
        base.push(format!("{}.toml", profile_name));
        base
    }
}

fn rule_configs_dir() -> PathBuf {
    let mut base = config_dir_path("crafter");
    base.push("rules");
    base
}

pub fn default_recordings_dir() -> PathBuf {
    let mut base = config_dir_path("crafter");
    base.push("recordings");
    base
}

pub fn mission_control_recordings_dir() -> PathBuf {
    let mut base = config_dir_path("mission-control");
    base.push("crafter");
    base.push("recordings");
    base
}

#[derive(Clone)]
pub struct RuleConfigEntry {
    name: String,
    path: Option<PathBuf>,
    editable: bool,
    extension: String,
}

enum RuleConfigDoc {
    Toml(toml::value::Table),
    Yaml(serde_yaml::Mapping),
}

#[derive(Clone, Copy)]
enum RuleEditorFieldId {
    TreeDensity,
    CoalDensity,
    IronDensity,
    DiamondDensity,
    CowDensity,
    ZombieDensity,
    SkeletonDensity,
    ZombieSpawnRate,
    ZombieDespawnRate,
    CowSpawnRate,
    CowDespawnRate,
    DayNightCycle,
    HungerEnabled,
    ThirstEnabled,
    FatigueEnabled,
    HealthEnabled,
    CraftaxEnabled,
    CraftaxMobsEnabled,
    CraftaxWorldgenEnabled,
    CraftaxItemsEnabled,
    CraftaxCombatEnabled,
    CraftaxChestsEnabled,
    CraftaxPotionsEnabled,
    CraftaxXpEnabled,
    CraftaxAchievementsEnabled,
    CraftaxOrcSoldierDensity,
    CraftaxOrcMageDensity,
    CraftaxKnightDensity,
    CraftaxKnightArcherDensity,
    CraftaxTrollDensity,
    CraftaxBatDensity,
    CraftaxSnailDensity,
    CraftaxSapphireDensity,
    CraftaxRubyDensity,
    CraftaxChestDensity,
    CraftaxPotionDropChance,
    CraftaxArrowDropChance,
    CraftaxGemDropChance,
}

#[derive(Clone, Copy)]
enum RuleEditorFieldKind {
    Float { step: f32, min: f32, max: f32 },
    Bool,
}

#[derive(Clone, Copy)]
struct RuleEditorField {
    id: RuleEditorFieldId,
    label: &'static str,
    kind: RuleEditorFieldKind,
    path: &'static [&'static str],
}

const RULE_EDITOR_FIELDS: &[RuleEditorField] = &[
    RuleEditorField {
        id: RuleEditorFieldId::TreeDensity,
        label: "Tree density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 5.0,
        },
        path: &["tree_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CoalDensity,
        label: "Coal density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 5.0,
        },
        path: &["coal_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::IronDensity,
        label: "Iron density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 5.0,
        },
        path: &["iron_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::DiamondDensity,
        label: "Diamond density",
        kind: RuleEditorFieldKind::Float {
            step: 0.01,
            min: 0.0,
            max: 1.0,
        },
        path: &["diamond_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CowDensity,
        label: "Cow density",
        kind: RuleEditorFieldKind::Float {
            step: 0.01,
            min: 0.0,
            max: 1.0,
        },
        path: &["cow_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::ZombieDensity,
        label: "Zombie density",
        kind: RuleEditorFieldKind::Float {
            step: 0.01,
            min: 0.0,
            max: 2.0,
        },
        path: &["zombie_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::SkeletonDensity,
        label: "Skeleton density",
        kind: RuleEditorFieldKind::Float {
            step: 0.01,
            min: 0.0,
            max: 2.0,
        },
        path: &["skeleton_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::ZombieSpawnRate,
        label: "Zombie spawn rate",
        kind: RuleEditorFieldKind::Float {
            step: 0.01,
            min: 0.0,
            max: 1.0,
        },
        path: &["zombie_spawn_rate"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::ZombieDespawnRate,
        label: "Zombie despawn rate",
        kind: RuleEditorFieldKind::Float {
            step: 0.01,
            min: 0.0,
            max: 1.0,
        },
        path: &["zombie_despawn_rate"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CowSpawnRate,
        label: "Cow spawn rate",
        kind: RuleEditorFieldKind::Float {
            step: 0.01,
            min: 0.0,
            max: 1.0,
        },
        path: &["cow_spawn_rate"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CowDespawnRate,
        label: "Cow despawn rate",
        kind: RuleEditorFieldKind::Float {
            step: 0.01,
            min: 0.0,
            max: 1.0,
        },
        path: &["cow_despawn_rate"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::DayNightCycle,
        label: "Day/night cycle",
        kind: RuleEditorFieldKind::Bool,
        path: &["day_night_cycle"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::HungerEnabled,
        label: "Hunger enabled",
        kind: RuleEditorFieldKind::Bool,
        path: &["hunger_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::ThirstEnabled,
        label: "Thirst enabled",
        kind: RuleEditorFieldKind::Bool,
        path: &["thirst_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::FatigueEnabled,
        label: "Energy enabled",
        kind: RuleEditorFieldKind::Bool,
        path: &["fatigue_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::HealthEnabled,
        label: "Health enabled",
        kind: RuleEditorFieldKind::Bool,
        path: &["health_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxEnabled,
        label: "Craftax enabled",
        kind: RuleEditorFieldKind::Bool,
        path: &["craftax", "enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxMobsEnabled,
        label: "Craftax mobs",
        kind: RuleEditorFieldKind::Bool,
        path: &["craftax", "mobs_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxWorldgenEnabled,
        label: "Craftax worldgen",
        kind: RuleEditorFieldKind::Bool,
        path: &["craftax", "worldgen_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxItemsEnabled,
        label: "Craftax items",
        kind: RuleEditorFieldKind::Bool,
        path: &["craftax", "items_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxCombatEnabled,
        label: "Craftax combat",
        kind: RuleEditorFieldKind::Bool,
        path: &["craftax", "combat_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxChestsEnabled,
        label: "Craftax chests",
        kind: RuleEditorFieldKind::Bool,
        path: &["craftax", "chests_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxPotionsEnabled,
        label: "Craftax potions",
        kind: RuleEditorFieldKind::Bool,
        path: &["craftax", "potions_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxXpEnabled,
        label: "Craftax XP",
        kind: RuleEditorFieldKind::Bool,
        path: &["craftax", "xp_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxAchievementsEnabled,
        label: "Craftax achievements",
        kind: RuleEditorFieldKind::Bool,
        path: &["craftax", "achievements_enabled"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxOrcSoldierDensity,
        label: "Orc soldier density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "orc_soldier_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxOrcMageDensity,
        label: "Orc mage density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "orc_mage_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxKnightDensity,
        label: "Knight density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "knight_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxKnightArcherDensity,
        label: "Knight archer density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "knight_archer_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxTrollDensity,
        label: "Troll density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "troll_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxBatDensity,
        label: "Bat density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "bat_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxSnailDensity,
        label: "Snail density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "snail_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxSapphireDensity,
        label: "Sapphire density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "sapphire_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxRubyDensity,
        label: "Ruby density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "ruby_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxChestDensity,
        label: "Chest density",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 3.0,
        },
        path: &["craftax", "spawn", "chest_density"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxPotionDropChance,
        label: "Potion drop chance",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 1.0,
        },
        path: &["craftax", "loot", "potion_drop_chance"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxArrowDropChance,
        label: "Arrow drop chance",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 1.0,
        },
        path: &["craftax", "loot", "arrow_drop_chance"],
    },
    RuleEditorField {
        id: RuleEditorFieldId::CraftaxGemDropChance,
        label: "Gem drop chance",
        kind: RuleEditorFieldKind::Float {
            step: 0.05,
            min: 0.0,
            max: 1.0,
        },
        path: &["craftax", "loot", "gem_drop_chance"],
    },
];

fn list_profiles() -> Vec<String> {
    let mut profiles = Vec::new();
    let dir = profiles_dir();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    profiles.push(stem.to_string());
                }
            }
        }
    }
    profiles.sort();
    profiles
}

fn list_rule_configs() -> Vec<RuleConfigEntry> {
    let mut configs: BTreeMap<String, RuleConfigEntry> = BTreeMap::new();

    for dir in builtin_rule_dirs() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = rule_config_extension(&path) {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        configs.entry(stem.to_string()).or_insert(RuleConfigEntry {
                            name: stem.to_string(),
                            path: Some(path),
                            editable: false,
                            extension: ext.to_string(),
                        });
                    }
                }
            }
        }
    }

    let user_dir = rule_configs_dir();
    if let Ok(entries) = std::fs::read_dir(&user_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = rule_config_extension(&path) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    configs.insert(
                        stem.to_string(),
                        RuleConfigEntry {
                            name: stem.to_string(),
                            path: Some(path),
                            editable: true,
                            extension: ext.to_string(),
                        },
                    );
                }
            }
        }
    }

    if configs.is_empty() {
        configs.insert(
            default_rule_config_name(),
            RuleConfigEntry {
                name: default_rule_config_name(),
                path: None,
                editable: false,
                extension: "toml".to_string(),
            },
        );
    }

    configs.into_values().collect()
}

fn builtin_rule_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("CRAFTER_CONFIG_DIR").map(PathBuf::from) {
        dirs.push(dir);
    }
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("configs");
        if candidate.exists() {
            dirs.push(candidate);
        }
    }
    dirs
}

fn read_rule_config_doc(path: &Path) -> Option<RuleConfigDoc> {
    let contents = std::fs::read_to_string(path).ok()?;
    match rule_config_extension(path) {
        Some("yaml") | Some("yml") => {
            let value: serde_yaml::Value = serde_yaml::from_str(&contents).ok()?;
            match value {
                serde_yaml::Value::Mapping(map) => Some(RuleConfigDoc::Yaml(map)),
                _ => Some(RuleConfigDoc::Yaml(serde_yaml::Mapping::new())),
            }
        }
        _ => {
            let value: toml::Value = toml::from_str(&contents).ok()?;
            match value {
                toml::Value::Table(table) => Some(RuleConfigDoc::Toml(table)),
                _ => Some(RuleConfigDoc::Toml(toml::value::Table::new())),
            }
        }
    }
}

fn write_rule_config_doc(path: &Path, doc: &RuleConfigDoc) -> bool {
    match doc {
        RuleConfigDoc::Toml(table) => {
            let value = toml::Value::Table(table.clone());
            if let Ok(contents) = toml::to_string_pretty(&value) {
                return std::fs::write(path, contents).is_ok();
            }
        }
        RuleConfigDoc::Yaml(map) => {
            let value = serde_yaml::Value::Mapping(map.clone());
            if let Ok(contents) = serde_yaml::to_string(&value) {
                return std::fs::write(path, contents).is_ok();
            }
        }
    }
    false
}

#[derive(Clone, Copy)]
enum RuleEditorValue {
    Float(f32),
    Bool(bool),
}

fn rule_editor_value(config: &SessionConfig, id: RuleEditorFieldId) -> RuleEditorValue {
    match id {
        RuleEditorFieldId::TreeDensity => RuleEditorValue::Float(config.tree_density),
        RuleEditorFieldId::CoalDensity => RuleEditorValue::Float(config.coal_density),
        RuleEditorFieldId::IronDensity => RuleEditorValue::Float(config.iron_density),
        RuleEditorFieldId::DiamondDensity => RuleEditorValue::Float(config.diamond_density),
        RuleEditorFieldId::CowDensity => RuleEditorValue::Float(config.cow_density),
        RuleEditorFieldId::ZombieDensity => RuleEditorValue::Float(config.zombie_density),
        RuleEditorFieldId::SkeletonDensity => RuleEditorValue::Float(config.skeleton_density),
        RuleEditorFieldId::ZombieSpawnRate => RuleEditorValue::Float(config.zombie_spawn_rate),
        RuleEditorFieldId::ZombieDespawnRate => RuleEditorValue::Float(config.zombie_despawn_rate),
        RuleEditorFieldId::CowSpawnRate => RuleEditorValue::Float(config.cow_spawn_rate),
        RuleEditorFieldId::CowDespawnRate => RuleEditorValue::Float(config.cow_despawn_rate),
        RuleEditorFieldId::DayNightCycle => RuleEditorValue::Bool(config.day_night_cycle),
        RuleEditorFieldId::HungerEnabled => RuleEditorValue::Bool(config.hunger_enabled),
        RuleEditorFieldId::ThirstEnabled => RuleEditorValue::Bool(config.thirst_enabled),
        RuleEditorFieldId::FatigueEnabled => RuleEditorValue::Bool(config.fatigue_enabled),
        RuleEditorFieldId::HealthEnabled => RuleEditorValue::Bool(config.health_enabled),
        RuleEditorFieldId::CraftaxEnabled => RuleEditorValue::Bool(config.craftax.enabled),
        RuleEditorFieldId::CraftaxMobsEnabled => {
            RuleEditorValue::Bool(config.craftax.mobs_enabled)
        }
        RuleEditorFieldId::CraftaxWorldgenEnabled => {
            RuleEditorValue::Bool(config.craftax.worldgen_enabled)
        }
        RuleEditorFieldId::CraftaxItemsEnabled => {
            RuleEditorValue::Bool(config.craftax.items_enabled)
        }
        RuleEditorFieldId::CraftaxCombatEnabled => {
            RuleEditorValue::Bool(config.craftax.combat_enabled)
        }
        RuleEditorFieldId::CraftaxChestsEnabled => {
            RuleEditorValue::Bool(config.craftax.chests_enabled)
        }
        RuleEditorFieldId::CraftaxPotionsEnabled => {
            RuleEditorValue::Bool(config.craftax.potions_enabled)
        }
        RuleEditorFieldId::CraftaxXpEnabled => RuleEditorValue::Bool(config.craftax.xp_enabled),
        RuleEditorFieldId::CraftaxAchievementsEnabled => {
            RuleEditorValue::Bool(config.craftax.achievements_enabled)
        }
        RuleEditorFieldId::CraftaxOrcSoldierDensity => {
            RuleEditorValue::Float(config.craftax.spawn.orc_soldier_density)
        }
        RuleEditorFieldId::CraftaxOrcMageDensity => {
            RuleEditorValue::Float(config.craftax.spawn.orc_mage_density)
        }
        RuleEditorFieldId::CraftaxKnightDensity => {
            RuleEditorValue::Float(config.craftax.spawn.knight_density)
        }
        RuleEditorFieldId::CraftaxKnightArcherDensity => {
            RuleEditorValue::Float(config.craftax.spawn.knight_archer_density)
        }
        RuleEditorFieldId::CraftaxTrollDensity => {
            RuleEditorValue::Float(config.craftax.spawn.troll_density)
        }
        RuleEditorFieldId::CraftaxBatDensity => {
            RuleEditorValue::Float(config.craftax.spawn.bat_density)
        }
        RuleEditorFieldId::CraftaxSnailDensity => {
            RuleEditorValue::Float(config.craftax.spawn.snail_density)
        }
        RuleEditorFieldId::CraftaxSapphireDensity => {
            RuleEditorValue::Float(config.craftax.spawn.sapphire_density)
        }
        RuleEditorFieldId::CraftaxRubyDensity => {
            RuleEditorValue::Float(config.craftax.spawn.ruby_density)
        }
        RuleEditorFieldId::CraftaxChestDensity => {
            RuleEditorValue::Float(config.craftax.spawn.chest_density)
        }
        RuleEditorFieldId::CraftaxPotionDropChance => {
            RuleEditorValue::Float(config.craftax.loot.potion_drop_chance)
        }
        RuleEditorFieldId::CraftaxArrowDropChance => {
            RuleEditorValue::Float(config.craftax.loot.arrow_drop_chance)
        }
        RuleEditorFieldId::CraftaxGemDropChance => {
            RuleEditorValue::Float(config.craftax.loot.gem_drop_chance)
        }
    }
}

fn set_rule_editor_value(config: &mut SessionConfig, id: RuleEditorFieldId, value: RuleEditorValue) {
    match (id, value) {
        (RuleEditorFieldId::TreeDensity, RuleEditorValue::Float(val)) => config.tree_density = val,
        (RuleEditorFieldId::CoalDensity, RuleEditorValue::Float(val)) => config.coal_density = val,
        (RuleEditorFieldId::IronDensity, RuleEditorValue::Float(val)) => config.iron_density = val,
        (RuleEditorFieldId::DiamondDensity, RuleEditorValue::Float(val)) => config.diamond_density = val,
        (RuleEditorFieldId::CowDensity, RuleEditorValue::Float(val)) => config.cow_density = val,
        (RuleEditorFieldId::ZombieDensity, RuleEditorValue::Float(val)) => config.zombie_density = val,
        (RuleEditorFieldId::SkeletonDensity, RuleEditorValue::Float(val)) => config.skeleton_density = val,
        (RuleEditorFieldId::ZombieSpawnRate, RuleEditorValue::Float(val)) => config.zombie_spawn_rate = val,
        (RuleEditorFieldId::ZombieDespawnRate, RuleEditorValue::Float(val)) => config.zombie_despawn_rate = val,
        (RuleEditorFieldId::CowSpawnRate, RuleEditorValue::Float(val)) => config.cow_spawn_rate = val,
        (RuleEditorFieldId::CowDespawnRate, RuleEditorValue::Float(val)) => config.cow_despawn_rate = val,
        (RuleEditorFieldId::DayNightCycle, RuleEditorValue::Bool(val)) => config.day_night_cycle = val,
        (RuleEditorFieldId::HungerEnabled, RuleEditorValue::Bool(val)) => config.hunger_enabled = val,
        (RuleEditorFieldId::ThirstEnabled, RuleEditorValue::Bool(val)) => config.thirst_enabled = val,
        (RuleEditorFieldId::FatigueEnabled, RuleEditorValue::Bool(val)) => config.fatigue_enabled = val,
        (RuleEditorFieldId::HealthEnabled, RuleEditorValue::Bool(val)) => config.health_enabled = val,
        (RuleEditorFieldId::CraftaxEnabled, RuleEditorValue::Bool(val)) => config.craftax.enabled = val,
        (RuleEditorFieldId::CraftaxMobsEnabled, RuleEditorValue::Bool(val)) => {
            config.craftax.mobs_enabled = val;
        }
        (RuleEditorFieldId::CraftaxWorldgenEnabled, RuleEditorValue::Bool(val)) => {
            config.craftax.worldgen_enabled = val;
        }
        (RuleEditorFieldId::CraftaxItemsEnabled, RuleEditorValue::Bool(val)) => {
            config.craftax.items_enabled = val;
        }
        (RuleEditorFieldId::CraftaxCombatEnabled, RuleEditorValue::Bool(val)) => {
            config.craftax.combat_enabled = val;
        }
        (RuleEditorFieldId::CraftaxChestsEnabled, RuleEditorValue::Bool(val)) => {
            config.craftax.chests_enabled = val;
        }
        (RuleEditorFieldId::CraftaxPotionsEnabled, RuleEditorValue::Bool(val)) => {
            config.craftax.potions_enabled = val;
        }
        (RuleEditorFieldId::CraftaxXpEnabled, RuleEditorValue::Bool(val)) => {
            config.craftax.xp_enabled = val;
        }
        (RuleEditorFieldId::CraftaxAchievementsEnabled, RuleEditorValue::Bool(val)) => {
            config.craftax.achievements_enabled = val;
        }
        (RuleEditorFieldId::CraftaxOrcSoldierDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.orc_soldier_density = val;
        }
        (RuleEditorFieldId::CraftaxOrcMageDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.orc_mage_density = val;
        }
        (RuleEditorFieldId::CraftaxKnightDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.knight_density = val;
        }
        (RuleEditorFieldId::CraftaxKnightArcherDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.knight_archer_density = val;
        }
        (RuleEditorFieldId::CraftaxTrollDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.troll_density = val;
        }
        (RuleEditorFieldId::CraftaxBatDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.bat_density = val;
        }
        (RuleEditorFieldId::CraftaxSnailDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.snail_density = val;
        }
        (RuleEditorFieldId::CraftaxSapphireDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.sapphire_density = val;
        }
        (RuleEditorFieldId::CraftaxRubyDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.ruby_density = val;
        }
        (RuleEditorFieldId::CraftaxChestDensity, RuleEditorValue::Float(val)) => {
            config.craftax.spawn.chest_density = val;
        }
        (RuleEditorFieldId::CraftaxPotionDropChance, RuleEditorValue::Float(val)) => {
            config.craftax.loot.potion_drop_chance = val;
        }
        (RuleEditorFieldId::CraftaxArrowDropChance, RuleEditorValue::Float(val)) => {
            config.craftax.loot.arrow_drop_chance = val;
        }
        (RuleEditorFieldId::CraftaxGemDropChance, RuleEditorValue::Float(val)) => {
            config.craftax.loot.gem_drop_chance = val;
        }
        _ => {}
    }
}

fn rule_editor_value_label(config: &SessionConfig, field: RuleEditorField) -> String {
    match rule_editor_value(config, field.id) {
        RuleEditorValue::Float(val) => format!("{:.2}", val),
        RuleEditorValue::Bool(val) => {
            if val {
                "On".to_string()
            } else {
                "Off".to_string()
            }
        }
    }
}

fn adjust_rule_editor_value(
    config: &mut SessionConfig,
    field: RuleEditorField,
    direction: i32,
) {
    match field.kind {
        RuleEditorFieldKind::Bool => {
            let current = rule_editor_value(config, field.id);
            if let RuleEditorValue::Bool(val) = current {
                set_rule_editor_value(config, field.id, RuleEditorValue::Bool(!val));
            }
        }
        RuleEditorFieldKind::Float { step, min, max } => {
            let current = rule_editor_value(config, field.id);
            if let RuleEditorValue::Float(val) = current {
                let delta = step * direction as f32;
                let next = (val + delta).clamp(min, max);
                set_rule_editor_value(config, field.id, RuleEditorValue::Float(next));
            }
        }
    }
}

fn set_doc_value(doc: &mut RuleConfigDoc, path: &[&str], value: RuleEditorValue) {
    match doc {
        RuleConfigDoc::Toml(table) => {
            let toml_value = match value {
                RuleEditorValue::Float(val) => toml::Value::Float(val as f64),
                RuleEditorValue::Bool(val) => toml::Value::Boolean(val),
            };
            set_toml_value(table, path, toml_value);
        }
        RuleConfigDoc::Yaml(map) => {
            let yaml_value = match value {
                RuleEditorValue::Float(val) => serde_yaml::Value::from(val as f64),
                RuleEditorValue::Bool(val) => serde_yaml::Value::Bool(val),
            };
            set_yaml_value(map, path, yaml_value);
        }
    }
}

fn set_toml_value(table: &mut toml::value::Table, path: &[&str], value: toml::Value) {
    if path.is_empty() {
        return;
    }
    if path.len() == 1 {
        table.insert(path[0].to_string(), value);
        return;
    }
    let entry = table
        .entry(path[0].to_string())
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
    if let toml::Value::Table(ref mut inner) = entry {
        set_toml_value(inner, &path[1..], value);
    } else {
        *entry = toml::Value::Table(toml::value::Table::new());
        if let toml::Value::Table(ref mut inner) = entry {
            set_toml_value(inner, &path[1..], value);
        }
    }
}

fn set_yaml_value(map: &mut serde_yaml::Mapping, path: &[&str], value: serde_yaml::Value) {
    if path.is_empty() {
        return;
    }
    let key = serde_yaml::Value::String(path[0].to_string());
    if path.len() == 1 {
        map.insert(key, value);
        return;
    }
    let entry = map
        .entry(key)
        .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    if let serde_yaml::Value::Mapping(ref mut inner) = entry {
        set_yaml_value(inner, &path[1..], value);
    } else {
        *entry = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        if let serde_yaml::Value::Mapping(ref mut inner) = entry {
            set_yaml_value(inner, &path[1..], value);
        }
    }
}

fn open_rule_editor(state: &mut CrafterState) -> bool {
    let mut entry = match state.rule_configs.get(state.rule_config_index) {
        Some(entry) => entry.clone(),
        None => return false,
    };

    if !entry.editable {
        if let Some(name) = create_rule_config_from_selected(state) {
            refresh_rule_configs(state);
            state.rule_config_index = rule_config_index(&state.rule_configs, &name);
            state.config.rule_config = name;
            if let Some(updated) = state.rule_configs.get(state.rule_config_index) {
                entry = updated.clone();
            }
        }
    }

    let path = match entry.path.clone() {
        Some(path) => path,
        None => return false,
    };
    let config = match SessionConfig::load_from_path(&path) {
        Ok(config) => config,
        Err(err) => {
            state.status = format!("Rule config error: {}", err);
            return false;
        }
    };
    let doc = read_rule_config_doc(&path).unwrap_or_else(|| {
        if entry.extension == "yaml" || entry.extension == "yml" {
            RuleConfigDoc::Yaml(serde_yaml::Mapping::new())
        } else {
            RuleConfigDoc::Toml(toml::value::Table::new())
        }
    });

    state.show_rule_editor = true;
    state.rule_editor_index = 0;
    state.rule_editor_path = Some(path);
    state.rule_editor_doc = Some(doc);
    state.rule_editor_config = Some(config);
    state.show_config_menu = false;
    true
}

fn save_rule_editor(state: &mut CrafterState) -> bool {
    let path = match state.rule_editor_path.clone() {
        Some(path) => path,
        None => return false,
    };
    let config = match state.rule_editor_config.as_ref() {
        Some(config) => config,
        None => return false,
    };
    let mut doc = match state.rule_editor_doc.take() {
        Some(doc) => doc,
        None => return false,
    };

    for field in RULE_EDITOR_FIELDS {
        let value = rule_editor_value(config, field.id);
        set_doc_value(&mut doc, field.path, value);
    }

    let ok = write_rule_config_doc(&path, &doc);
    state.rule_editor_doc = Some(doc);
    ok
}

fn rule_config_index(configs: &[RuleConfigEntry], name: &str) -> usize {
    configs
        .iter()
        .position(|config| config.name == name)
        .unwrap_or(0)
}

fn selected_rule_config_name(state: &CrafterState) -> String {
    state
        .rule_configs
        .get(state.rule_config_index)
        .map(|config| config.name.clone())
        .unwrap_or_else(default_rule_config_name)
}

fn selected_rule_config_display_name(state: &CrafterState) -> String {
    state
        .rule_configs
        .get(state.rule_config_index)
        .map(rule_config_display_name)
        .unwrap_or_else(|| format!("{}.toml", default_rule_config_name()))
}

fn rule_config_extension(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => Some("toml"),
        Some("yaml") => Some("yaml"),
        Some("yml") => Some("yml"),
        _ => None,
    }
}

fn rule_config_display_name(entry: &RuleConfigEntry) -> String {
    if entry.extension.is_empty() {
        entry.name.clone()
    } else {
        format!("{}.{}", entry.name, entry.extension)
    }
}

fn refresh_rule_configs(state: &mut CrafterState) {
    state.rule_configs = list_rule_configs();
    state.rule_config_index = rule_config_index(&state.rule_configs, &state.config.rule_config);
    if state.rule_configs.is_empty() {
        state.rule_config_index = 0;
    } else if state.rule_config_index >= state.rule_configs.len() {
        state.rule_config_index = 0;
    }
    state.config.rule_config = selected_rule_config_name(state);
}

fn create_rule_config_from_selected(state: &mut CrafterState) -> Option<String> {
    let ext = state
        .rule_configs
        .get(state.rule_config_index)
        .map(|entry| entry.extension.clone())
        .unwrap_or_else(|| "toml".to_string());
    create_rule_config_from_selected_with_ext(state, &ext)
}

fn create_rule_config_from_selected_with_ext(
    state: &mut CrafterState,
    ext: &str,
) -> Option<String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = format!("custom_{}", timestamp);
    let mut target = rule_configs_dir();
    let _ = std::fs::create_dir_all(&target);
    target.push(format!("{}.{}", name, ext));

    let source = state
        .rule_configs
        .get(state.rule_config_index)
        .and_then(|entry| entry.path.clone());

    let should_copy = source
        .as_ref()
        .and_then(|path| rule_config_extension(path))
        .map(|source_ext| source_ext == ext || (source_ext == "yaml" && ext == "yml") || (source_ext == "yml" && ext == "yaml"))
        .unwrap_or(false);

    if should_copy {
        if let Some(source_path) = source {
            let _ = std::fs::copy(source_path, &target);
        }
    } else {
        let contents = if ext == "yaml" || ext == "yml" {
            format!("base: \"{}\"\n", default_rule_config_name())
        } else {
            format!("base = \"{}\"\n", default_rule_config_name())
        };
        let _ = std::fs::write(&target, contents);
    }

    Some(name)
}

fn delete_selected_rule_config(state: &mut CrafterState) -> bool {
    if let Some(entry) = state.rule_configs.get(state.rule_config_index) {
        if entry.editable {
            if let Some(path) = &entry.path {
                return std::fs::remove_file(path).is_ok();
            }
        }
    }
    false
}

fn edit_selected_rule_config(state: &mut CrafterState) -> bool {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let entry = match state.rule_configs.get(state.rule_config_index) {
        Some(entry) => entry.clone(),
        None => return false,
    };

    let path = if entry.editable {
        entry.path
    } else {
        let name = create_rule_config_from_selected(state);
        refresh_rule_configs(state);
        if let Some(name) = name {
            state.rule_config_index = rule_config_index(&state.rule_configs, &name);
        }
        state
            .rule_configs
            .get(state.rule_config_index)
            .and_then(|entry| entry.path.clone())
    };

    if let Some(path) = path {
        let _ = Command::new(editor).arg(path).status();
        true
    } else {
        false
    }
}
fn load_profile_config(profile_name: &str) -> CrafterConfig {
    let path = profile_path(profile_name);
    if let Ok(contents) = std::fs::read_to_string(path) {
        if let Ok(config) = toml::from_str::<CrafterConfig>(&contents) {
            return config;
        }
    }
    CrafterConfig::default()
}

fn save_profile_config(profile_name: &str, config: &CrafterConfig) {
    let path = profile_path(profile_name);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(contents) = toml::to_string_pretty(config) {
        let _ = std::fs::write(path, contents);
    }
}

fn load_initial_profile() -> (Vec<String>, usize, CrafterConfig) {
    let mut profile_names = vec!["default".to_string()];
    profile_names.extend(list_profiles());
    profile_names.dedup();
    let profile_index = 0;
    let config = load_profile_config(&profile_names[profile_index]);
    (profile_names, profile_index, config)
}

fn load_session_config(game_config: &CrafterConfig) -> Option<SessionConfig> {
    let name = game_config.rule_config.trim();
    if name.is_empty() {
        return Some(SessionConfig::default());
    }

    let direct_path = PathBuf::from(name);
    if direct_path.exists() {
        return SessionConfig::load_from_path(direct_path).ok();
    }

    for ext in ["toml", "yaml", "yml"] {
        let user_path = rule_configs_dir().join(format!("{}.{}", name, ext));
        if user_path.exists() {
            return SessionConfig::load_from_path(user_path).ok();
        }
    }

    SessionConfig::load_named(name).ok()
}

#[derive(Clone, Copy)]
enum AchievementFilterOp {
    Eq,
    Lt,
    Lte,
    Gt,
    Gte,
}

struct RecordingsFilter {
    total: Option<(AchievementFilterOp, u32)>,
    unique: Option<(AchievementFilterOp, u32)>,
}

fn parse_recordings_filter(query: &str) -> RecordingsFilter {
    let mut filter = RecordingsFilter {
        total: None,
        unique: None,
    };

    for token in query.split_whitespace() {
        if let Some(expr) = token
            .strip_prefix("total")
            .or_else(|| token.strip_prefix("t"))
        {
            if let Some(parsed) = parse_filter_expr(expr) {
                filter.total = Some(parsed);
            }
        } else if let Some(expr) = token
            .strip_prefix("unique")
            .or_else(|| token.strip_prefix("u"))
        {
            if let Some(parsed) = parse_filter_expr(expr) {
                filter.unique = Some(parsed);
            }
        }
    }

    filter
}

fn parse_filter_expr(expr: &str) -> Option<(AchievementFilterOp, u32)> {
    let trimmed = expr.strip_prefix(':').unwrap_or(expr);
    let (op, rest) = if let Some(rest) = trimmed.strip_prefix(">=") {
        (AchievementFilterOp::Gte, rest)
    } else if let Some(rest) = trimmed.strip_prefix("<=") {
        (AchievementFilterOp::Lte, rest)
    } else if let Some(rest) = trimmed.strip_prefix('>') {
        (AchievementFilterOp::Gt, rest)
    } else if let Some(rest) = trimmed.strip_prefix('<') {
        (AchievementFilterOp::Lt, rest)
    } else if let Some(rest) = trimmed.strip_prefix('=') {
        (AchievementFilterOp::Eq, rest)
    } else {
        (AchievementFilterOp::Eq, trimmed)
    };

    let value = rest.parse::<u32>().ok()?;
    Some((op, value))
}

fn matches_filter(op: AchievementFilterOp, left: u32, right: u32) -> bool {
    match op {
        AchievementFilterOp::Eq => left == right,
        AchievementFilterOp::Lt => left < right,
        AchievementFilterOp::Lte => left <= right,
        AchievementFilterOp::Gt => left > right,
        AchievementFilterOp::Gte => left >= right,
    }
}

fn filtered_recording_indices(recordings: &[RecordingInfo], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return (0..recordings.len()).collect();
    }

    let filter = parse_recordings_filter(query);
    recordings
        .iter()
        .enumerate()
        .filter_map(|(idx, rec)| {
            if let Some((op, value)) = filter.total {
                if !matches_filter(op, rec.total_achievements, value) {
                    return None;
                }
            }
            if let Some((op, value)) = filter.unique {
                if !matches_filter(op, rec.unique_achievements, value) {
                    return None;
                }
            }
            Some(idx)
        })
        .collect()
}

fn achievement_stats(achievements: &Achievements) -> (u32, u32) {
    let total = Achievements::all_names()
        .iter()
        .filter_map(|name| achievements.get(name))
        .sum();
    let unique = achievements.total_unlocked();
    (total, unique)
}

fn list_recordings(dir: &Path) -> Vec<RecordingInfo> {
    if !dir.exists() {
        return Vec::new();
    }

    let mut recordings = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(recording) = Recording::load_json(&path) {
                    let (total_achievements, unique_achievements) =
                        if let Some(last_state) = recording
                            .steps
                            .last()
                            .and_then(|step| step.state_after.as_ref())
                        {
                            achievement_stats(&last_state.achievements)
                        } else {
                            let mut replay = ReplaySession::from_recording(&recording);
                            while replay.step().is_some() {}
                            let state = replay.get_state();
                            achievement_stats(&state.achievements)
                        };
                    let timestamp = entry
                        .metadata()
                        .and_then(|meta| meta.modified())
                        .and_then(|t| {
                            t.duration_since(std::time::UNIX_EPOCH).map_err(|err| {
                                std::io::Error::new(std::io::ErrorKind::Other, err)
                            })
                        })
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    recordings.push(RecordingInfo {
                        path: path.clone(),
                        name: path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        total_steps: recording.total_steps,
                        total_reward: recording.total_reward,
                        timestamp,
                        total_achievements,
                        unique_achievements,
                    });
                }
            }
        }
    }

    recordings.sort_by(|a, b| {
        b.timestamp
            .cmp(&a.timestamp)
            .then_with(|| b.name.cmp(&a.name))
    });
    recordings
}

fn make_frame_update(
    state: &crafter_core::GameState,
    graphics_mode: bool,
    tile_size: u32,
    reward: f32,
    newly_unlocked: Vec<String>,
) -> CrafterUpdate {
    let visible_mobs = visible_mob_previews(state);
    let density_lines = map_density_lines(state);
    let has_adjacent_table = has_adjacent_table(state);
    let has_adjacent_furnace = has_adjacent_furnace(state);
    if graphics_mode {
        let (rgba_data, pixel_w, pixel_h, _cells_w, _cells_h) =
            render_state_graphics(state, tile_size);
        CrafterUpdate::Frame {
            lines: vec![],
            rgba_data: Some(rgba_data),
            rgba_width: pixel_w,
            rgba_height: pixel_h,
            score: (reward * 100.0) as u32,
            health: state.inventory.health as i32,
            food: state.inventory.food as i32,
            thirst: state.inventory.drink as i32,
            energy: state.inventory.energy as i32,
            tick: state.step,
            achievements: newly_unlocked,
            visible_mobs,
            density_lines,
            has_adjacent_table,
            has_adjacent_furnace,
            inventory: InventoryData::from_crafter(&state.inventory),
        }
    } else {
        let lines = render_state(state);
        CrafterUpdate::Frame {
            lines,
            rgba_data: None,
            rgba_width: 0,
            rgba_height: 0,
            score: (reward * 100.0) as u32,
            health: state.inventory.health as i32,
            food: state.inventory.food as i32,
            thirst: state.inventory.drink as i32,
            energy: state.inventory.energy as i32,
            tick: state.step,
            achievements: newly_unlocked,
            visible_mobs,
            density_lines,
            has_adjacent_table,
            has_adjacent_furnace,
            inventory: InventoryData::from_crafter(&state.inventory),
        }
    }
}

pub fn spawn_crafter_loop(
    cmd_rx: Receiver<CrafterCommand>,
    tx: Sender<CrafterUpdate>,
    recordings_dir: PathBuf,
) {
    thread::spawn(move || {
        let mut running = false;
        let mut paused = false;
        let mut target_hz = 10u32;
        let mut last_tick = Instant::now();
        let mut recording_session: Option<RecordingSession> = None;
        let mut pending_action = Action::Noop;
        let mut frame_width = 64u32;
        let mut frame_height = 64u32;
        let mut current_seed: Option<u64> = None;
        let mut graphics_mode = true;
        let mut tile_size = 10u32;
        let mut logical_time = false;

        let mut replay_session: Option<ReplaySession> = None;
        let mut replay_speed = 1.0f32;
        let mut replay_paused = false;

        loop {
            let timeout = if running && !paused && !replay_paused && !logical_time {
                let hz = (target_hz as f32 * replay_speed).max(1.0);
                Duration::from_secs_f32(1.0 / hz)
            } else {
                Duration::from_millis(250)
            };

            match cmd_rx.recv_timeout(timeout) {
                Ok(cmd) => match cmd {
                    CrafterCommand::Start { config: game_config } => {
                        replay_session = None;

                        running = true;
                        paused = false;
                        target_hz = game_config.tick_rate.clamp(1, 30);
                        frame_width = game_config.world_width.clamp(16, 64);
                        frame_height = game_config.world_height.clamp(16, 64);
                        graphics_mode = game_config.graphics_mode;
                        logical_time = game_config.logical_time;

                        let seed = if game_config.random_seed {
                            None
                        } else {
                            Some(game_config.seed)
                        };
                        current_seed = seed;

                        let session_config = load_session_config(&game_config).unwrap_or_else(|| {
                            SessionConfig {
                                world_size: (frame_width, frame_height),
                                seed,
                                view_radius: 3,
                                ..Default::default()
                            }
                        });
                        let session_config = SessionConfig {
                            world_size: (frame_width, frame_height),
                            seed,
                            view_radius: 3,
                            full_world_state: true,
                            time_mode: if logical_time {
                                crafter_core::TimeMode::Logical
                            } else {
                                crafter_core::TimeMode::RealTime {
                                    ticks_per_second: target_hz as f32,
                                    pause_on_disconnect: true,
                                }
                            },
                            ..session_config
                        };
                        let rec_session =
                            RecordingSession::new(session_config, RecordingOptions::minimal());

                        let initial_state = rec_session.get_state();
                        let initial_frame = make_frame_update(
                            &initial_state,
                            graphics_mode,
                            tile_size,
                            0.0,
                            vec![],
                        );
                        let _ = tx.send(initial_frame);

                        recording_session = Some(rec_session);
                        last_tick = Instant::now();
                        let _ = tx.send(CrafterUpdate::Running { running: true });
                        let _ = tx.send(CrafterUpdate::Paused { paused: false });
                        let _ = tx.send(CrafterUpdate::ReplayMode {
                            active: false,
                            current_step: 0,
                            total_steps: 0,
                        });
                        let status_msg = if logical_time {
                            "Logical mode - step on input"
                        } else {
                            "Recording..."
                        };
                        let _ = tx.send(CrafterUpdate::Status {
                            message: status_msg.to_string(),
                        });
                    }
                    CrafterCommand::Stop => {
                        if let Some(rec_sess) = recording_session.take() {
                            let recording = rec_sess.finish();
                            if recording.total_steps > 0 {
                                save_recording(&recording, &tx, &recordings_dir);
                            }
                        }
                        replay_session = None;
                        running = false;
                        paused = false;
                        let _ = tx.send(CrafterUpdate::Running { running: false });
                        let _ = tx.send(CrafterUpdate::ReplayMode {
                            active: false,
                            current_step: 0,
                            total_steps: 0,
                        });
                        let _ = tx.send(CrafterUpdate::Status {
                            message: "Stopped".to_string(),
                        });
                    }
                    CrafterCommand::StopAndDiscard => {
                        recording_session = None;
                        replay_session = None;
                        running = false;
                        paused = false;
                        let _ = tx.send(CrafterUpdate::Running { running: false });
                        let _ = tx.send(CrafterUpdate::ReplayMode {
                            active: false,
                            current_step: 0,
                            total_steps: 0,
                        });
                        let _ = tx.send(CrafterUpdate::Status {
                            message: "Session discarded".to_string(),
                        });
                    }
                    CrafterCommand::SetPaused(pause) => {
                        paused = pause;
                        replay_paused = pause;
                        if !paused {
                            last_tick = Instant::now();
                        }
                        let _ = tx.send(CrafterUpdate::Paused { paused });
                        let _ = tx.send(CrafterUpdate::Status {
                            message: if paused {
                                "Paused"
                            } else {
                                "Running"
                            }
                            .to_string(),
                        });
                    }
                    CrafterCommand::Action(action) => {
                        if replay_session.is_none() {
                            if logical_time && running && !paused {
                                if let Some(ref mut rec_sess) = recording_session {
                                    let result = rec_sess.step(action);

                                    let game_state = &result.state;
                                    let frame = make_frame_update(
                                        game_state,
                                        graphics_mode,
                                        tile_size,
                                        result.reward,
                                        result.newly_unlocked.clone(),
                                    );
                                    let _ = tx.send(frame);

                                    for ach in &result.newly_unlocked {
                                        let _ = tx.send(CrafterUpdate::Event {
                                            message: format!("Unlocked: {}", ach),
                                        });
                                    }

                                    for event in &result.debug_events {
                                        let _ = tx.send(CrafterUpdate::Event {
                                            message: event.clone(),
                                        });
                                    }

                                    if result.done {
                                        let reason = result
                                            .done_reason
                                            .map(|r| format!("{:?}", r))
                                            .unwrap_or_else(|| "Unknown".to_string());
                                        let _ = tx.send(CrafterUpdate::Event {
                                            message: format!("Game Over: {}", reason),
                                        });

                                        let recording = rec_sess.recording().clone();
                                        save_recording(&recording, &tx, &recordings_dir);

                                        running = false;
                                        let _ = tx.send(CrafterUpdate::Running { running: false });
                                        let _ = tx.send(CrafterUpdate::Status {
                                            message: format!(
                                                "Game Over: {} (saved)",
                                                reason
                                            ),
                                        });
                                    }
                                }
                            } else {
                                pending_action = action;
                            }
                        }
                    }
                    CrafterCommand::Reset => {
                        if let Some(rec_sess) = recording_session.take() {
                            let recording = rec_sess.finish();
                            if recording.total_steps > 0 {
                                save_recording(&recording, &tx, &recordings_dir);
                            }
                        }
                        replay_session = None;

                        let config = SessionConfig {
                            world_size: (frame_width, frame_height),
                            seed: current_seed,
                            view_radius: 3,
                            full_world_state: true,
                            ..Default::default()
                        };
                        recording_session =
                            Some(RecordingSession::new(config, RecordingOptions::minimal()));
                        let _ = tx.send(CrafterUpdate::ReplayMode {
                            active: false,
                            current_step: 0,
                            total_steps: 0,
                        });
                        let _ = tx.send(CrafterUpdate::Status {
                            message: "Recording...".to_string(),
                        });
                    }
                    CrafterCommand::SaveRecording => {
                        if let Some(ref rec_sess) = recording_session {
                            let recording = rec_sess.recording().clone();
                            if recording.total_steps > 0 {
                                save_recording(&recording, &tx, &recordings_dir);
                            }
                        }
                    }
                    CrafterCommand::LoadRecording { path: _ } => {}
                    CrafterCommand::StartReplay { path } => {
                        if let Some(rec_sess) = recording_session.take() {
                            let recording = rec_sess.finish();
                            if recording.total_steps > 0 {
                                save_recording(&recording, &tx, &recordings_dir);
                            }
                        }

                        match Recording::load_json(&path) {
                            Ok(recording) => {
                                let total = recording.total_steps as usize;
                                replay_session = Some(ReplaySession::from_recording(&recording));
                                running = true;
                                replay_paused = false;
                                paused = false;
                                last_tick = Instant::now();
                                let _ = tx.send(CrafterUpdate::Running { running: true });
                                let _ = tx.send(CrafterUpdate::ReplayMode {
                                    active: true,
                                    current_step: 0,
                                    total_steps: total,
                                });
                                let _ = tx.send(CrafterUpdate::Status {
                                    message: "Replaying...".to_string(),
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(CrafterUpdate::Event {
                                    message: format!("Failed to load: {}", e),
                                });
                            }
                        }
                    }
                    CrafterCommand::StopReplay => {
                        replay_session = None;
                        running = false;
                        let _ = tx.send(CrafterUpdate::Running { running: false });
                        let _ = tx.send(CrafterUpdate::ReplayMode {
                            active: false,
                            current_step: 0,
                            total_steps: 0,
                        });
                        let _ = tx.send(CrafterUpdate::Status {
                            message: "Replay stopped".to_string(),
                        });
                    }
                    CrafterCommand::ReplayStep => {
                        if let Some(ref mut replay) = replay_session {
                            if let Some(result) = replay.step() {
                                let state = replay.get_state();
                                let frame = make_frame_update(
                                    &state,
                                    graphics_mode,
                                    tile_size,
                                    result.reward,
                                    result.newly_unlocked.clone(),
                                );
                                let _ = tx.send(frame);
                                let _ = tx.send(CrafterUpdate::ReplayMode {
                                    active: true,
                                    current_step: replay.current_step(),
                                    total_steps: replay.total_steps(),
                                });
                                for ach in &result.newly_unlocked {
                                    let _ = tx.send(CrafterUpdate::Event {
                                        message: format!("Unlocked: {}", ach),
                                    });
                                }
                                for event in &result.debug_events {
                                    let _ = tx.send(CrafterUpdate::Event {
                                        message: event.clone(),
                                    });
                                }
                                if result.done {
                                    let reason = result
                                        .done_reason
                                        .map(|r| format!("{:?}", r))
                                        .unwrap_or_else(|| "Unknown".to_string());
                                    let _ = tx.send(CrafterUpdate::Event {
                                        message: format!("Game Over: {}", reason),
                                    });
                                }
                                if result.done || replay.is_complete() {
                                    let _ = tx.send(CrafterUpdate::Event {
                                        message: "Replay complete".to_string(),
                                    });
                                    replay_session = None;
                                    running = false;
                                    let _ = tx.send(CrafterUpdate::Running { running: false });
                                    let _ = tx.send(CrafterUpdate::ReplayMode {
                                        active: false,
                                        current_step: 0,
                                        total_steps: 0,
                                    });
                                    let _ = tx.send(CrafterUpdate::Status {
                                        message: "Replay complete".to_string(),
                                    });
                                }
                            } else {
                                let _ = tx.send(CrafterUpdate::Event {
                                    message: "Replay complete".to_string(),
                                });
                                replay_session = None;
                                running = false;
                                let _ = tx.send(CrafterUpdate::Running { running: false });
                                let _ = tx.send(CrafterUpdate::ReplayMode {
                                    active: false,
                                    current_step: 0,
                                    total_steps: 0,
                                });
                                let _ = tx.send(CrafterUpdate::Status {
                                    message: "Replay complete".to_string(),
                                });
                            }
                        }
                    }
                    CrafterCommand::SetReplaySpeed(speed) => {
                        replay_speed = speed.clamp(0.1, 10.0);
                    }
                    CrafterCommand::BranchFromReplay => {
                        if let Some(ref replay) = replay_session {
                            if !replay_paused {
                                let _ = tx.send(CrafterUpdate::Event {
                                    message: "Pause replay to branch".to_string(),
                                });
                            } else {
                                let save = SaveData::from_session(replay.session(), None);
                                let session = save.into_session();
                                let rec_sess =
                                    RecordingSession::from_session(session, RecordingOptions::minimal());
                                let state = rec_sess.get_state();
                                let frame = make_frame_update(
                                    &state,
                                    graphics_mode,
                                    tile_size,
                                    0.0,
                                    vec![],
                                );
                                let _ = tx.send(frame);

                                current_seed = rec_sess.session().config.seed;
                                recording_session = Some(rec_sess);
                                replay_session = None;
                                replay_paused = false;
                                running = true;
                                paused = false;
                                pending_action = Action::Noop;
                                let _ = tx.send(CrafterUpdate::Running { running: true });
                                let _ = tx.send(CrafterUpdate::Paused { paused: false });
                                let _ = tx.send(CrafterUpdate::InputCapture { capture: true });
                                let _ = tx.send(CrafterUpdate::ReplayMode {
                                    active: false,
                                    current_step: 0,
                                    total_steps: 0,
                                });
                                let _ = tx.send(CrafterUpdate::Status {
                                    message: "Branched replay".to_string(),
                                });
                            }
                        }
                    }
                    CrafterCommand::ListRecordings => {
                        let recordings = list_recordings(&recordings_dir);
                        let _ = tx.send(CrafterUpdate::RecordingsList { recordings });
                    }
                    CrafterCommand::SetTileSize(new_tile_size) => {
                        tile_size = new_tile_size.clamp(4, 16);
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if running && !paused && !logical_time {
                        let now = Instant::now();
                        let delta = now.duration_since(last_tick);
                        last_tick = now;
                        let secs = delta.as_secs_f32();
                        let actual_hz = if secs > 0.0 { 1.0 / secs } else { 0.0 };
                        let _ = tx.send(CrafterUpdate::Tick { actual_hz });

                        if let Some(ref mut replay) = replay_session {
                            if !replay_paused {
                                if let Some(result) = replay.step() {
                                    let state = replay.get_state();
                                    let frame = make_frame_update(
                                        &state,
                                        graphics_mode,
                                        tile_size,
                                        result.reward,
                                        result.newly_unlocked.clone(),
                                    );
                                    let _ = tx.send(frame);
                                    let _ = tx.send(CrafterUpdate::ReplayMode {
                                        active: true,
                                        current_step: replay.current_step(),
                                        total_steps: replay.total_steps(),
                                    });
                                    for ach in &result.newly_unlocked {
                                        let _ = tx.send(CrafterUpdate::Event {
                                            message: format!("Unlocked: {}", ach),
                                        });
                                    }
                                    for event in &result.debug_events {
                                        let _ = tx.send(CrafterUpdate::Event {
                                            message: event.clone(),
                                        });
                                    }
                                    if result.done {
                                        let reason = result
                                            .done_reason
                                            .map(|r| format!("{:?}", r))
                                            .unwrap_or_else(|| "Unknown".to_string());
                                        let _ = tx.send(CrafterUpdate::Event {
                                            message: format!("Game Over: {}", reason),
                                        });
                                    }

                                    if result.done || replay.is_complete() {
                                        let _ = tx.send(CrafterUpdate::Event {
                                            message: "Replay complete".to_string(),
                                        });
                                        replay_session = None;
                                        running = false;
                                        replay_paused = false;
                                        let _ = tx.send(CrafterUpdate::Running { running: false });
                                        let _ = tx.send(CrafterUpdate::ReplayMode {
                                            active: false,
                                            current_step: 0,
                                            total_steps: 0,
                                        });
                                        let _ = tx.send(CrafterUpdate::Status {
                                            message: "Replay complete".to_string(),
                                        });
                                    }
                                } else {
                                    let _ = tx.send(CrafterUpdate::Event {
                                        message: "Replay complete".to_string(),
                                    });
                                    replay_session = None;
                                    running = false;
                                    replay_paused = false;
                                    let _ = tx.send(CrafterUpdate::Running { running: false });
                                    let _ = tx.send(CrafterUpdate::ReplayMode {
                                        active: false,
                                        current_step: 0,
                                        total_steps: 0,
                                    });
                                    let _ = tx.send(CrafterUpdate::Status {
                                        message: "Replay complete".to_string(),
                                    });
                                }
                            }
                        } else if let Some(ref mut rec_sess) = recording_session {
                            let result = rec_sess.step(pending_action);
                            pending_action = Action::Noop;

                            let game_state = &result.state;
                            let frame = make_frame_update(
                                game_state,
                                graphics_mode,
                                tile_size,
                                result.reward,
                                result.newly_unlocked.clone(),
                            );
                            let _ = tx.send(frame);

                            for ach in &result.newly_unlocked {
                                let _ = tx.send(CrafterUpdate::Event {
                                    message: format!("Unlocked: {}", ach),
                                });
                            }

                            for event in &result.debug_events {
                                let _ = tx.send(CrafterUpdate::Event {
                                    message: event.clone(),
                                });
                            }

                            if result.done {
                                let reason = result
                                    .done_reason
                                    .map(|r| format!("{:?}", r))
                                    .unwrap_or_else(|| "Unknown".to_string());
                                let _ = tx.send(CrafterUpdate::Event {
                                    message: format!("Game Over: {}", reason),
                                });

                                let recording = rec_sess.recording().clone();
                                save_recording(&recording, &tx, &recordings_dir);

                                running = false;
                                let _ = tx.send(CrafterUpdate::Running { running: false });
                                let _ = tx.send(CrafterUpdate::Status {
                                    message: format!("Game Over: {} (saved)", reason),
                                });
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

fn save_recording(recording: &Recording, tx: &Sender<CrafterUpdate>, recordings_dir: &Path) {
    if std::fs::create_dir_all(recordings_dir).is_err() {
        let _ = tx.send(CrafterUpdate::Event {
            message: "Failed to create recordings dir".to_string(),
        });
        return;
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let filename = format!("session_{}.json", timestamp);
    let path = recordings_dir.join(&filename);

    match recording.save_json(&path) {
        Ok(()) => {
            let _ = tx.send(CrafterUpdate::RecordingSaved { path: path.clone() });
            let _ = tx.send(CrafterUpdate::Event {
                message: format!("Saved: {}", filename),
            });
        }
        Err(e) => {
            let _ = tx.send(CrafterUpdate::Event {
                message: format!("Save failed: {}", e),
            });
        }
    }
}

fn render_state(state: &crafter_core::GameState) -> Vec<String> {
    let renderer = TextRenderer::minimal();
    match renderer.render(state) {
        Ok(text) => {
            let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
            let mut in_view = false;
            let mut view_lines = Vec::new();

            for line in &lines {
                if line.starts_with("=== VIEW ===") {
                    in_view = true;
                    continue;
                }
                if in_view {
                    if line.starts_with("===") || line.is_empty() {
                        break;
                    }
                    view_lines.push(line.clone());
                }
            }

            if view_lines.is_empty() {
                lines
            } else {
                view_lines
            }
        }
        Err(_) => vec!["Render error".to_string()],
    }
}

fn visible_mob_previews(state: &crafter_core::GameState) -> Vec<MobPreview> {
    let view = match &state.view {
        Some(view) => view,
        None => return Vec::new(),
    };

    let icon_tile_size = 8u32;
    let renderer = ImageRenderer::new(ImageRendererConfig {
        tile_size: icon_tile_size,
        show_status_bars: false,
        apply_lighting: false,
    });

    let mut previews = std::collections::HashMap::<char, MobPreview>::new();
    for (_, _, obj) in &view.objects {
        if let Some((ch, name, detail)) = mob_info(obj) {
            if previews.contains_key(&ch) {
                continue;
            }
            let (rgba, width, height) = match renderer.render_entity_icon(obj) {
                Some((rgba, width, height)) => (Some(rgba), width, height),
                None => (None, 0, 0),
            };
            previews.insert(
                ch,
                MobPreview {
                    label: format!("{} = {}", ch, name),
                    detail: Some(detail),
                    rgba,
                    width,
                    height,
                },
            );
        }
    }

    let order = [
        ('Z', "Zombie"),
        ('S', "Skeleton"),
        ('C', "Cow (food)"),
        ('O', "Orc"),
        ('M', "Orc Mage"),
        ('K', "Knight"),
        ('A', "Archer"),
        ('t', "Troll"),
        ('B', "Bat"),
        ('N', "Snail"),
    ];

    let mut lines = Vec::new();
    for (ch, _) in order {
        if let Some(preview) = previews.remove(&ch) {
            lines.push(preview);
        }
    }
    lines
}

fn has_adjacent_table(state: &crafter_core::GameState) -> bool {
    if let Some(ref world) = state.world {
        return world.has_adjacent_table(state.player_pos);
    }
    state
        .view
        .as_ref()
        .map(|view| has_adjacent_material_in_view(view, Material::Table))
        .unwrap_or(false)
}

fn has_adjacent_furnace(state: &crafter_core::GameState) -> bool {
    if let Some(ref world) = state.world {
        return world.has_adjacent_furnace(state.player_pos);
    }
    state
        .view
        .as_ref()
        .map(|view| has_adjacent_material_in_view(view, Material::Furnace))
        .unwrap_or(false)
}

fn has_adjacent_material_in_view(
    view: &crafter_core::world::WorldView,
    material: Material,
) -> bool {
    let center = view.radius as i32;
    let neighbors = [
        (center - 1, center),
        (center + 1, center),
        (center, center - 1),
        (center, center + 1),
    ];
    neighbors.iter().any(|&(x, y)| {
        view.is_in_bounds(x, y) && view.get_material(x, y) == Some(material)
    })
}

fn mob_info(obj: &GameObject) -> Option<(char, &'static str, String)> {
    match obj {
        GameObject::Zombie(zombie) => Some((
            'Z',
            "Zombie",
            format!(
                "{}; melee dmg2 (sleep 7) cd5",
                hits_summary(zombie.health)
            ),
        )),
        GameObject::Skeleton(skeleton) => Some((
            'S',
            "Skeleton",
            format!(
                "{}; arrows dmg2, reload4, shoot<=5, flee<=3",
                hits_summary(skeleton.health)
            ),
        )),
        GameObject::Cow(cow) => Some((
            'C',
            "Cow (food)",
            format!("{}; food +6 on kill", hits_summary(cow.health)),
        )),
        GameObject::CraftaxMob(mob) => match mob.kind {
            crafter_core::entity::CraftaxMobKind::OrcSoldier => Some((
                'O',
                "Orc",
                format_craftax_detail(mob.health, mob.kind),
            )),
            crafter_core::entity::CraftaxMobKind::OrcMage => Some((
                'M',
                "Orc Mage",
                format_craftax_detail(mob.health, mob.kind),
            )),
            crafter_core::entity::CraftaxMobKind::Knight => Some((
                'K',
                "Knight",
                format_craftax_detail(mob.health, mob.kind),
            )),
            crafter_core::entity::CraftaxMobKind::KnightArcher => Some((
                'A',
                "Archer",
                format_craftax_detail(mob.health, mob.kind),
            )),
            crafter_core::entity::CraftaxMobKind::Troll => Some((
                't',
                "Troll",
                format_craftax_detail(mob.health, mob.kind),
            )),
            crafter_core::entity::CraftaxMobKind::Bat => Some((
                'B',
                "Bat",
                format_craftax_detail(mob.health, mob.kind),
            )),
            crafter_core::entity::CraftaxMobKind::Snail => Some((
                'N',
                "Snail",
                format_craftax_detail(mob.health, mob.kind),
            )),
        },
        _ => None,
    }
}

fn hits_summary(health: u8) -> String {
    let hits = [
        hits_to_kill(health, 1),
        hits_to_kill(health, 2),
        hits_to_kill(health, 3),
        hits_to_kill(health, 5),
        hits_to_kill(health, 8),
    ];
    format!(
        "HP{} hits H/W/S/I/D={}/{}/{}/{}/{}",
        health, hits[0], hits[1], hits[2], hits[3], hits[4]
    )
}

fn hits_to_kill(health: u8, damage: u8) -> u8 {
    if damage == 0 {
        return 0;
    }
    (health + damage - 1) / damage
}

fn format_craftax_detail(health: u8, kind: crafter_core::entity::CraftaxMobKind) -> String {
    let stats = crafter_core::craftax::mobs::stats(kind);
    let mut detail = hits_summary(health);
    if stats.is_melee() && stats.is_ranged() {
        detail.push_str(&format!(
            "; melee {} ranged {} rng{} cd{}",
            stats.melee_damage, stats.ranged_damage, stats.range, stats.cooldown
        ));
    } else if stats.is_ranged() {
        detail.push_str(&format!(
            "; ranged {} rng{} cd{}",
            stats.ranged_damage, stats.range, stats.cooldown
        ));
    } else if stats.is_melee() {
        detail.push_str(&format!("; melee {} cd{}", stats.melee_damage, stats.cooldown));
    } else {
        detail.push_str("; harmless");
    }
    detail
}

fn craft_menu_indices(crafter: &CrafterState) -> Vec<usize> {
    if !crafter.has_adjacent_table {
        return Vec::new();
    }
    let craftax_items_enabled = load_session_config(&crafter.config)
        .map(|config| config.craftax.enabled && config.craftax.items_enabled)
        .unwrap_or(true);
    let has_furnace = crafter.has_adjacent_furnace;
    let inv = &crafter.inventory;
    let mut indices = Vec::new();
    for (idx, (_, action, _)) in CRAFT_ITEMS.iter().enumerate() {
        let is_craftax_item = matches!(
            action,
            Action::MakeDiamondPickaxe
                | Action::MakeDiamondSword
                | Action::MakeIronArmor
                | Action::MakeDiamondArmor
                | Action::MakeBow
                | Action::MakeArrow
        );
        if is_craftax_item && !craftax_items_enabled {
            continue;
        }

        let can_craft = match action {
            Action::MakeWoodPickaxe => inv.wood >= 1,
            Action::MakeStonePickaxe => inv.wood >= 1 && inv.stone >= 1,
            Action::MakeIronPickaxe => has_furnace && inv.wood >= 1 && inv.coal >= 1 && inv.iron >= 1,
            Action::MakeDiamondPickaxe => inv.wood >= 1 && inv.diamond >= 1,
            Action::MakeWoodSword => inv.wood >= 1,
            Action::MakeStoneSword => inv.wood >= 1 && inv.stone >= 1,
            Action::MakeIronSword => has_furnace && inv.wood >= 1 && inv.coal >= 1 && inv.iron >= 1,
            Action::MakeDiamondSword => inv.wood >= 1 && inv.diamond >= 2,
            Action::MakeIronArmor => has_furnace && inv.iron >= 3 && inv.coal >= 3,
            Action::MakeDiamondArmor => inv.diamond >= 3,
            Action::MakeBow => inv.wood >= 2,
            Action::MakeArrow => inv.wood >= 1 && inv.stone >= 1,
            _ => false,
        };
        if can_craft {
            indices.push(idx);
        }
    }
    indices
}

fn map_density_lines(state: &crafter_core::GameState) -> Vec<String> {
    let world = match &state.world {
        Some(world) => world,
        None => return vec!["Map density unavailable".to_string()],
    };

    let total_tiles = (world.area.0 as usize).saturating_mul(world.area.1 as usize).max(1);
    let mut tree = 0usize;
    let mut coal = 0usize;
    let mut iron = 0usize;
    let mut diamond = 0usize;
    let mut sapphire = 0usize;
    let mut ruby = 0usize;
    let mut chest = 0usize;

    for mat in &world.materials {
        match *mat {
            crafter_core::material::Material::Tree => tree += 1,
            crafter_core::material::Material::Coal => coal += 1,
            crafter_core::material::Material::Iron => iron += 1,
            crafter_core::material::Material::Diamond => diamond += 1,
            crafter_core::material::Material::Sapphire => sapphire += 1,
            crafter_core::material::Material::Ruby => ruby += 1,
            crafter_core::material::Material::Chest => chest += 1,
            _ => {}
        }
    }

    let mut cow = 0usize;
    let mut zombie = 0usize;
    let mut skeleton = 0usize;
    let mut orc = 0usize;
    let mut mage = 0usize;
    let mut knight = 0usize;
    let mut archer = 0usize;
    let mut troll = 0usize;
    let mut bat = 0usize;
    let mut snail = 0usize;

    for obj in world.objects.values() {
        match obj {
            GameObject::Cow(_) => cow += 1,
            GameObject::Zombie(_) => zombie += 1,
            GameObject::Skeleton(_) => skeleton += 1,
            GameObject::CraftaxMob(mob) => match mob.kind {
                crafter_core::entity::CraftaxMobKind::OrcSoldier => orc += 1,
                crafter_core::entity::CraftaxMobKind::OrcMage => mage += 1,
                crafter_core::entity::CraftaxMobKind::Knight => knight += 1,
                crafter_core::entity::CraftaxMobKind::KnightArcher => archer += 1,
                crafter_core::entity::CraftaxMobKind::Troll => troll += 1,
                crafter_core::entity::CraftaxMobKind::Bat => bat += 1,
                crafter_core::entity::CraftaxMobKind::Snail => snail += 1,
            },
            _ => {}
        }
    }

    let entries = [
        ("Tree", tree),
        ("Coal", coal),
        ("Iron", iron),
        ("Diamond", diamond),
        ("Sapphire", sapphire),
        ("Ruby", ruby),
        ("Chest", chest),
        ("Cow", cow),
        ("Zombie", zombie),
        ("Skeleton", skeleton),
        ("Orc", orc),
        ("Mage", mage),
        ("Knight", knight),
        ("Archer", archer),
        ("Troll", troll),
        ("Bat", bat),
        ("Snail", snail),
    ];

    let mut labels = Vec::new();
    for (name, count) in entries {
        let pct = ((count * 100) as f32 / total_tiles as f32).round() as u32;
        let pct_label = if count > 0 && pct == 0 {
            "<1%".to_string()
        } else {
            format!("{}%", pct)
        };
        labels.push(format!("{} {} ({})", name, count, pct_label));
    }

    let max_len = labels.iter().map(|s| s.len()).max().unwrap_or(0);
    let col_width = (max_len + 2).max(10);
    let columns = 2usize;
    let rows = (labels.len() + columns - 1) / columns;
    let mut lines = Vec::new();

    for row in 0..rows {
        let mut line = String::new();
        for col in 0..columns {
            let idx = col * rows + row;
            if idx >= labels.len() {
                continue;
            }
            let entry = &labels[idx];
            if col > 0 {
                let pad = col_width.saturating_sub(line.len());
                for _ in 0..pad {
                    line.push(' ');
                }
            }
            line.push_str(entry);
        }
        lines.push(line);
    }

    lines
}

fn render_state_graphics(
    state: &crafter_core::GameState,
    _tile_size: u32,
) -> (Vec<u8>, u32, u32, u32, u32) {
    let view = match &state.view {
        Some(v) => v,
        None => return (vec![], 0, 0, 0, 0),
    };

    let view_size = view.size() as u32;
    let tile_size = 10u32;

    let config = ImageRendererConfig {
        tile_size,
        show_status_bars: true,
        apply_lighting: true,
    };

    let renderer = ImageRenderer::new(config);
    let rgb_bytes = renderer.render_bytes(state);

    let pixel_w = view_size * tile_size;
    let status_bar_height = tile_size * 2;
    let pixel_h = view_size * tile_size + status_bar_height;

    let expected_rgb_size = (pixel_w * pixel_h * 3) as usize;
    if rgb_bytes.len() != expected_rgb_size {
        return (vec![], 0, 0, 0, 0);
    }

    let cells_w = pixel_w / 2;
    let cells_h = pixel_h / 2;

    let rgba_bytes = rgb_to_rgba(&rgb_bytes);

    (rgba_bytes, pixel_w, pixel_h, cells_w, cells_h)
}

fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let pixel_count = rgb.len() / 3;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for chunk in rgb.chunks_exact(3) {
        rgba.push(chunk[0]);
        rgba.push(chunk[1]);
        rgba.push(chunk[2]);
        rgba.push(255);
    }
    rgba
}

pub fn handle_key(
    crafter: &mut CrafterState,
    key: KeyEvent,
    cmd_tx: &Sender<CrafterCommand>,
) -> CrafterKeyOutcome {
    let mut graphics_mode_update = None;

    if crafter.show_recordings {
        let filtered = filtered_recording_indices(&crafter.recordings, &crafter.recordings_search);
        if crafter.selected_recording >= filtered.len() {
            crafter.selected_recording = filtered.len().saturating_sub(1);
        }
        let handled = match key.code {
            KeyCode::Esc => {
                if crafter.recordings_search_active {
                    crafter.recordings_search_active = false;
                } else {
                    crafter.show_recordings = false;
                }
                true
            }
            KeyCode::Char('/') => {
                crafter.recordings_search_active = true;
                true
            }
            KeyCode::Backspace if crafter.recordings_search_active => {
                crafter.recordings_search.pop();
                crafter.selected_recording = 0;
                true
            }
            KeyCode::Enter => {
                if crafter.recordings_search_active {
                    crafter.recordings_search_active = false;
                } else if let Some(&idx) = filtered.get(crafter.selected_recording) {
                    if let Some(rec) = crafter.recordings.get(idx) {
                        let _ = cmd_tx.send(CrafterCommand::StartReplay {
                            path: rec.path.clone(),
                        });
                        crafter.show_recordings = false;
                    }
                }
                true
            }
            KeyCode::Up | KeyCode::Char('k') if !crafter.recordings_search_active => {
                if crafter.selected_recording > 0 {
                    crafter.selected_recording -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') if !crafter.recordings_search_active => {
                if crafter.selected_recording + 1 < filtered.len() {
                    crafter.selected_recording += 1;
                }
                true
            }
            KeyCode::Char(ch) if crafter.recordings_search_active => {
                if !ch.is_control() {
                    crafter.recordings_search.push(ch);
                    crafter.selected_recording = 0;
                }
                true
            }
            _ => false,
        };
        return CrafterKeyOutcome {
            handled,
            graphics_mode_update,
        };
    }

    if crafter.show_craft_menu {
        let craft_indices = craft_menu_indices(crafter);
        if craft_indices.is_empty() {
            crafter.craft_selection = 0;
        } else if crafter.craft_selection >= craft_indices.len() {
            crafter.craft_selection = craft_indices.len().saturating_sub(1);
        }
        let handled = match key.code {
            KeyCode::Esc => {
                crafter.show_craft_menu = false;
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if crafter.craft_selection > 0 {
                    crafter.craft_selection -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if crafter.craft_selection + 1 < craft_indices.len() {
                    crafter.craft_selection += 1;
                }
                true
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(&idx) = craft_indices.get(crafter.craft_selection) {
                    let (_, action, _) = CRAFT_ITEMS[idx];
                    let _ = cmd_tx.send(CrafterCommand::Action(action));
                    crafter.show_craft_menu = false;
                }
                true
            }
            _ => false,
        };
        return CrafterKeyOutcome {
            handled,
            graphics_mode_update,
        };
    }

    if crafter.show_place_menu {
        let handled = match key.code {
            KeyCode::Esc => {
                crafter.show_place_menu = false;
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if crafter.place_selection > 0 {
                    crafter.place_selection -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if crafter.place_selection + 1 < PLACE_ITEMS.len() {
                    crafter.place_selection += 1;
                }
                true
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some((_, action, _)) = PLACE_ITEMS.get(crafter.place_selection) {
                    let _ = cmd_tx.send(CrafterCommand::Action(*action));
                    crafter.show_place_menu = false;
                }
                true
            }
            _ => false,
        };
        return CrafterKeyOutcome {
            handled,
            graphics_mode_update,
        };
    }

    if crafter.show_rule_editor {
        let handled = match key.code {
            KeyCode::Esc => {
                crafter.show_rule_editor = false;
                crafter.show_config_menu = true;
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if crafter.rule_editor_index > 0 {
                    crafter.rule_editor_index -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if crafter.rule_editor_index + 1 < RULE_EDITOR_FIELDS.len() {
                    crafter.rule_editor_index += 1;
                }
                true
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(config) = crafter.rule_editor_config.as_mut() {
                    let field = RULE_EDITOR_FIELDS[crafter.rule_editor_index];
                    adjust_rule_editor_value(config, field, -1);
                }
                true
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                if let Some(config) = crafter.rule_editor_config.as_mut() {
                    let field = RULE_EDITOR_FIELDS[crafter.rule_editor_index];
                    adjust_rule_editor_value(config, field, 1);
                }
                true
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if save_rule_editor(crafter) {
                    refresh_rule_configs(crafter);
                    crafter.status = "Saved rule config".to_string();
                } else {
                    crafter.status = "Failed to save rule config".to_string();
                }
                crafter.show_rule_editor = false;
                crafter.show_config_menu = true;
                true
            }
            _ => false,
        };
        return CrafterKeyOutcome {
            handled,
            graphics_mode_update,
        };
    }

    if crafter.show_config_menu {
        let handled = match key.code {
            KeyCode::Esc => {
                crafter.show_config_menu = false;
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if crafter.config_selection > 0 {
                    crafter.config_selection -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if crafter.config_selection + 1 < CONFIG_ITEMS.len() {
                    crafter.config_selection += 1;
                }
                true
            }
            KeyCode::Left | KeyCode::Char('h') => {
                match crafter.config_selection {
                    0 => {
                        if !crafter.profile_names.is_empty() {
                            if crafter.profile_index == 0 {
                                crafter.profile_index = crafter.profile_names.len() - 1;
                            } else {
                                crafter.profile_index -= 1;
                            }
                            crafter.config = load_profile_config(
                                &crafter.profile_names[crafter.profile_index],
                            );
                            refresh_rule_configs(&mut *crafter);
                            graphics_mode_update = Some(crafter.config.graphics_mode);
                        }
                    }
                    1 => {
                        if !crafter.rule_configs.is_empty() {
                            if crafter.rule_config_index == 0 {
                                crafter.rule_config_index = crafter.rule_configs.len() - 1;
                            } else {
                                crafter.rule_config_index -= 1;
                            }
                            crafter.config.rule_config = selected_rule_config_name(crafter);
                        }
                    }
                    2 => crafter.config.logical_time = !crafter.config.logical_time,
                    3 => {
                        crafter.config.tick_rate =
                            crafter.config.tick_rate.saturating_sub(1).max(1);
                    }
                    4 => {
                        crafter.config.world_width =
                            crafter.config.world_width.saturating_sub(4).max(16);
                    }
                    5 => {
                        crafter.config.world_height =
                            crafter.config.world_height.saturating_sub(4).max(16);
                    }
                    6 => crafter.config.random_seed = !crafter.config.random_seed,
                    7 => crafter.config.seed = crafter.config.seed.saturating_sub(1),
                    8 => {
                        crafter.config.graphics_mode = !crafter.config.graphics_mode;
                        graphics_mode_update = Some(crafter.config.graphics_mode);
                    }
                    _ => {}
                }
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                match crafter.config_selection {
                    0 => {
                        if !crafter.profile_names.is_empty() {
                            crafter.profile_index =
                                (crafter.profile_index + 1) % crafter.profile_names.len();
                            crafter.config = load_profile_config(
                                &crafter.profile_names[crafter.profile_index],
                            );
                            refresh_rule_configs(&mut *crafter);
                            graphics_mode_update = Some(crafter.config.graphics_mode);
                        }
                    }
                    1 => {
                        if !crafter.rule_configs.is_empty() {
                            crafter.rule_config_index =
                                (crafter.rule_config_index + 1) % crafter.rule_configs.len();
                            crafter.config.rule_config = selected_rule_config_name(crafter);
                        }
                    }
                    2 => crafter.config.logical_time = !crafter.config.logical_time,
                    3 => crafter.config.tick_rate = (crafter.config.tick_rate + 1).min(30),
                    4 => crafter.config.world_width = (crafter.config.world_width + 4).min(64),
                    5 => crafter.config.world_height =
                        crafter.config.world_height.saturating_add(4).min(64),
                    6 => crafter.config.random_seed = !crafter.config.random_seed,
                    7 => crafter.config.seed = crafter.config.seed.saturating_add(1),
                    8 => {
                        crafter.config.graphics_mode = !crafter.config.graphics_mode;
                        graphics_mode_update = Some(crafter.config.graphics_mode);
                    }
                    _ => {}
                }
                true
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if crafter.config_selection == 1 {
                    if let Some(name) = create_rule_config_from_selected(crafter) {
                        refresh_rule_configs(crafter);
                        crafter.rule_config_index = rule_config_index(&crafter.rule_configs, &name);
                        crafter.config.rule_config = name;
                        crafter.status = "Created rule config".to_string();
                    }
                    true
                } else {
                    false
                }
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if crafter.config_selection == 1 {
                    if let Some(name) =
                        create_rule_config_from_selected_with_ext(crafter, "yaml")
                    {
                        refresh_rule_configs(crafter);
                        crafter.rule_config_index = rule_config_index(&crafter.rule_configs, &name);
                        crafter.config.rule_config = name;
                        crafter.status = "Created rule config (yaml)".to_string();
                    }
                    true
                } else {
                    false
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if crafter.config_selection == 1 {
                    if delete_selected_rule_config(crafter) {
                        refresh_rule_configs(crafter);
                        crafter.status = "Deleted rule config".to_string();
                    } else {
                        crafter.status = "Cannot delete rule config".to_string();
                    }
                    true
                } else {
                    false
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if crafter.config_selection == 1 {
                    if open_rule_editor(crafter) {
                        crafter.status = "Editing rule config".to_string();
                    } else {
                        crafter.status = "Cannot edit rule config".to_string();
                    }
                    true
                } else {
                    false
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if crafter.config_selection == CONFIG_ITEMS.len() - 1 {
                    crafter.show_config_menu = false;
                    if let Some(profile_name) =
                        crafter.profile_names.get(crafter.profile_index)
                    {
                        save_profile_config(profile_name, &crafter.config);
                    }
                    crafter.config.rule_config = selected_rule_config_name(crafter);
                    let _ = cmd_tx.send(CrafterCommand::Start {
                        config: crafter.config.clone(),
                    });
                    crafter.input_capture = true;
                }
                true
            }
            _ => false,
        };
        return CrafterKeyOutcome {
            handled,
            graphics_mode_update,
        };
    }

    let handled = match key.code {
        KeyCode::Char('s') | KeyCode::Char('S') => {
            if !crafter.running && !crafter.input_capture {
                crafter.show_config_menu = true;
                crafter.config_selection = 0;
            } else if crafter.input_capture {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::MoveDown));
                crafter.last_action = Some(Action::MoveDown);
            }
            true
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            if !crafter.input_capture {
                let _ = cmd_tx.send(CrafterCommand::ListRecordings);
                crafter.show_recordings = true;
            }
            true
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if crafter.input_capture && crafter.running && !crafter.replay_active {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::PlaceTable));
            } else if !crafter.input_capture && !crafter.replay_active {
                crafter.show_craft_menu = true;
                crafter.craft_selection = 0;
            }
            true
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            if crafter.input_capture && crafter.running && !crafter.replay_active {
                if crafter.has_adjacent_table {
                    crafter.show_craft_menu = true;
                    crafter.craft_selection = 0;
                } else {
                    crafter.status = "Need a table nearby to craft.".to_string();
                }
            }
            true
        }
        KeyCode::Char('b') | KeyCode::Char('B') => {
            if crafter.replay_active && crafter.paused {
                let _ = cmd_tx.send(CrafterCommand::BranchFromReplay);
            } else if crafter.input_capture && crafter.running && !crafter.replay_active {
                crafter.show_place_menu = true;
                crafter.place_selection = 0;
            }
            true
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if crafter.replay_active {
                let _ = cmd_tx.send(CrafterCommand::StopReplay);
            }
            if !crafter.running {
                let _ = cmd_tx.send(CrafterCommand::Start {
                    config: crafter.config.clone(),
                });
                crafter.input_capture = true;
            } else if crafter.input_capture {
                if crafter.has_adjacent_table {
                    crafter.show_craft_menu = true;
                    crafter.craft_selection = 0;
                } else {
                    crafter.status = "Need a table nearby to craft.".to_string();
                }
            } else {
                crafter.input_capture = !crafter.input_capture;
            }
            true
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            if crafter.running && crafter.paused {
                let _ = cmd_tx.send(CrafterCommand::SetPaused(false));
            } else if crafter.input_capture && crafter.running && !crafter.replay_active {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::PlacePlant));
            } else if crafter.running {
                let _ = cmd_tx.send(CrafterCommand::SetPaused(!crafter.paused));
            }
            true
        }
        KeyCode::Backspace | KeyCode::Delete => {
            if crafter.running && crafter.paused && !crafter.replay_active {
                let _ = cmd_tx.send(CrafterCommand::StopAndDiscard);
                crafter.input_capture = false;
            }
            true
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if crafter.input_capture && crafter.running && !crafter.replay_active {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::PlaceStone));
            } else if crafter.running && !crafter.replay_active {
                let _ = cmd_tx.send(CrafterCommand::Reset);
            }
            true
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            if crafter.replay_active {
                let _ = cmd_tx.send(CrafterCommand::StopReplay);
            }
            true
        }
        KeyCode::Esc => {
            if crafter.input_capture {
                crafter.input_capture = false;
            } else if crafter.replay_active {
                let _ = cmd_tx.send(CrafterCommand::StopReplay);
            }
            true
        }
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::MoveUp));
            crafter.last_action = Some(Action::MoveUp);
            true
        }
        KeyCode::Down if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::MoveDown));
            crafter.last_action = Some(Action::MoveDown);
            true
        }
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::MoveLeft));
            crafter.last_action = Some(Action::MoveLeft);
            true
        }
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::MoveRight));
            crafter.last_action = Some(Action::MoveRight);
            true
        }
        KeyCode::Char(' ') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::Do));
            crafter.last_action = Some(Action::Do);
            true
        }
        KeyCode::Tab | KeyCode::Char('z') | KeyCode::Char('Z') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::Sleep));
            crafter.last_action = Some(Action::Sleep);
            true
        }
        KeyCode::Char('1') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::MakeWoodPickaxe));
            true
        }
        KeyCode::Char('2') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::MakeStonePickaxe));
            true
        }
            KeyCode::Char('3') if crafter.input_capture => {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::MakeIronPickaxe));
                true
            }
            KeyCode::Char('4') if crafter.input_capture => {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::MakeWoodSword));
                true
            }
            KeyCode::Char('5') if crafter.input_capture => {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::MakeStoneSword));
                true
            }
            KeyCode::Char('6') if crafter.input_capture => {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::MakeIronSword));
                true
            }
            KeyCode::Char('7') if crafter.input_capture => {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::MakeDiamondPickaxe));
                true
            }
            KeyCode::Char('8') if crafter.input_capture => {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::MakeDiamondSword));
                true
            }
            KeyCode::Char('9') if crafter.input_capture => {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::MakeDiamondArmor));
                true
            }
        KeyCode::Char('g') | KeyCode::Char('G') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::ShootArrow));
            true
        }
        KeyCode::Char('q') | KeyCode::Char('Q') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::DrinkPotionRed));
            true
        }
        KeyCode::Char('e') | KeyCode::Char('E') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::DrinkPotionGreen));
            true
        }
        KeyCode::Char('y') | KeyCode::Char('Y') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::DrinkPotionBlue));
            true
        }
        KeyCode::Char('u') | KeyCode::Char('U') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::DrinkPotionPink));
            true
        }
        KeyCode::Char('i') | KeyCode::Char('I') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::DrinkPotionCyan));
            true
        }
        KeyCode::Char('o') | KeyCode::Char('O') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::DrinkPotionYellow));
            true
        }
        KeyCode::Char('f') | KeyCode::Char('F') if crafter.input_capture => {
            let _ = cmd_tx.send(CrafterCommand::Action(Action::PlaceFurnace));
            true
        }
        _ => false,
    };

    CrafterKeyOutcome {
        handled,
        graphics_mode_update,
    }
}

pub fn drain_updates(crafter: &mut CrafterState, rx: &Receiver<CrafterUpdate>) {
    while let Ok(update) = rx.try_recv() {
        match update {
            CrafterUpdate::Tick { actual_hz } => {
                crafter.actual_hz = actual_hz;
            }
            CrafterUpdate::Status { message } => {
                crafter.status = message;
            }
            CrafterUpdate::Running { running } => {
                crafter.running = running;
                if running {
                    crafter.achievements.clear();
                } else {
                    crafter.input_capture = false;
                }
            }
            CrafterUpdate::Paused { paused } => {
                crafter.paused = paused;
            }
            CrafterUpdate::InputCapture { capture } => {
                crafter.input_capture = capture;
            }
            CrafterUpdate::Frame {
                lines,
                rgba_data,
                rgba_width,
                rgba_height,
                score,
                health,
                food,
                thirst,
                energy,
                tick,
                achievements,
                visible_mobs,
                density_lines,
                has_adjacent_table,
                has_adjacent_furnace,
                inventory,
            } => {
                crafter.frame_lines = lines;
                crafter.frame_rgba = rgba_data;
                crafter.frame_width = rgba_width;
                crafter.frame_height = rgba_height;
                if rgba_width > 0 {
                    crafter.last_tile_size = rgba_width / 7;
                }
                crafter.score = score;
                crafter.health = health;
                crafter.food = food;
                crafter.thirst = thirst;
                crafter.energy = energy;
                crafter.tick = tick;
                crafter.visible_mobs = visible_mobs;
                crafter.density_lines = density_lines;
                crafter.has_adjacent_table = has_adjacent_table;
                crafter.has_adjacent_furnace = has_adjacent_furnace;
                for ach in achievements {
                    if !crafter.achievements.contains(&ach) {
                        crafter.achievements.push(ach);
                    }
                }
                crafter.inventory = inventory;
            }
            CrafterUpdate::Event { message } => {
                crafter.events.push(message);
                if crafter.events.len() > 10 {
                    crafter.events.remove(0);
                }
            }
            CrafterUpdate::RecordingSaved { path: _ } => {}
            CrafterUpdate::RecordingsList { recordings } => {
                crafter.recordings = recordings;
                crafter.selected_recording = 0;
            }
            CrafterUpdate::ReplayMode {
                active,
                current_step,
                total_steps,
            } => {
                crafter.replay_active = active;
                crafter.replay_step = current_step;
                crafter.replay_total = total_steps;
            }
        }
    }
}

pub fn draw_list(
    buffer: *mut ot::OptimizedBuffer,
    crafter: &CrafterState,
    width: u32,
    height: u32,
    highlight_bg: [f32; 4],
) {
    let fg = [0.78, 0.81, 0.86, 1.0];
    let dim_fg = [0.5, 0.52, 0.55, 1.0];
    let green = [0.2, 0.8, 0.2, 1.0];
    let cyan = [0.2, 0.8, 0.9, 1.0];

    let filtered_recordings = if crafter.show_recordings {
        filtered_recording_indices(&crafter.recordings, &crafter.recordings_search)
    } else {
        Vec::new()
    };

    let header = if crafter.show_recordings {
        "Recordings"
    } else {
        HEADER
    };
    unsafe {
        ot::bufferDrawText(
            buffer,
            header.as_bytes().as_ptr(),
            header.len(),
            2,
            2,
            fg.as_ptr(),
            std::ptr::null(),
            0,
        );
    }

    let status = if crafter.show_recordings {
        format!(
            "{} / {} recordings",
            filtered_recordings.len(),
            crafter.recordings.len()
        )
    } else if crafter.replay_active {
        format!(
            "REPLAY: {}/{}  {}",
            crafter.replay_step,
            crafter.replay_total,
            if crafter.paused { "[PAUSED]" } else { "" }
        )
    } else {
        format!(
            "HP: {}  Food: {}  Drink: {}  Energy: {}  {}",
            crafter.health,
            crafter.food,
            crafter.thirst,
            crafter.energy,
            if crafter.paused {
                "[PAUSED]"
            } else if crafter.input_capture {
                "[PLAYING]"
            } else {
                ""
            }
        )
    };
    unsafe {
        ot::bufferDrawText(
            buffer,
            status.as_bytes().as_ptr(),
            status.len(),
            2,
            3,
            dim_fg.as_ptr(),
            std::ptr::null(),
            0,
        );
    }

    let y_start = 5u32;
    let max_y = height.saturating_sub(4);

    if crafter.show_rule_editor {
        let menu_header = "=== RULE CONFIG ===";
        unsafe {
            ot::bufferDrawText(
                buffer,
                menu_header.as_bytes().as_ptr(),
                menu_header.len(),
                2,
                y_start,
                fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }

        let mut y = y_start + 1;
        if let Some(path) = crafter.rule_editor_path.as_ref() {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("rule config");
            let header = format!("Editing: {}", name);
            if y < max_y {
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        header.as_bytes().as_ptr(),
                        header.len(),
                        2,
                        y,
                        dim_fg.as_ptr(),
                        std::ptr::null(),
                        0,
                    );
                }
                y = y.saturating_add(1);
            }
        }

        let config = match crafter.rule_editor_config.as_ref() {
            Some(config) => config,
            None => return,
        };
        y = y.saturating_add(1);
        let total = RULE_EDITOR_FIELDS.len();
        let max_rows = max_y.saturating_sub(y) as usize;
        if max_rows > 0 {
            let selected = crafter.rule_editor_index.min(total.saturating_sub(1));
            let start = if selected >= max_rows {
                selected + 1 - max_rows
            } else {
                0
            };
            for (idx, field) in RULE_EDITOR_FIELDS
                .iter()
                .enumerate()
                .skip(start)
                .take(max_rows)
            {
                let row_y = y.saturating_add((idx - start) as u32);
                if row_y >= max_y {
                    break;
                }
                let is_selected = idx == selected;
                let value = rule_editor_value_label(config, *field);
                let line = format!("{:<20} {}", field.label, value);
                if is_selected {
                    let fill_width = width.saturating_sub(2);
                    unsafe {
                        ot::bufferFillRect(buffer, 1, row_y, fill_width, 1, highlight_bg.as_ptr())
                    };
                }
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        line.as_bytes().as_ptr(),
                        line.len(),
                        2,
                        row_y,
                        if is_selected { fg.as_ptr() } else { dim_fg.as_ptr() },
                        std::ptr::null(),
                        0,
                    );
                }
            }
        }
        return;
    }

    if crafter.show_config_menu {
        let menu_header = "=== SETTINGS ===";
        unsafe {
            ot::bufferDrawText(
                buffer,
                menu_header.as_bytes().as_ptr(),
                menu_header.len(),
                2,
                y_start,
                fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }
        let mut y = y_start + 2;
        for (i, label) in CONFIG_ITEMS.iter().enumerate() {
            if y >= max_y {
                break;
            }
            let is_selected = i == crafter.config_selection;
            let value = match i {
                0 => format!(
                    "{}: {}",
                    label,
                    crafter
                        .profile_names
                        .get(crafter.profile_index)
                        .map(|name| name.as_str())
                        .unwrap_or("default")
                ),
                1 => format!(
                    "{}: {}",
                    label,
                    selected_rule_config_display_name(crafter)
                ),
                2 => format!(
                    "{}: {}",
                    label,
                    if crafter.config.logical_time {
                        "Logical (AI)"
                    } else {
                        "Real-time"
                    }
                ),
                3 => format!("{}: {} Hz", label, crafter.config.tick_rate),
                4 => format!("{}: {}", label, crafter.config.world_width),
                5 => format!("{}: {}", label, crafter.config.world_height),
                6 => format!(
                    "{}: {}",
                    label,
                    if crafter.config.random_seed {
                        "Random"
                    } else {
                        "Fixed"
                    }
                ),
                7 => format!("{}: {}", label, crafter.config.seed),
                8 => format!(
                    "{}: {}",
                    label,
                    if crafter.config.graphics_mode {
                        "Pixel"
                    } else {
                        "ASCII"
                    }
                ),
                _ => label.to_string(),
            };
            if is_selected {
                let fill_width = width.saturating_sub(2);
                unsafe {
                    ot::bufferFillRect(buffer, 1, y, fill_width, 1, highlight_bg.as_ptr())
                };
            }
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    value.as_bytes().as_ptr(),
                    value.len(),
                    2,
                    y,
                    if is_selected { fg.as_ptr() } else { dim_fg.as_ptr() },
                    std::ptr::null(),
                    0,
                );
            }
            y = y.saturating_add(1);
        }
        return;
    }

    if crafter.show_craft_menu {
        let menu_header = "=== CRAFT ===";
        unsafe {
            ot::bufferDrawText(
                buffer,
                menu_header.as_bytes().as_ptr(),
                menu_header.len(),
                2,
                y_start,
                fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }
        let mut y = y_start + 1;
        let craft_indices = craft_menu_indices(crafter);
        if craft_indices.is_empty() {
            let line = "Nothing craftable (need resources)";
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    line.as_bytes().as_ptr(),
                    line.len(),
                    2,
                    y,
                    dim_fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            return;
        }
        for (row, &idx) in craft_indices.iter().enumerate() {
            if y >= max_y {
                break;
            }
            let (name, _, req) = CRAFT_ITEMS[idx];
            let is_selected = row == crafter.craft_selection;
            let line = format!("{} - {}", name, req);
            if is_selected {
                let fill_width = width.saturating_sub(2);
                unsafe { ot::bufferFillRect(buffer, 1, y, fill_width, 1, highlight_bg.as_ptr()) };
            }
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    line.as_bytes().as_ptr(),
                    line.len(),
                    2,
                    y,
                    if is_selected { fg.as_ptr() } else { dim_fg.as_ptr() },
                    std::ptr::null(),
                    0,
                );
            }
            y = y.saturating_add(1);
        }
        return;
    }

    if crafter.show_place_menu {
        let menu_header = "=== BUILD ===";
        unsafe {
            ot::bufferDrawText(
                buffer,
                menu_header.as_bytes().as_ptr(),
                menu_header.len(),
                2,
                y_start,
                fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }
        let mut y = y_start + 1;
        for (i, (name, _, desc)) in PLACE_ITEMS.iter().enumerate() {
            if y >= max_y {
                break;
            }
            let is_selected = i == crafter.place_selection;
            let line = format!("{} - {}", name, desc);
            if is_selected {
                let fill_width = width.saturating_sub(2);
                unsafe { ot::bufferFillRect(buffer, 1, y, fill_width, 1, highlight_bg.as_ptr()) };
            }
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    line.as_bytes().as_ptr(),
                    line.len(),
                    2,
                    y,
                    if is_selected { fg.as_ptr() } else { dim_fg.as_ptr() },
                    std::ptr::null(),
                    0,
                );
            }
            y = y.saturating_add(1);
        }
        return;
    }

    if crafter.show_recordings {
        let search_label = "Search (/):";
        let search_value = if crafter.recordings_search_active {
            format!("{}_", crafter.recordings_search)
        } else {
            crafter.recordings_search.clone()
        };
        let search_line = format!("{} {}", search_label, search_value);
        unsafe {
            ot::bufferDrawText(
                buffer,
                search_line.as_bytes().as_ptr(),
                search_line.len(),
                2,
                y_start,
                dim_fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }

        let list_start = y_start.saturating_add(2);

        if crafter.recordings.is_empty() {
            let msg = "No recordings found. Play a game first!";
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    msg.as_bytes().as_ptr(),
                    msg.len(),
                    2,
                    list_start,
                    dim_fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            let hint = "[C] New game  [Esc] Back";
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    hint.as_bytes().as_ptr(),
                    hint.len(),
                    2,
                    list_start + 2,
                    dim_fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
        } else if filtered_recordings.is_empty() {
            let msg = "No recordings match the search.";
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    msg.as_bytes().as_ptr(),
                    msg.len(),
                    2,
                    list_start,
                    dim_fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
        } else {
            let mut y = list_start;
            for (i, &idx) in filtered_recordings.iter().enumerate() {
                if y >= max_y {
                    break;
                }
                let rec = &crafter.recordings[idx];
                let is_selected = i == crafter.selected_recording;
                let total_possible = Achievements::all_names().len() as u32;
                let line = format!(
                    "{} {} steps, {:.1} reward  ach:{}/{}",
                    rec.name,
                    rec.total_steps,
                    rec.total_reward,
                    rec.unique_achievements,
                    total_possible
                );
                let display_line = if line.len() > width.saturating_sub(4) as usize {
                    &line[..width.saturating_sub(4) as usize]
                } else {
                    line.as_str()
                };
                if is_selected {
                    let fill_width = width.saturating_sub(2);
                    unsafe { ot::bufferFillRect(buffer, 1, y, fill_width, 1, highlight_bg.as_ptr()) };
                }
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        display_line.as_bytes().as_ptr(),
                        display_line.len(),
                        2,
                        y,
                        if is_selected { fg.as_ptr() } else { dim_fg.as_ptr() },
                        std::ptr::null(),
                        0,
                    );
                }
                y = y.saturating_add(1);
            }
        }
        return;
    }

    if !crafter.running {
        let msg = "Press [C] to start, [L] for recordings";
        unsafe {
            ot::bufferDrawText(
                buffer,
                msg.as_bytes().as_ptr(),
                msg.len(),
                2,
                y_start,
                dim_fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }
        return;
    }

    if let Some(ref rgba_data) = crafter.frame_rgba {
        if crafter.frame_width > 0 && crafter.frame_height > 0 && !rgba_data.is_empty() {
            let dest_x = 2u32;
            let dest_y = y_start;
            let bytes_per_row = crafter.frame_width * 4;

            let cells_w = crafter.frame_width / 2;
            let cells_h = crafter.frame_height / 2;

            unsafe {
                let frame_buffer = ot::createOptimizedBuffer(cells_w, cells_h, false);
                if !frame_buffer.is_null() {
                    ot::bufferDrawSuperSampleBuffer(
                        frame_buffer,
                        0,
                        0,
                        rgba_data.as_ptr(),
                        rgba_data.len(),
                        1,
                        bytes_per_row,
                    );

                    ot::drawFrameBuffer(
                        buffer,
                        dest_x as i32,
                        dest_y as i32,
                        frame_buffer,
                        0,
                        0,
                        cells_w,
                        cells_h,
                    );

                    ot::destroyOptimizedBuffer(frame_buffer);
                }
            }
        }
    } else {
        let mut y = y_start;
        let frame_color = if crafter.replay_active { cyan } else { green };
        for line in &crafter.frame_lines {
            if y >= max_y {
                break;
            }
            let display_line = if line.len() > width.saturating_sub(4) as usize {
                &line[..width.saturating_sub(4) as usize]
            } else {
                line.as_str()
            };
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    display_line.as_bytes().as_ptr(),
                    display_line.len(),
                    2,
                    y,
                    frame_color.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            y = y.saturating_add(1);
        }
    }
}

pub fn draw_detail(
    buffer: *mut ot::OptimizedBuffer,
    crafter: &CrafterState,
    list_width: u32,
    width: u32,
    height: u32,
    show_detail: bool,
) {
    if !show_detail {
        return;
    }
    let fg = [0.78, 0.81, 0.86, 1.0];
    let dim_fg = [0.5, 0.52, 0.55, 1.0];
    let green = [0.2, 0.8, 0.2, 1.0];
    let x = list_width.saturating_add(4);
    let max_y = height.saturating_sub(4);
    let mut y = 5;

    let header = "Game Info";
    unsafe {
        ot::bufferDrawText(
            buffer,
            header.as_bytes().as_ptr(),
            header.len(),
            x,
            y,
            fg.as_ptr(),
            std::ptr::null(),
            0,
        );
    }
    y = y.saturating_add(2);

    let info_lines = [
        format!("Status: {}", crafter.status),
        format!(
            "Health: {}  Food: {}  Drink: {}  Energy: {}",
            crafter.health, crafter.food, crafter.thirst, crafter.energy
        ),
        format!("Tick: {}", crafter.tick),
    ];

    for line in info_lines {
        if y >= max_y {
            break;
        }
        unsafe {
            ot::bufferDrawText(
                buffer,
                line.as_bytes().as_ptr(),
                line.len(),
                x,
                y,
                dim_fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }
        y = y.saturating_add(1);
    }

    let visible = &crafter.visible_mobs;
    if !visible.is_empty() {
        y = y.saturating_add(1);
        if y < max_y {
            let header = "Visible Mobs";
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    header.as_bytes().as_ptr(),
                    header.len(),
                    x,
                    y,
                    fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            y = y.saturating_add(1);
            let max_row_height = visible
                .iter()
                .map(|preview| {
                    let text_lines = if preview.detail.is_some() { 2 } else { 1 };
                    (preview.height / 2).max(text_lines)
                })
                .max()
                .unwrap_or(1);
            let max_rows = ((max_y.saturating_sub(y)) / max_row_height)
                .max(1)
                .min(6) as usize;
            for (idx, preview) in visible.iter().take(max_rows).enumerate() {
                if y >= max_y {
                    break;
                }
                let mut label_x = x;
                let mut row_height = if preview.detail.is_some() { 2 } else { 1 };
                if let Some(ref rgba) = preview.rgba {
                    if preview.width > 0 && preview.height > 0 {
                        let cells_w = preview.width / 2;
                        let cells_h = preview.height / 2;
                        let bytes_per_row = preview.width * 4;
                        row_height = row_height.max(cells_h.max(1));
                        unsafe {
                            let icon_buffer = ot::createOptimizedBuffer(cells_w, cells_h, false);
                            if !icon_buffer.is_null() {
                                ot::bufferDrawSuperSampleBuffer(
                                    icon_buffer,
                                    0,
                                    0,
                                    rgba.as_ptr(),
                                    rgba.len(),
                                    1,
                                    bytes_per_row,
                                );
                                ot::drawFrameBuffer(
                                    buffer,
                                    x as i32,
                                    y as i32,
                                    icon_buffer,
                                    0,
                                    0,
                                    cells_w,
                                    cells_h,
                                );
                                ot::destroyOptimizedBuffer(icon_buffer);
                            }
                        }
                        label_x = x.saturating_add(cells_w.saturating_add(1));
                    }
                }
                let label_y = y;
                if label_y >= max_y {
                    break;
                }
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        preview.label.as_bytes().as_ptr(),
                        preview.label.len(),
                        label_x,
                        label_y,
                        dim_fg.as_ptr(),
                        std::ptr::null(),
                        0,
                    );
                }
                if let Some(ref detail) = preview.detail {
                    let detail_y = y.saturating_add(1);
                    if detail_y < max_y {
                        unsafe {
                            ot::bufferDrawText(
                                buffer,
                                detail.as_bytes().as_ptr(),
                                detail.len(),
                                label_x,
                                detail_y,
                                dim_fg.as_ptr(),
                                std::ptr::null(),
                                0,
                            );
                        }
                    }
                }
                y = y.saturating_add(row_height);
                if idx + 1 == max_rows && visible.len() > max_rows && y < max_y {
                    let remaining = visible.len() - max_rows;
                    let more_line = format!("... +{} more", remaining);
                    unsafe {
                        ot::bufferDrawText(
                            buffer,
                            more_line.as_bytes().as_ptr(),
                            more_line.len(),
                            x,
                            y,
                            dim_fg.as_ptr(),
                            std::ptr::null(),
                            0,
                        );
                    }
                    y = y.saturating_add(1);
                }
            }
        }
    }

    y = y.saturating_add(1);
    let inv = &crafter.inventory;
    let inv_lines = [
        format!("Wood: {}  Stone: {}  Coal: {}", inv.wood, inv.stone, inv.coal),
        format!(
            "Iron: {}  Diamond: {}  Sapling: {}",
            inv.iron, inv.diamond, inv.sapling
        ),
        format!("Sapphire: {}  Ruby: {}", inv.sapphire, inv.ruby),
        format!(
            "Pickaxe: W{} S{} I{} D{}",
            inv.wood_pickaxe, inv.stone_pickaxe, inv.iron_pickaxe, inv.diamond_pickaxe
        ),
        format!(
            "Sword: W{} S{} I{} D{}",
            inv.wood_sword, inv.stone_sword, inv.iron_sword, inv.diamond_sword
        ),
        format!("Bow: {}  Arrows: {}", inv.bow, inv.arrows),
        format!(
            "Armor: H{} C{} L{} B{}",
            inv.armor_helmet,
            inv.armor_chestplate,
            inv.armor_leggings,
            inv.armor_boots
        ),
        format!(
            "Potions: R{} G{} B{} P{} C{} Y{}",
            inv.potion_red,
            inv.potion_green,
            inv.potion_blue,
            inv.potion_pink,
            inv.potion_cyan,
            inv.potion_yellow
        ),
        format!("XP: {}  Level: {}  SP: {}", inv.xp, inv.level, inv.stat_points),
    ];

    if y < max_y {
        if !crafter.density_lines.is_empty() {
            let detail_width = width.saturating_sub(x.saturating_add(2));
            let col_width = (detail_width / 2).max(20);
            let right_x = x.saturating_add(col_width.saturating_add(2));
            let start_y = y;

            let mut left_y = start_y;
            let header = "Map Density";
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    header.as_bytes().as_ptr(),
                    header.len(),
                    x,
                    left_y,
                    fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            left_y = left_y.saturating_add(1);
            for line in crafter.density_lines.iter() {
                if left_y >= max_y {
                    break;
                }
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        line.as_bytes().as_ptr(),
                        line.len(),
                        x,
                        left_y,
                        dim_fg.as_ptr(),
                        std::ptr::null(),
                        0,
                    );
                }
                left_y = left_y.saturating_add(1);
            }

            let mut right_y = start_y;
            let inv_header = "Inventory";
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    inv_header.as_bytes().as_ptr(),
                    inv_header.len(),
                    right_x,
                    right_y,
                    fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            right_y = right_y.saturating_add(1);
            for line in inv_lines {
                if right_y >= max_y {
                    break;
                }
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        line.as_bytes().as_ptr(),
                        line.len(),
                        right_x,
                        right_y,
                        dim_fg.as_ptr(),
                        std::ptr::null(),
                        0,
                    );
                }
                right_y = right_y.saturating_add(1);
            }

            let used = left_y.max(right_y);
            y = used.saturating_add(1);
        } else {
            let inv_header = "Inventory";
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    inv_header.as_bytes().as_ptr(),
                    inv_header.len(),
                    x,
                    y,
                    fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            y = y.saturating_add(1);
            for line in inv_lines {
                if y >= max_y {
                    break;
                }
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        line.as_bytes().as_ptr(),
                        line.len(),
                        x,
                        y,
                        dim_fg.as_ptr(),
                        std::ptr::null(),
                        0,
                    );
                }
                y = y.saturating_add(1);
            }
            y = y.saturating_add(1);
        }
    }

    let craftax_enabled = load_session_config(&crafter.config)
        .map(|config| config.craftax.enabled && config.craftax.achievements_enabled)
        .unwrap_or(false);
    let all_achievements = if craftax_enabled {
        Achievements::all_names_with_craftax()
    } else {
        Achievements::all_names().to_vec()
    };
    let achievements_header =
        format!("Achievements ({}/{})", crafter.achievements.len(), all_achievements.len());
    let achievements_max_len = all_achievements
        .iter()
        .map(|ach| ach.len())
        .max()
        .unwrap_or(0)
        .max(achievements_header.len()) as u32;
    let achievements_col_width = achievements_max_len.saturating_add(2).max(12);
    let achievements_available_width = width.saturating_sub(x + 2);
    let achievements_columns = (achievements_available_width / achievements_col_width)
        .max(1)
        .min(3) as usize;
    let achievements_rows =
        (all_achievements.len() + achievements_columns - 1) / achievements_columns;
    let achievements_block_height = 1 + achievements_rows as u32;
    let events_block_height = if crafter.events.is_empty() { 0 } else { 1 + 1 + 3 };

    y = y.saturating_add(1);
    let reserved_after_legend = 1 + achievements_block_height + events_block_height;
    let legend_limit_y = max_y.saturating_sub(reserved_after_legend);
    if y < legend_limit_y {
        let legend_y_start = y;
        let legend_header = "Map Legend";
        unsafe {
            ot::bufferDrawText(
                buffer,
                legend_header.as_bytes().as_ptr(),
                legend_header.len(),
                x,
                y,
                fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }
        y = y.saturating_add(1);

        let legend_items = [
            "@ = Player",
            "T = Tree",
            ". = Grass",
            "~ = Water/Stone",
            ": = Coal",
            "I = Iron ore",
            "D = Diamond",
            "Z = Zombie",
            "S = Skeleton",
            "C = Cow (food)",
            "s = Sapphire",
            "r = Ruby",
            "H = Chest",
            "O = Orc",
            "M = Orc Mage",
            "K = Knight",
            "A = Archer",
            "t = Troll",
            "B = Bat",
            "N = Snail",
        ];

        let legend_max_len = legend_items
            .iter()
            .map(|item| item.len())
            .max()
            .unwrap_or(0)
            .max(legend_header.len()) as u32;

        let legend_col_width = legend_max_len.saturating_add(2).max(12);
        let legend_available_width = width.saturating_sub(x + 2);
        let legend_columns = (legend_available_width / legend_col_width).max(1).min(2) as usize;
        let legend_rows = (legend_items.len() + legend_columns - 1) / legend_columns;
        let legend_rows = legend_rows.min((legend_limit_y.saturating_sub(y)) as usize);

        for row in 0..legend_rows {
            let row_y = y + row as u32;
            if row_y >= legend_limit_y {
                break;
            }
            for col in 0..legend_columns {
                let idx = col * legend_rows + row;
                if idx >= legend_items.len() {
                    break;
                }
                let legend = legend_items[idx];
                let col_x = x + (col as u32 * legend_col_width);
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        legend.as_bytes().as_ptr(),
                        legend.len(),
                        col_x,
                        row_y,
                        dim_fg.as_ptr(),
                        std::ptr::null(),
                        0,
                    );
                }
            }
        }

        let legend_bottom = legend_y_start.saturating_add(legend_rows as u32);

        let action_x = x.saturating_add(legend_col_width * legend_columns as u32 + 4);
        let action_header = "Action Legend";
        let action_items = [
            "WASD = Move",
            "Space = Interact",
            "Tab = Sleep",
            "T = Place table",
            "R = Place stone",
            "F = Place furnace",
            "P = Place plant",
            "C = Craft menu (table)",
            "1 = Wood pickaxe",
            "2 = Stone pickaxe",
            "3 = Iron pickaxe",
            "4 = Wood sword",
            "5 = Stone sword",
            "6 = Iron sword",
            "7 = Diamond pickaxe",
            "8 = Diamond sword",
            "9 = Diamond armor",
            "G = Shoot arrow",
            "Q/E/Y/U/I/O = Drink potions",
        ];
        if action_x < width.saturating_sub(4) {
            let mut action_y = legend_y_start;
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    action_header.as_bytes().as_ptr(),
                    action_header.len(),
                    action_x,
                    action_y,
                    fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            action_y = action_y.saturating_add(1);
            for action in action_items {
                if action_y >= legend_bottom {
                    break;
                }
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        action.as_bytes().as_ptr(),
                        action.len(),
                        action_x,
                        action_y,
                        dim_fg.as_ptr(),
                        std::ptr::null(),
                        0,
                    );
                }
                action_y = action_y.saturating_add(1);
            }
        }
        y = legend_bottom;
    }

    y = y.saturating_add(1);
    if y < max_y {
        unsafe {
            ot::bufferDrawText(
                buffer,
                achievements_header.as_bytes().as_ptr(),
                achievements_header.len(),
                x,
                y,
                fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }
        y = y.saturating_add(1);

        let unlocked: HashSet<&str> = crafter.achievements.iter().map(|ach| ach.as_str()).collect();
        let rows = achievements_rows;

        for row in 0..rows {
            let row_y = y + row as u32;
            if row_y >= max_y {
                break;
            }
            for col in 0..achievements_columns {
                let idx = col * rows + row;
                if idx >= all_achievements.len() {
                    break;
                }
                let ach = all_achievements[idx];
                let col_x = x + (col as u32 * achievements_col_width);
                let color = if unlocked.contains(ach) {
                    green.as_ptr()
                } else {
                    dim_fg.as_ptr()
                };
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        ach.as_bytes().as_ptr(),
                        ach.len(),
                        col_x,
                        row_y,
                        color,
                        std::ptr::null(),
                        0,
                    );
                }
            }
        }
        y = y.saturating_add(rows as u32);
    }

    y = y.saturating_add(1);
    if y < max_y && !crafter.events.is_empty() {
        let header = "Recent Events";
        unsafe {
            ot::bufferDrawText(
                buffer,
                header.as_bytes().as_ptr(),
                header.len(),
                x,
                y,
                fg.as_ptr(),
                std::ptr::null(),
                0,
            );
        }
        y = y.saturating_add(1);

        for event in crafter.events.iter().rev().take(3) {
            if y >= max_y {
                break;
            }
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    event.as_bytes().as_ptr(),
                    event.len(),
                    x,
                    y,
                    dim_fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            y = y.saturating_add(1);
        }
    }
}

pub fn action_hint(crafter: &CrafterState) -> String {
    if crafter.show_rule_editor {
        "[Up/Down] Select  [Left/Right] Adjust  [Enter] Toggle  [S] Save  [Esc] Back"
            .to_string()
    } else if crafter.show_config_menu {
        "[Up/Down] Select  [Left/Right] Adjust  [N] New  [Y] New YAML  [E] Edit  [D] Delete  [Enter] Start  [Esc] Back"
            .to_string()
    } else if crafter.show_craft_menu || crafter.show_place_menu {
        "[Up/Down] Select  [Enter/Space] Confirm  [Esc] Cancel".to_string()
    } else if crafter.show_recordings {
        "[Up/Down] Select  [Enter] Replay  [/] Search  [C] New game  [Esc] Back".to_string()
    } else if crafter.replay_active {
        "[P] Pause  [B] Branch  [X/Esc] Stop replay  [C] New game".to_string()
    } else if crafter.running && crafter.paused {
        "[P] Resume  [Backspace] Delete session  [R] Reset  [L] Recordings".to_string()
    } else if crafter.input_capture {
        "[WASD] Move  [Space] Interact  [Tab] Sleep  [T/R/F/P] Place  [C] Craft menu  [1-9] Quick craft  [G] Shoot  [Q/E/Y/U/I/O] Potions  [Esc] Release"
            .to_string()
    } else if crafter.running {
        "[C] Capture input  [P] Pause  [R] Reset  [L] Recordings".to_string()
    } else {
        "[S] Settings  [C] Start  [L] Recordings".to_string()
    }
}
