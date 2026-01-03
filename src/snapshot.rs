//! Snapshot API for agent-based interaction with Crafter
//!
//! This module provides a structured request/response pattern for interacting
//! with Crafter games, similar to the message/email snapshot pattern used
//! in the mission-control system.

use crate::action::Action;
use crate::entity::GameObject;
use crate::material::Material;
use crate::session::{DoneReason, Session, StepResult};
use crate::SessionConfig;
use std::collections::HashMap;
use uuid::Uuid;

/// Snapshot request (mirrors mc_api::CrafterSnapshotRequest)
#[derive(Debug, Clone)]
pub struct SnapshotRequest {
    pub session_id: Option<String>,
    pub seed: Option<u64>,
    pub actions: Vec<SnapshotAction>,
    pub view_size: Option<u32>,
}

/// Action enum (mirrors mc_api::CrafterAction)
#[derive(Debug, Clone, Copy)]
pub enum SnapshotAction {
    Noop,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Do,
    Sleep,
    PlaceStone,
    PlaceTable,
    PlaceFurnace,
    PlacePlant,
    MakeWoodPickaxe,
    MakeStonePickaxe,
    MakeIronPickaxe,
    MakeWoodSword,
    MakeStoneSword,
    MakeIronSword,
}

impl SnapshotAction {
    pub fn to_action(self) -> Action {
        match self {
            Self::Noop => Action::Noop,
            Self::MoveLeft => Action::MoveLeft,
            Self::MoveRight => Action::MoveRight,
            Self::MoveUp => Action::MoveUp,
            Self::MoveDown => Action::MoveDown,
            Self::Do => Action::Do,
            Self::Sleep => Action::Sleep,
            Self::PlaceStone => Action::PlaceStone,
            Self::PlaceTable => Action::PlaceTable,
            Self::PlaceFurnace => Action::PlaceFurnace,
            Self::PlacePlant => Action::PlacePlant,
            Self::MakeWoodPickaxe => Action::MakeWoodPickaxe,
            Self::MakeStonePickaxe => Action::MakeStonePickaxe,
            Self::MakeIronPickaxe => Action::MakeIronPickaxe,
            Self::MakeWoodSword => Action::MakeWoodSword,
            Self::MakeStoneSword => Action::MakeStoneSword,
            Self::MakeIronSword => Action::MakeIronSword,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "noop" => Some(Self::Noop),
            "l" | "left" | "move_left" => Some(Self::MoveLeft),
            "r" | "right" | "move_right" => Some(Self::MoveRight),
            "u" | "up" | "move_up" => Some(Self::MoveUp),
            "d" | "down" | "move_down" => Some(Self::MoveDown),
            "do" | "interact" => Some(Self::Do),
            "sleep" => Some(Self::Sleep),
            "table" | "place_table" => Some(Self::PlaceTable),
            "furnace" | "place_furnace" => Some(Self::PlaceFurnace),
            "stone" | "place_stone" => Some(Self::PlaceStone),
            "plant" | "place_plant" => Some(Self::PlacePlant),
            "pick" | "wood_pick" | "make_wood_pickaxe" => Some(Self::MakeWoodPickaxe),
            "spick" | "stone_pick" | "make_stone_pickaxe" => Some(Self::MakeStonePickaxe),
            "ipick" | "iron_pick" | "make_iron_pickaxe" => Some(Self::MakeIronPickaxe),
            "sword" | "wood_sword" | "make_wood_sword" => Some(Self::MakeWoodSword),
            "ssword" | "stone_sword" | "make_stone_sword" => Some(Self::MakeStoneSword),
            "isword" | "iron_sword" | "make_iron_sword" => Some(Self::MakeIronSword),
            _ => None,
        }
    }
}

/// Entity visible in the game world
#[derive(Debug, Clone)]
pub struct SnapshotEntity {
    pub kind: String,
    pub pos: (i32, i32),
    pub health: Option<i32>,
}

/// Player stats
#[derive(Debug, Clone)]
pub struct SnapshotStats {
    pub health: i32,
    pub food: i32,
    pub drink: i32,
    pub energy: i32,
}

/// Player inventory
#[derive(Debug, Clone)]
pub struct SnapshotInventory {
    pub wood: i32,
    pub stone: i32,
    pub coal: i32,
    pub iron: i32,
    pub diamond: i32,
    pub sapling: i32,
    pub wood_pickaxe: i32,
    pub stone_pickaxe: i32,
    pub iron_pickaxe: i32,
    pub wood_sword: i32,
    pub stone_sword: i32,
    pub iron_sword: i32,
}

/// Label/value pair for details
#[derive(Debug, Clone)]
pub struct SnapshotLine {
    pub label: String,
    pub value: String,
}

/// Snapshot response
#[derive(Debug, Clone)]
pub struct SnapshotResponse {
    pub session_id: String,
    pub step: u64,
    pub done: bool,
    pub done_reason: Option<String>,
    pub player_pos: (i32, i32),
    pub player_facing: (i8, i8),
    pub stats: SnapshotStats,
    pub inventory: SnapshotInventory,
    pub map_lines: Vec<String>,
    pub map_legend: Vec<SnapshotLine>,
    pub entities: Vec<SnapshotEntity>,
    pub achievements: Vec<String>,
    pub newly_unlocked: Vec<String>,
    pub reward: f32,
    pub available_actions: Vec<String>,
    pub hints: Vec<String>,
}

/// Manager for Crafter game sessions
pub struct SnapshotManager {
    sessions: HashMap<String, Session>,
    default_config: SessionConfig,
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            default_config: SessionConfig {
                world_size: (64, 64),
                view_radius: 4, // 4 = 9x9 grid
                ..Default::default()
            },
        }
    }

    /// Process a snapshot request and return a response
    pub fn process(&mut self, request: SnapshotRequest) -> SnapshotResponse {
        // Convert view_size to view_radius (view_size = 2*radius + 1)
        let view_radius = request.view_size.map(|s| (s - 1) / 2).unwrap_or(4);

        // Get or create session
        let (session_id, session) = if let Some(id) = request.session_id {
            if let Some(s) = self.sessions.get_mut(&id) {
                (id, s)
            } else {
                // Session not found, create new
                let new_id = Uuid::new_v4().to_string();
                let config = SessionConfig {
                    seed: request.seed,
                    view_radius,
                    ..self.default_config.clone()
                };
                self.sessions.insert(new_id.clone(), Session::new(config));
                (new_id.clone(), self.sessions.get_mut(&new_id).unwrap())
            }
        } else {
            // Create new session
            let new_id = Uuid::new_v4().to_string();
            let config = SessionConfig {
                seed: request.seed,
                view_radius,
                ..self.default_config.clone()
            };
            self.sessions.insert(new_id.clone(), Session::new(config));
            (new_id.clone(), self.sessions.get_mut(&new_id).unwrap())
        };

        // Execute actions
        let mut last_result: Option<StepResult> = None;
        let mut all_newly_unlocked = Vec::new();
        let mut total_reward = 0.0;

        for action in request.actions {
            let result = session.step(action.to_action());
            total_reward += result.reward;
            all_newly_unlocked.extend(result.newly_unlocked.clone());
            let done = result.done;
            last_result = Some(result);
            if done {
                break;
            }
        }

        // Drop the mutable borrow
        drop(session);
        
        // Get an immutable borrow for building response
        let session = self.sessions.get(&session_id).unwrap();

        // Build response from current state
        self.build_response(session_id, session, last_result, all_newly_unlocked, total_reward)
    }

    fn build_response(
        &self,
        session_id: String,
        session: &Session,
        last_result: Option<StepResult>,
        newly_unlocked: Vec<String>,
        reward: f32,
    ) -> SnapshotResponse {
        let state = session.get_state();
        let inv = &state.inventory;

        // Build map lines
        let view_radius = session.config.view_radius as i32;
        let half = view_radius;
        let mut map_lines = Vec::new();

        for dy in -half..=half {
            let mut row = String::new();
            for dx in -half..=half {
                let pos = (state.player_pos.0 + dx, state.player_pos.1 + dy);
                if dx == 0 && dy == 0 {
                    row.push('@');
                    continue;
                }

                // Check for mobs
                let mob = session.world.objects.values().any(|o| match o {
                    GameObject::Cow(c) => c.pos == pos,
                    GameObject::Zombie(z) => z.pos == pos,
                    GameObject::Skeleton(s) => s.pos == pos,
                    _ => false,
                });

                if mob {
                    row.push('M');
                } else {
                    let ch = match session.world.get_material(pos) {
                        Some(Material::Grass) => '.',
                        Some(Material::Water) => '~',
                        Some(Material::Stone) => '#',
                        Some(Material::Tree) => 'T',
                        Some(Material::Coal) => 'c',
                        Some(Material::Iron) => 'i',
                        Some(Material::Diamond) => 'D',
                        Some(Material::Table) => '+',
                        Some(Material::Furnace) => 'F',
                        Some(Material::Sand) => ':',
                        Some(Material::Lava) => 'L',
                        Some(Material::Path) => '=',
                        None => ' ',
                    };
                    row.push(ch);
                }
            }
            map_lines.push(row);
        }

        // Build entities list
        let view_size = view_radius * 2 + 1;
        let mut entities = Vec::new();
        for obj in session.world.objects.values() {
            match obj {
                GameObject::Cow(c) => {
                    let dist = ((c.pos.0 - state.player_pos.0).abs()
                        + (c.pos.1 - state.player_pos.1).abs()) as i32;
                    if dist <= view_size {
                        entities.push(SnapshotEntity {
                            kind: "cow".to_string(),
                            pos: c.pos,
                            health: Some(c.health as i32),
                        });
                    }
                }
                GameObject::Zombie(z) => {
                    let dist = ((z.pos.0 - state.player_pos.0).abs()
                        + (z.pos.1 - state.player_pos.1).abs()) as i32;
                    if dist <= view_size {
                        entities.push(SnapshotEntity {
                            kind: "zombie".to_string(),
                            pos: z.pos,
                            health: Some(z.health as i32),
                        });
                    }
                }
                GameObject::Skeleton(s) => {
                    let dist = ((s.pos.0 - state.player_pos.0).abs()
                        + (s.pos.1 - state.player_pos.1).abs()) as i32;
                    if dist <= view_size {
                        entities.push(SnapshotEntity {
                            kind: "skeleton".to_string(),
                            pos: s.pos,
                            health: Some(s.health as i32),
                        });
                    }
                }
                _ => {}
            }
        }

        // Get done status
        let (done, done_reason) = if let Some(ref result) = last_result {
            (
                result.done,
                result.done_reason.as_ref().map(|r| match r {
                    DoneReason::Death => "death".to_string(),
                    DoneReason::MaxSteps => "max_steps".to_string(),
                    DoneReason::Reset => "reset".to_string(),
                }),
            )
        } else {
            (false, None)
        };

        // Build achievements list
        let ach = &state.achievements;
        let mut achievements = Vec::new();
        if ach.collect_coal > 0 { achievements.push("collect_coal".to_string()); }
        if ach.collect_diamond > 0 { achievements.push("collect_diamond".to_string()); }
        if ach.collect_drink > 0 { achievements.push("collect_drink".to_string()); }
        if ach.collect_iron > 0 { achievements.push("collect_iron".to_string()); }
        if ach.collect_sapling > 0 { achievements.push("collect_sapling".to_string()); }
        if ach.collect_stone > 0 { achievements.push("collect_stone".to_string()); }
        if ach.collect_wood > 0 { achievements.push("collect_wood".to_string()); }
        if ach.defeat_skeleton > 0 { achievements.push("defeat_skeleton".to_string()); }
        if ach.defeat_zombie > 0 { achievements.push("defeat_zombie".to_string()); }
        if ach.eat_cow > 0 { achievements.push("eat_cow".to_string()); }
        if ach.eat_plant > 0 { achievements.push("eat_plant".to_string()); }
        if ach.make_iron_pickaxe > 0 { achievements.push("make_iron_pickaxe".to_string()); }
        if ach.make_iron_sword > 0 { achievements.push("make_iron_sword".to_string()); }
        if ach.make_stone_pickaxe > 0 { achievements.push("make_stone_pickaxe".to_string()); }
        if ach.make_stone_sword > 0 { achievements.push("make_stone_sword".to_string()); }
        if ach.make_wood_pickaxe > 0 { achievements.push("make_wood_pickaxe".to_string()); }
        if ach.make_wood_sword > 0 { achievements.push("make_wood_sword".to_string()); }
        if ach.place_furnace > 0 { achievements.push("place_furnace".to_string()); }
        if ach.place_plant > 0 { achievements.push("place_plant".to_string()); }
        if ach.place_stone > 0 { achievements.push("place_stone".to_string()); }
        if ach.place_table > 0 { achievements.push("place_table".to_string()); }
        if ach.wake_up > 0 { achievements.push("wake_up".to_string()); }

        // Available actions
        let available_actions = vec![
            "move_left".to_string(),
            "move_right".to_string(),
            "move_up".to_string(),
            "move_down".to_string(),
            "do".to_string(),
            "sleep".to_string(),
            "place_table".to_string(),
            "place_stone".to_string(),
            "place_furnace".to_string(),
            "place_plant".to_string(),
            "make_wood_pickaxe".to_string(),
            "make_stone_pickaxe".to_string(),
            "make_iron_pickaxe".to_string(),
            "make_wood_sword".to_string(),
            "make_stone_sword".to_string(),
            "make_iron_sword".to_string(),
        ];

        // Build hints based on current state
        let mut hints = Vec::new();
        if inv.wood < 2 && inv.wood_pickaxe == 0 {
            hints.push("Collect wood by facing a tree and using 'do'".to_string());
        }
        if inv.wood >= 2 && ach.place_table == 0 {
            hints.push("Place a crafting table with 'place_table' (needs 2 wood)".to_string());
        }
        if inv.wood >= 1 && inv.wood_pickaxe == 0 && ach.place_table > 0 {
            hints.push("Make a wood pickaxe with 'make_wood_pickaxe' while near table".to_string());
        }
        if inv.wood >= 1 && inv.wood_sword == 0 && ach.place_table > 0 {
            hints.push("Make a wood sword with 'make_wood_sword' while near table".to_string());
        }
        if inv.health < 5 && inv.food > 2 {
            hints.push("Use 'sleep' to restore health (consumes food)".to_string());
        }

        let map_legend = vec![
            SnapshotLine { label: "@".to_string(), value: "Player".to_string() },
            SnapshotLine { label: ".".to_string(), value: "Grass".to_string() },
            SnapshotLine { label: "T".to_string(), value: "Tree".to_string() },
            SnapshotLine { label: "#".to_string(), value: "Stone".to_string() },
            SnapshotLine { label: "~".to_string(), value: "Water".to_string() },
            SnapshotLine { label: ":".to_string(), value: "Sand".to_string() },
            SnapshotLine { label: "c".to_string(), value: "Coal".to_string() },
            SnapshotLine { label: "i".to_string(), value: "Iron".to_string() },
            SnapshotLine { label: "D".to_string(), value: "Diamond".to_string() },
            SnapshotLine { label: "+".to_string(), value: "Table".to_string() },
            SnapshotLine { label: "F".to_string(), value: "Furnace".to_string() },
            SnapshotLine { label: "M".to_string(), value: "Mob".to_string() },
        ];

        SnapshotResponse {
            session_id,
            step: state.step,
            done,
            done_reason,
            player_pos: state.player_pos,
            player_facing: state.player_facing,
            stats: SnapshotStats {
                health: inv.health as i32,
                food: inv.food as i32,
                drink: inv.drink as i32,
                energy: inv.energy as i32,
            },
            inventory: SnapshotInventory {
                wood: inv.wood as i32,
                stone: inv.stone as i32,
                coal: inv.coal as i32,
                iron: inv.iron as i32,
                diamond: inv.diamond as i32,
                sapling: inv.sapling as i32,
                wood_pickaxe: inv.wood_pickaxe as i32,
                stone_pickaxe: inv.stone_pickaxe as i32,
                iron_pickaxe: inv.iron_pickaxe as i32,
                wood_sword: inv.wood_sword as i32,
                stone_sword: inv.stone_sword as i32,
                iron_sword: inv.iron_sword as i32,
            },
            map_lines,
            map_legend,
            entities,
            achievements,
            newly_unlocked,
            reward,
            available_actions,
            hints,
        }
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    /// Remove a session
    pub fn remove_session(&mut self, id: &str) -> Option<Session> {
        self.sessions.remove(id)
    }

    /// List all session IDs
    pub fn session_ids(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let mut manager = SnapshotManager::new();
        let request = SnapshotRequest {
            session_id: None,
            seed: Some(42),
            actions: vec![],
            view_size: None,
        };

        let response = manager.process(request);
        assert!(!response.session_id.is_empty());
        assert_eq!(response.step, 0);
        assert!(!response.done);
        assert_eq!(response.stats.health, 9);
    }

    #[test]
    fn test_execute_actions() {
        let mut manager = SnapshotManager::new();

        // Start new game
        let request = SnapshotRequest {
            session_id: None,
            seed: Some(777),
            actions: vec![
                SnapshotAction::MoveRight,
                SnapshotAction::MoveRight,
                SnapshotAction::MoveRight,
                SnapshotAction::MoveRight,
                SnapshotAction::Do, // Chop tree
            ],
            view_size: None,
        };

        let response = manager.process(request);
        assert_eq!(response.step, 5);
        assert!(response.inventory.wood >= 1 || response.newly_unlocked.contains(&"collect_wood".to_string()));
    }

    #[test]
    fn test_resume_session() {
        let mut manager = SnapshotManager::new();

        // Start new game
        let request1 = SnapshotRequest {
            session_id: None,
            seed: Some(42),
            actions: vec![SnapshotAction::MoveRight],
            view_size: None,
        };
        let response1 = manager.process(request1);
        let session_id = response1.session_id.clone();

        // Resume with more actions
        let request2 = SnapshotRequest {
            session_id: Some(session_id.clone()),
            seed: None,
            actions: vec![SnapshotAction::MoveRight, SnapshotAction::MoveRight],
            view_size: None,
        };
        let response2 = manager.process(request2);

        assert_eq!(response2.session_id, session_id);
        assert_eq!(response2.step, 3); // 1 + 2 more
    }
}
