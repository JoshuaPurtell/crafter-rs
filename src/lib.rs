//! Crafter Core - A Rust implementation of the Crafter game engine
//!
//! This crate provides the core game logic for a Minecraft-like survival game,
//! including world generation, entity management, crafting, and game simulation.
//!
//! ## Features
//!
//! - `png` - Enable PNG image rendering (requires the `image` crate)
//!
//! ## Modules
//!
//! - [`session`] - Game session management
//! - [`recording`] - Recording and replay for training data
//! - [`saveload`] - Save/load game state to disk
//! - [`rewards`] - Configurable reward functions
//! - [`image_renderer`] - PNG image rendering (requires `png` feature)
//! - [`renderer`] - Text and JSON renderers

pub mod action;
pub mod achievement;
pub mod config;
pub mod craftax;
pub mod entity;
pub mod image_renderer;
pub mod inventory;
pub mod material;
mod parity; // Parity tests against Python Crafter
pub mod recording;
pub mod renderer;
pub mod rewards;
pub mod saveload;
pub mod session;
pub mod snapshot;
pub mod world;
pub mod worldgen;

// Core types
pub use action::Action;
pub use achievement::Achievements;
pub use config::SessionConfig;
pub use entity::{Arrow, Cow, GameObject, Mob, Plant, Player, Position, Skeleton, Zombie};
pub use inventory::Inventory;
pub use material::Material;
pub use session::{GameState, Session, StepResult, TimeMode};
pub use world::World;

// Recording and replay
pub use recording::{Recording, RecordingOptions, RecordingSession, ReplaySession};

// Save/load
pub use saveload::{SaveData, SessionSaveLoad};

// Rewards
pub use rewards::{RewardCalculator, RewardConfig, RewardResult};

// Image rendering
pub use image_renderer::{ColorPalette, ImageRenderer, ImageRendererConfig};

// Snapshot API
pub use snapshot::{
    SnapshotAction, SnapshotEntity, SnapshotInventory, SnapshotLine, SnapshotManager,
    SnapshotRequest, SnapshotResponse, SnapshotStats,
};
