use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use crafter_core::image_renderer::{ImageRenderer, ImageRendererConfig};
use crafter_core::recording::{Recording, RecordingOptions, RecordingSession, ReplaySession};
use crafter_core::{Achievements, SaveData};
use crafter_core::renderer::{Renderer, TextRenderer};
use crafter_core::{Action, SessionConfig};
use opentui_sys as ot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    pub last_action: Option<Action>,
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
            world_width: 48,
            world_height: 20,
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
            last_action: None,
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
}

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
                if path.extension().map(|e| e == "toml").unwrap_or(false) {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        configs.entry(stem.to_string()).or_insert(RuleConfigEntry {
                            name: stem.to_string(),
                            path: Some(path),
                            editable: false,
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
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    configs.insert(
                        stem.to_string(),
                        RuleConfigEntry {
                            name: stem.to_string(),
                            path: Some(path),
                            editable: true,
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
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = format!("custom_{}", timestamp);
    let mut target = rule_configs_dir();
    let _ = std::fs::create_dir_all(&target);
    target.push(format!("{}.toml", name));

    let source = state
        .rule_configs
        .get(state.rule_config_index)
        .and_then(|entry| entry.path.clone());

    if let Some(source_path) = source {
        let _ = std::fs::copy(source_path, &target);
    } else {
        let contents = format!("base = \"{}\"\n", default_rule_config_name());
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

    let user_path = rule_configs_dir().join(format!("{}.toml", name));
    if user_path.exists() {
        return SessionConfig::load_from_path(user_path).ok();
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
        let mut frame_width = 48u32;
        let mut frame_height = 24u32;
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
                if crafter.craft_selection + 1 < CRAFT_ITEMS.len() {
                    crafter.craft_selection += 1;
                }
                true
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some((_, action, _)) = CRAFT_ITEMS.get(crafter.craft_selection) {
                    let _ = cmd_tx.send(CrafterCommand::Action(*action));
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
                    if edit_selected_rule_config(crafter) {
                        refresh_rule_configs(crafter);
                        crafter.status = "Edited rule config".to_string();
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
            } else {
                crafter.input_capture = !crafter.input_capture;
            }
            true
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            if crafter.input_capture && crafter.running && !crafter.replay_active {
                let _ = cmd_tx.send(CrafterCommand::Action(Action::PlacePlant));
            } else if crafter.running {
                let _ = cmd_tx.send(CrafterCommand::SetPaused(!crafter.paused));
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
            "HP: {}  Food: {}  Thirst: {}  {}",
            crafter.health,
            crafter.food,
            crafter.thirst,
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
                    crafter
                        .rule_configs
                        .get(crafter.rule_config_index)
                        .map(|config| config.name.as_str())
                        .unwrap_or("classic")
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
        for (i, (name, _, req)) in CRAFT_ITEMS.iter().enumerate() {
            if y >= max_y {
                break;
            }
            let is_selected = i == crafter.craft_selection;
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
            "Health: {}  Food: {}  Thirst: {}  Energy: {}",
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

    y = y.saturating_add(1);
    if y < max_y {
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

        let inv = &crafter.inventory;
        let inv_lines = [
            format!("Wood: {}  Stone: {}  Coal: {}", inv.wood, inv.stone, inv.coal),
            format!(
                "Iron: {}  Diamond: {}  Sapling: {}",
                inv.iron, inv.diamond, inv.sapling
            ),
            format!(
                "Sapphire: {}  Ruby: {}",
                inv.sapphire, inv.ruby
            ),
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
    }

    y = y.saturating_add(1);
    if y < max_y {
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
            "T = Tree / Troll",
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
            "T = Troll",
            "B = Bat",
            "N = Snail",
        ];

        let legend_max_len = legend_items
            .iter()
            .map(|item| item.len())
            .max()
            .unwrap_or(0)
            .max(legend_header.len()) as u32;

        for legend in legend_items {
            if y >= max_y {
                break;
            }
            unsafe {
                ot::bufferDrawText(
                    buffer,
                    legend.as_bytes().as_ptr(),
                    legend.len(),
                    x,
                    y,
                    dim_fg.as_ptr(),
                    std::ptr::null(),
                    0,
                );
            }
            y = y.saturating_add(1);
        }

        let action_x = x.saturating_add(legend_max_len + 4);
        let action_header = "Action Legend";
        let action_items = [
            "WASD = Move",
            "Space = Interact",
            "Tab = Sleep",
            "T = Place table",
            "R = Place stone",
            "F = Place furnace",
            "P = Place plant",
            "T (menu) = Craft",
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
                if action_y >= max_y {
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
    }

    y = y.saturating_add(1);
    if y < max_y && !crafter.achievements.is_empty() {
        let header = "Achievements";
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

        let max_len = crafter
            .achievements
            .iter()
            .map(|ach| ach.len())
            .max()
            .unwrap_or(0)
            .max(header.len()) as u32;
        let col_width = max_len.saturating_add(2).max(12);
        let available_width = width.saturating_sub(x + 2);
        let columns = (available_width / col_width).max(1) as usize;
        let rows = (crafter.achievements.len() + columns - 1) / columns;
        let rows = rows.min((max_y.saturating_sub(y)) as usize);

        for row in 0..rows {
            let row_y = y + row as u32;
            if row_y >= max_y {
                break;
            }
            for col in 0..columns {
                let idx = col * rows + row;
                if idx >= crafter.achievements.len() {
                    break;
                }
                let ach = &crafter.achievements[idx];
                let col_x = x + (col as u32 * col_width);
                unsafe {
                    ot::bufferDrawText(
                        buffer,
                        ach.as_bytes().as_ptr(),
                        ach.len(),
                        col_x,
                        row_y,
                        dim_fg.as_ptr(),
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

        for event in crafter.events.iter().rev().take(5) {
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
    if crafter.show_config_menu {
        "[Up/Down] Select  [Left/Right] Adjust  [Enter] Start  [Esc] Back".to_string()
    } else if crafter.show_craft_menu || crafter.show_place_menu {
        "[Up/Down] Select  [Enter/Space] Confirm  [Esc] Cancel".to_string()
    } else if crafter.show_recordings {
        "[Up/Down] Select  [Enter] Replay  [/] Search  [C] New game  [Esc] Back".to_string()
    } else if crafter.replay_active {
        "[P] Pause  [B] Branch  [X/Esc] Stop replay  [C] New game".to_string()
    } else if crafter.input_capture {
        "[WASD] Move  [Space] Interact  [Tab] Sleep  [T/R/F/P] Place  [1-6] Craft  [G] Shoot  [Q/E/Y/U/I/O] Potions  [Esc] Release"
            .to_string()
    } else if crafter.running {
        "[C] Capture input  [P] Pause  [R] Reset  [L] Recordings".to_string()
    } else {
        "[S] Settings  [C] Start  [L] Recordings".to_string()
    }
}
