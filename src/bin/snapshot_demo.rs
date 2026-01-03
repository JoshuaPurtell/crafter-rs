//! Demo of the snapshot/request pattern for Crafter local play
//!
//! This demonstrates how an agent can interact with Crafter using
//! a structured request/response pattern similar to email/messages.

use crafter_core::{SnapshotAction, SnapshotManager, SnapshotRequest};

fn main() {
    let mut manager = SnapshotManager::new();

    println!("=== Crafter Snapshot API Demo ===\n");

    // Start a new game with seed 777
    println!("1. Starting new game with seed 777...\n");
    let response = manager.process(SnapshotRequest {
        session_id: None,
        seed: Some(777),
        actions: vec![],
        view_size: None,
        config_name: None,
        config_path: None,
        config_toml: None,
    });

    print_snapshot(&response);

    // Execute some actions: move right 4 times and chop tree
    println!("\n2. Moving right 4 times and chopping tree...\n");
    let response = manager.process(SnapshotRequest {
        session_id: Some(response.session_id.clone()),
        seed: None,
        actions: vec![
            SnapshotAction::MoveRight,
            SnapshotAction::MoveRight,
            SnapshotAction::MoveRight,
            SnapshotAction::MoveRight,
            SnapshotAction::Do,
        ],
        view_size: None,
        config_name: None,
        config_path: None,
        config_toml: None,
    });

    print_snapshot(&response);

    // Continue gathering wood
    println!("\n3. Gathering more wood...\n");
    let response = manager.process(SnapshotRequest {
        session_id: Some(response.session_id.clone()),
        seed: None,
        actions: vec![
            SnapshotAction::MoveDown,
            SnapshotAction::MoveDown,
            SnapshotAction::MoveDown,
            SnapshotAction::MoveDown,
            SnapshotAction::MoveDown,
            SnapshotAction::MoveDown,
            SnapshotAction::MoveDown,
            SnapshotAction::MoveDown,
            SnapshotAction::MoveLeft,
            SnapshotAction::MoveLeft,
            SnapshotAction::MoveLeft,
            SnapshotAction::MoveLeft,
            SnapshotAction::Do,
            SnapshotAction::MoveLeft,
            SnapshotAction::Do,
        ],
        view_size: None,
        config_name: None,
        config_path: None,
        config_toml: None,
    });

    print_snapshot(&response);

    // Place table and make sword
    println!("\n4. Placing table and making wood sword...\n");
    let response = manager.process(SnapshotRequest {
        session_id: Some(response.session_id.clone()),
        seed: None,
        actions: vec![
            SnapshotAction::PlaceTable,
            SnapshotAction::MakeWoodSword,
            SnapshotAction::MakeWoodPickaxe,
        ],
        view_size: None,
        config_name: None,
        config_path: None,
        config_toml: None,
    });

    print_snapshot(&response);

    println!("\n=== Demo Complete ===");
    println!("\nThis pattern allows agents to:");
    println!("  - Start sessions with optional seeds");
    println!("  - Execute batches of actions");
    println!("  - Resume sessions by ID");
    println!("  - Get structured state snapshots");
}

fn print_snapshot(response: &crafter_core::SnapshotResponse) {
    println!("Session: {}", response.session_id);
    println!("Step: {}", response.step);
    println!("Done: {}", response.done);

    println!("\nStats:");
    println!(
        "  HP:{} Food:{} Drink:{} Energy:{}",
        response.stats.health, response.stats.food, response.stats.drink, response.stats.energy
    );

    println!("\nInventory:");
    println!(
        "  Wood:{} Stone:{} Coal:{} Iron:{}",
        response.inventory.wood,
        response.inventory.stone,
        response.inventory.coal,
        response.inventory.iron
    );
    println!(
        "  WPick:{} SPick:{} | WSword:{} SSword:{}",
        response.inventory.wood_pickaxe,
        response.inventory.stone_pickaxe,
        response.inventory.wood_sword,
        response.inventory.stone_sword
    );

    println!("\nMap:");
    for line in &response.map_lines {
        println!("  {}", line);
    }

    if !response.achievements.is_empty() {
        println!("\nAchievements: {:?}", response.achievements);
    }

    if !response.newly_unlocked.is_empty() {
        println!("\nNewly Unlocked: {:?}", response.newly_unlocked);
    }

    if response.reward > 0.0 {
        println!("\nReward: {}", response.reward);
    }

    if !response.hints.is_empty() {
        println!("\nHints:");
        for hint in &response.hints {
            println!("  - {}", hint);
        }
    }
}
