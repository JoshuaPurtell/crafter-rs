//! Demo of the snapshot/request pattern for Crafter local play
//!
//! This demonstrates how an agent can interact with Crafter using
//! a structured request/response pattern similar to email/messages.

use crafter_core::{SnapshotAction, SnapshotManager, SnapshotRequest, Session, SessionConfig};
use crafter_core::entity::GameObject;
use crafter_core::material::Material;

fn main() {
    if let Ok(config_name) = std::env::var("CRAFTER_CONFIG") {
        let mode = std::env::var("CRAFTER_MODE").unwrap_or_else(|_| "probe".to_string());
        if mode == "achievements" {
            run_headless_achievements(&config_name);
        } else if mode == "interactive" {
            run_interactive(&config_name);
        } else {
            run_headless_probe(&config_name);
        }
        return;
    }

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

fn run_interactive(config_name: &str) {
    let mut manager = SnapshotManager::new();
    let seed = std::env::var("CRAFTER_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    let (config_name, config_path) = snapshot_config_source(config_name);

    println!("=== Crafter Snapshot Interactive ===");
    println!("Config: {}", config_name.as_deref().unwrap_or("default"));
    println!("Type 'help' for commands, 'quit' to exit.\n");

    let mut response = manager.process(SnapshotRequest {
        session_id: None,
        seed,
        actions: vec![],
        view_size: None,
        config_name,
        config_path,
        config_toml: None,
    });
    print_snapshot(&response);

    let mut input = String::new();
    loop {
        input.clear();
        print!("\naction> ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        if std::io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let line = input.trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("quit") || line.eq_ignore_ascii_case("exit") {
            break;
        }
        if line.eq_ignore_ascii_case("help") {
            print_interactive_help();
            continue;
        }

        let mut actions = Vec::new();
        for token in line.split_whitespace() {
            let expanded = expand_actions_from_token(token);
            if expanded.is_empty() {
                println!("Unknown token: {}", token);
            } else {
                actions.extend(expanded);
            }
        }

        if actions.is_empty() {
            continue;
        }

        response = manager.process(SnapshotRequest {
            session_id: Some(response.session_id.clone()),
            seed: None,
            actions,
            view_size: None,
            config_name: None,
            config_path: None,
            config_toml: None,
        });

        print_snapshot(&response);
        if response.done {
            println!("\nGame over.");
            break;
        }
    }
}

fn snapshot_config_source(config_name: &str) -> (Option<String>, Option<String>) {
    if std::path::Path::new(config_name).exists() {
        (None, Some(config_name.to_string()))
    } else {
        (Some(config_name.to_string()), None)
    }
}

fn expand_actions_from_token(token: &str) -> Vec<SnapshotAction> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut split = trimmed.len();
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() {
            split = idx;
            break;
        }
    }
    let (prefix, suffix) = trimmed.split_at(split);
    let count = if suffix.is_empty() {
        1
    } else {
        suffix.parse::<usize>().unwrap_or(1)
    };
    if let Some(action) = parse_action_token(prefix) {
        std::iter::repeat(action).take(count).collect()
    } else {
        Vec::new()
    }
}

fn parse_action_token(token: &str) -> Option<SnapshotAction> {
    match token.to_ascii_lowercase().as_str() {
        "w" | "up" | "north" => Some(SnapshotAction::MoveUp),
        "s" | "down" | "south" => Some(SnapshotAction::MoveDown),
        "a" | "left" | "west" => Some(SnapshotAction::MoveLeft),
        "d" | "right" | "east" => Some(SnapshotAction::MoveRight),
        "e" | "do" | "use" => Some(SnapshotAction::Do),
        "sleep" => Some(SnapshotAction::Sleep),
        "table" | "place_table" => Some(SnapshotAction::PlaceTable),
        "furnace" | "place_furnace" => Some(SnapshotAction::PlaceFurnace),
        "plant" | "place_plant" => Some(SnapshotAction::PlacePlant),
        "stone" | "place_stone" => Some(SnapshotAction::PlaceStone),
        "wood_sword" => Some(SnapshotAction::MakeWoodSword),
        "wood_pickaxe" => Some(SnapshotAction::MakeWoodPickaxe),
        "stone_sword" => Some(SnapshotAction::MakeStoneSword),
        "stone_pickaxe" => Some(SnapshotAction::MakeStonePickaxe),
        "iron_sword" => Some(SnapshotAction::MakeIronSword),
        "iron_pickaxe" => Some(SnapshotAction::MakeIronPickaxe),
        "diamond_sword" => Some(SnapshotAction::MakeDiamondSword),
        "diamond_pickaxe" => Some(SnapshotAction::MakeDiamondPickaxe),
        "bow" | "make_bow" => Some(SnapshotAction::MakeBow),
        "arrow" | "make_arrow" => Some(SnapshotAction::MakeArrow),
        "iron_armor" => Some(SnapshotAction::MakeIronArmor),
        "diamond_armor" => Some(SnapshotAction::MakeDiamondArmor),
        "shoot" | "shoot_arrow" => Some(SnapshotAction::ShootArrow),
        "potion_red" | "drink_red" => Some(SnapshotAction::DrinkPotionRed),
        "potion_green" | "drink_green" => Some(SnapshotAction::DrinkPotionGreen),
        "potion_blue" | "drink_blue" => Some(SnapshotAction::DrinkPotionBlue),
        "potion_pink" | "drink_pink" => Some(SnapshotAction::DrinkPotionPink),
        "potion_cyan" | "drink_cyan" => Some(SnapshotAction::DrinkPotionCyan),
        "potion_yellow" | "drink_yellow" => Some(SnapshotAction::DrinkPotionYellow),
        "noop" => Some(SnapshotAction::Noop),
        _ => None,
    }
}

fn print_interactive_help() {
    println!("Movement: w a s d (or up/down/left/right)");
    println!("Interact: e (do), sleep");
    println!("Place: table, furnace, stone, plant");
    println!("Craft: wood_sword, wood_pickaxe, stone_sword, stone_pickaxe, iron_sword, iron_pickaxe");
    println!("Craftax: diamond_sword, diamond_pickaxe, bow, arrow, iron_armor, diamond_armor");
    println!("Combat: shoot");
    println!("Potions: drink_red, drink_green, drink_blue, drink_pink, drink_cyan, drink_yellow");
    println!("Repeat with suffix: w5, right3, do2");
}

fn run_headless_probe(config_name: &str) {
    let config = if let Ok(loaded) = SessionConfig::load_named(config_name) {
        loaded
    } else if std::path::Path::new(config_name).exists() {
        SessionConfig::load_from_path(config_name).unwrap_or_default()
    } else {
        SessionConfig::default()
    };

    let mut session = Session::new(config);
    let mut counts = std::collections::HashMap::<&'static str, u32>::new();

    for y in 0..session.world.area.1 {
        for x in 0..session.world.area.0 {
            let pos = (x as i32, y as i32);
            if let Some(mat) = session.world.get_material(pos) {
                let key = match mat {
                    Material::Sapphire => "sapphire",
                    Material::Ruby => "ruby",
                    Material::Chest => "chest",
                    _ => "other",
                };
                *counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    let mut mob_counts = std::collections::HashMap::<&'static str, u32>::new();
    for obj in session.world.objects.values() {
        if let GameObject::CraftaxMob(mob) = obj {
            let key = match mob.kind {
                crafter_core::entity::CraftaxMobKind::OrcSoldier => "orc_soldier",
                crafter_core::entity::CraftaxMobKind::OrcMage => "orc_mage",
                crafter_core::entity::CraftaxMobKind::Knight => "knight",
                crafter_core::entity::CraftaxMobKind::KnightArcher => "knight_archer",
                crafter_core::entity::CraftaxMobKind::Troll => "troll",
                crafter_core::entity::CraftaxMobKind::Bat => "bat",
                crafter_core::entity::CraftaxMobKind::Snail => "snail",
            };
            *mob_counts.entry(key).or_insert(0) += 1;
        }
    }

    println!("=== Headless Craftax Probe ===");
    println!("Config: {}", config_name);
    println!(
        "Materials: sapphire={} ruby={} chest={}",
        counts.get("sapphire").unwrap_or(&0),
        counts.get("ruby").unwrap_or(&0),
        counts.get("chest").unwrap_or(&0)
    );
    println!(
        "Mobs: orc_soldier={} orc_mage={} knight={} knight_archer={} troll={} bat={} snail={}",
        mob_counts.get("orc_soldier").unwrap_or(&0),
        mob_counts.get("orc_mage").unwrap_or(&0),
        mob_counts.get("knight").unwrap_or(&0),
        mob_counts.get("knight_archer").unwrap_or(&0),
        mob_counts.get("troll").unwrap_or(&0),
        mob_counts.get("bat").unwrap_or(&0),
        mob_counts.get("snail").unwrap_or(&0)
    );

    let _ = session.step(SnapshotAction::MoveRight.to_action());
    let _ = session.step(SnapshotAction::MoveRight.to_action());
    let _ = session.step(SnapshotAction::Do.to_action());
    let state = session.get_state();

    println!(
        "Inventory: sapphire={} ruby={} arrows={} potions_red={} bow={} diamond_pickaxe={}",
        state.inventory.sapphire,
        state.inventory.ruby,
        state.inventory.arrows,
        state.inventory.potion_red,
        state.inventory.bow,
        state.inventory.diamond_pickaxe
    );
}

fn run_headless_achievements(config_name: &str) {
    let mut config = if let Ok(loaded) = SessionConfig::load_named(config_name) {
        loaded
    } else if std::path::Path::new(config_name).exists() {
        SessionConfig::load_from_path(config_name).unwrap_or_default()
    } else {
        SessionConfig::default()
    };

    config.max_steps = config.max_steps.or(Some(10000));
    if config.seed.is_none() {
        config.seed = Some(2024);
    }
    match std::env::var("CRAFTER_SAFE").ok().as_deref() {
        Some("1") => {
            config.zombie_spawn_rate = 0.0;
            config.zombie_density = 0.0;
            config.skeleton_density = 0.0;
            config.craftax.mobs_enabled = false;
            config.craftax.combat_enabled = false;
            config.hunger_enabled = false;
            config.thirst_enabled = false;
            config.fatigue_enabled = false;
        }
        Some("2") => {
            config.hunger_enabled = false;
            config.thirst_enabled = false;
            config.fatigue_enabled = false;
        }
        _ => {}
    }
    let mut session = Session::new(config);

    let mut plan: std::collections::VecDeque<crafter_core::Action> = std::collections::VecDeque::new();
    let mut last_action = crafter_core::Action::Noop;

    for _ in 0..session.config.max_steps.unwrap_or(10000) {
        let state = session.get_state();
        if state.inventory.health == 0 {
            break;
        }

        let inv = &state.inventory;
        let has_ranged = inv.bow > 0 && inv.arrows > 0;
        let should_flee = inv.health <= 4 || (inv.best_sword_tier() < 2 && !has_ranged);
        if should_flee {
            if let Some(action) = flee_action(&session, 6) {
                let _ = session.step(action);
                continue;
            }
        }

        if let Some(craft_action) = craft_action_if_available(&session) {
            let _ = session.step(craft_action);
            continue;
        }

        if !plan.is_empty() && state.inventory.best_sword_tier() == 0 {
            if hostile_nearby(&session, 4) {
                plan.clear();
            }
        }

        if plan.is_empty() {
            if let Some(new_plan) = plan_actions_if_needed(&session) {
                plan = new_plan;
            }
        }

        if let Some(action) = plan.pop_front() {
            last_action = action;
            let _ = session.step(action);
            continue;
        }

        let action = choose_action(&session, last_action);
        last_action = action;
        let _ = session.step(action);
        if session.get_state().step % 1000 == 0 {
            let inv = &session.get_state().inventory;
            println!(
                "Step {}: HP:{} Food:{} Drink:{} Energy:{} XP:{} Level:{}",
                session.get_state().step,
                inv.health,
                inv.food,
                inv.drink,
                inv.energy,
                inv.xp,
                inv.level
            );
        }
    }

    let state = session.get_state();
    let ach = &state.achievements;
    let mut unlocked = Vec::new();
    for name in crafter_core::Achievements::all_names_with_craftax() {
        if let Some(count) = ach.get(name) {
            if count > 0 {
                unlocked.push((name.to_string(), count));
            }
        }
    }

    println!("\n=== Headless Achievement Run ===");
    println!("Config: {}", config_name);
    println!("Step: {}", state.step);
    println!(
        "Vitals: HP:{} Food:{} Drink:{} Energy:{}",
        state.inventory.health, state.inventory.food, state.inventory.drink, state.inventory.energy
    );
    println!(
        "Resources: Wood:{} Stone:{} Coal:{} Iron:{} Diamond:{} Sapphire:{} Ruby:{}",
        state.inventory.wood,
        state.inventory.stone,
        state.inventory.coal,
        state.inventory.iron,
        state.inventory.diamond,
        state.inventory.sapphire,
        state.inventory.ruby
    );
    println!(
        "Tools: WP:{} SP:{} IP:{} DP:{} WS:{} SS:{} IS:{} DS:{} Bow:{} Arrows:{}",
        state.inventory.wood_pickaxe,
        state.inventory.stone_pickaxe,
        state.inventory.iron_pickaxe,
        state.inventory.diamond_pickaxe,
        state.inventory.wood_sword,
        state.inventory.stone_sword,
        state.inventory.iron_sword,
        state.inventory.diamond_sword,
        state.inventory.bow,
        state.inventory.arrows
    );
    println!(
        "Armor: H{} C{} L{} B{}",
        state.inventory.armor_helmet,
        state.inventory.armor_chestplate,
        state.inventory.armor_leggings,
        state.inventory.armor_boots
    );
    println!(
        "Potions: R{} G{} B{} P{} C{} Y{}",
        state.inventory.potion_red,
        state.inventory.potion_green,
        state.inventory.potion_blue,
        state.inventory.potion_pink,
        state.inventory.potion_cyan,
        state.inventory.potion_yellow
    );
    println!("XP:{} Level:{} StatPoints:{}", state.inventory.xp, state.inventory.level, state.inventory.stat_points);
    println!("\nAchievements unlocked: {}", unlocked.len());
    for (name, count) in unlocked {
        println!("  - {} ({})", name, count);
    }
}

fn plan_actions_if_needed(session: &Session) -> Option<std::collections::VecDeque<crafter_core::Action>> {
    let state = session.get_state();
    let pos = state.player_pos;
    let facing = state.player_facing;
    let inv = &state.inventory;
    let world = &session.world;

    if inv.drink <= 6 {
        let mut water_targets = std::collections::HashSet::new();
        for y in 0..world.area.1 {
            for x in 0..world.area.0 {
                let pos = (x as i32, y as i32);
                if world.get_material(pos) == Some(Material::Water) {
                    water_targets.insert(pos);
                }
            }
        }
        if let Some(path) = find_path_to_face_any(world, pos, facing, &water_targets) {
            if !path.is_empty() {
                return Some(path.into_iter().collect());
            }
        }
    }

    if inv.food <= 6 {
        let mut food_targets = std::collections::HashSet::new();
        for obj in world.objects.values() {
            match obj {
                GameObject::Cow(cow) => {
                    food_targets.insert(cow.pos);
                }
                GameObject::Plant(plant) => {
                    if plant.is_ripe() {
                        food_targets.insert(plant.pos);
                    }
                }
                _ => {}
            }
        }
        if let Some(path) = find_path_to_face_any(world, pos, facing, &food_targets) {
            if !path.is_empty() {
                return Some(path.into_iter().collect());
            }
        }
    }

    let targets = find_targets(session);
    if !targets.is_empty() {
        if let Some(path) = find_path_to_face_any(world, pos, facing, &targets) {
            if !path.is_empty() {
                return Some(path.into_iter().collect());
            }
        }
    }

    if inv.wood >= 4 && !world.has_adjacent_table(pos) {
        if world.get_material((pos.0 + facing.0 as i32, pos.1 + facing.1 as i32))
            == Some(Material::Grass)
        {
            return Some([crafter_core::Action::PlaceTable].into_iter().collect());
        }
    }

    if inv.stone >= 5 && !world.has_adjacent_furnace(pos) {
        if world.get_material((pos.0 + facing.0 as i32, pos.1 + facing.1 as i32))
            == Some(Material::Grass)
        {
            return Some([crafter_core::Action::PlaceFurnace].into_iter().collect());
        }
    }

    None
}

fn choose_action(session: &Session, last_action: crafter_core::Action) -> crafter_core::Action {
    let state = session.get_state();
    let inv = &state.inventory;
    let ach = &state.achievements;
    let pos = state.player_pos;
    let facing = state.player_facing;
    let facing_pos = (pos.0 + facing.0 as i32, pos.1 + facing.1 as i32);
    let world = &session.world;

    if inv.health <= 4 && inv.potion_red > 0 {
        return crafter_core::Action::DrinkPotionRed;
    }
    if inv.energy <= 3 && inv.potion_green > 0 {
        return crafter_core::Action::DrinkPotionGreen;
    }
    if inv.drink <= 3 && inv.potion_blue > 0 {
        return crafter_core::Action::DrinkPotionBlue;
    }
    if inv.food <= 3 && inv.potion_pink > 0 {
        return crafter_core::Action::DrinkPotionPink;
    }
    if inv.health <= 4 && inv.energy <= 4 && inv.potion_cyan > 0 {
        return crafter_core::Action::DrinkPotionCyan;
    }
    if inv.food <= 4 && inv.drink <= 4 && inv.potion_yellow > 0 {
        return crafter_core::Action::DrinkPotionYellow;
    }
    if ach.drink_potion == 0 {
        if inv.potion_red > 0 {
            return crafter_core::Action::DrinkPotionRed;
        }
        if inv.potion_green > 0 {
            return crafter_core::Action::DrinkPotionGreen;
        }
        if inv.potion_blue > 0 {
            return crafter_core::Action::DrinkPotionBlue;
        }
        if inv.potion_pink > 0 {
            return crafter_core::Action::DrinkPotionPink;
        }
        if inv.potion_cyan > 0 {
            return crafter_core::Action::DrinkPotionCyan;
        }
        if inv.potion_yellow > 0 {
            return crafter_core::Action::DrinkPotionYellow;
        }
    }

    if inv.drink <= 6 {
        if world.get_material(facing_pos) == Some(Material::Water) {
            return crafter_core::Action::Do;
        }
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let check = (pos.0 + dx, pos.1 + dy);
            if world.get_material(check) == Some(Material::Water) {
                return match (dx, dy) {
                    (-1, 0) => crafter_core::Action::MoveLeft,
                    (1, 0) => crafter_core::Action::MoveRight,
                    (0, -1) => crafter_core::Action::MoveUp,
                    (0, 1) => crafter_core::Action::MoveDown,
                    _ => crafter_core::Action::Noop,
                };
            }
        }
    }

    if inv.food <= 6 {
        if let Some(GameObject::Plant(plant)) = world.get_object_at(facing_pos) {
            if plant.is_ripe() {
                return crafter_core::Action::Do;
            }
        }
        for obj in world.objects.values() {
            if let GameObject::Cow(cow) = obj {
                let dist = (cow.pos.0 - pos.0).abs() + (cow.pos.1 - pos.1).abs();
                if dist <= 1 && facing_pos == cow.pos {
                    return crafter_core::Action::Do;
                }
                if dist <= 6 {
                    if cow.pos.0 < pos.0 {
                        return crafter_core::Action::MoveLeft;
                    }
                    if cow.pos.0 > pos.0 {
                        return crafter_core::Action::MoveRight;
                    }
                    if cow.pos.1 < pos.1 {
                        return crafter_core::Action::MoveUp;
                    }
                    if cow.pos.1 > pos.1 {
                        return crafter_core::Action::MoveDown;
                    }
                }
            }
        }
    }

    if ach.eat_cow == 0 {
        if let Some(GameObject::Cow(cow)) = world.get_object_at(facing_pos) {
            let _ = cow;
            return crafter_core::Action::Do;
        }
    }
    if ach.eat_plant == 0 {
        if let Some(GameObject::Plant(plant)) = world.get_object_at(facing_pos) {
            if plant.is_ripe() {
                return crafter_core::Action::Do;
            }
        }
    }

    if let Some(mat) = world.get_material(facing_pos) {
        let can_collect = match mat {
            Material::Tree => inv.wood < 9,
            Material::Stone => inv.best_pickaxe_tier() >= 1 && inv.stone < 9,
            Material::Coal => inv.best_pickaxe_tier() >= 1 && inv.coal < 9,
            Material::Iron => inv.best_pickaxe_tier() >= 2 && inv.iron < 9,
            Material::Diamond => inv.best_pickaxe_tier() >= 3 && inv.diamond < 9,
            Material::Sapphire => inv.best_pickaxe_tier() >= 4 && inv.sapphire < 9,
            Material::Ruby => inv.best_pickaxe_tier() >= 4 && inv.ruby < 9,
            Material::Grass => inv.sapling < 3,
            _ => false,
        };
        if can_collect {
            return crafter_core::Action::Do;
        }
    }

    if world.get_material(facing_pos) == Some(Material::Chest) {
        return crafter_core::Action::Do;
    }

    if let Some(GameObject::Plant(plant)) = world.get_object_at(facing_pos) {
        if plant.is_ripe() {
            return crafter_core::Action::Do;
        }
    }

    if inv.best_sword_tier() == 0 {
        for obj in world.objects.values() {
            if obj.is_hostile() {
                let dist = (obj.position().0 - pos.0).abs() + (obj.position().1 - pos.1).abs();
                if dist <= 2 {
                    if obj.position().0 < pos.0 {
                        return crafter_core::Action::MoveRight;
                    }
                    if obj.position().0 > pos.0 {
                        return crafter_core::Action::MoveLeft;
                    }
                    if obj.position().1 < pos.1 {
                        return crafter_core::Action::MoveDown;
                    }
                    if obj.position().1 > pos.1 {
                        return crafter_core::Action::MoveUp;
                    }
                }
            }
        }
    }

    if inv.best_sword_tier() > 0 {
        if let Some(obj) = world.get_object_at(facing_pos) {
            if obj.is_hostile() {
                return crafter_core::Action::Do;
            }
        }
    }

    if inv.bow > 0 && inv.arrows > 0 {
        if let Some(obj) = world.get_object_at(facing_pos) {
            if obj.is_hostile() {
                return crafter_core::Action::ShootArrow;
            }
        }
    }

    let near_table = world.has_adjacent_table(pos);
    let near_furnace = world.has_adjacent_furnace(pos);

    if near_table {
        if inv.wood_sword == 0 && inv.wood >= 1 {
            return crafter_core::Action::MakeWoodSword;
        }
        if inv.wood_pickaxe == 0 && inv.wood >= 1 {
            return crafter_core::Action::MakeWoodPickaxe;
        }
        if inv.stone_sword == 0 && inv.wood >= 1 && inv.stone >= 1 {
            return crafter_core::Action::MakeStoneSword;
        }
        if inv.stone_pickaxe == 0 && inv.wood >= 1 && inv.stone >= 1 {
            return crafter_core::Action::MakeStonePickaxe;
        }
        if near_furnace {
            if inv.iron_sword == 0 && inv.wood >= 1 && inv.coal >= 1 && inv.iron >= 1 {
                return crafter_core::Action::MakeIronSword;
            }
            if inv.iron_pickaxe == 0 && inv.wood >= 1 && inv.coal >= 1 && inv.iron >= 1 {
                return crafter_core::Action::MakeIronPickaxe;
            }
        }
        if inv.diamond_pickaxe == 0 && inv.wood >= 1 && inv.diamond >= 1 {
            return crafter_core::Action::MakeDiamondPickaxe;
        }
        if inv.diamond_sword == 0 && inv.wood >= 1 && inv.diamond >= 2 {
            return crafter_core::Action::MakeDiamondSword;
        }
        if inv.bow == 0 && inv.wood >= 2 {
            return crafter_core::Action::MakeBow;
        }
        if inv.arrows < 3 && inv.wood >= 1 && inv.stone >= 1 {
            return crafter_core::Action::MakeArrow;
        }
        if inv.armor_helmet < 2 && inv.diamond >= 3 {
            return crafter_core::Action::MakeDiamondArmor;
        }
        if near_furnace && inv.armor_helmet == 0 && inv.iron >= 3 && inv.coal >= 3 {
            return crafter_core::Action::MakeIronArmor;
        }
    }

    if inv.wood >= 4 && !near_table && ach.place_table == 0 {
        if world.get_material(facing_pos) == Some(Material::Grass) {
            return crafter_core::Action::PlaceTable;
        }
    }
    if inv.stone >= 5 && !near_furnace && ach.place_furnace == 0 {
        if world.get_material(facing_pos) == Some(Material::Grass) {
            return crafter_core::Action::PlaceFurnace;
        }
    }
    if inv.stone > 0 && ach.place_stone == 0 {
        if world.get_material(facing_pos) == Some(Material::Grass) {
            return crafter_core::Action::PlaceStone;
        }
    }
    if inv.sapling > 0 && ach.place_plant == 0 {
        if world.get_material(facing_pos) == Some(Material::Grass) {
            return crafter_core::Action::PlacePlant;
        }
    }

    if inv.energy <= 2 && !state.player_sleeping {
        let mut safe = true;
        for obj in world.objects.values() {
            if obj.is_hostile() {
                let dist = (obj.position().0 - pos.0).abs() + (obj.position().1 - pos.1).abs();
                if dist < 8 {
                    safe = false;
                    break;
                }
            }
        }
        if safe {
            return crafter_core::Action::Sleep;
        }
    }

    if let Some(action) = explore_step(world, pos, last_action) {
        return action;
    }

    crafter_core::Action::Noop
}

fn explore_step(world: &crafter_core::world::World, pos: (i32, i32), _last: crafter_core::Action) -> Option<crafter_core::Action> {
    let dirs = [
        (crafter_core::Action::MoveUp, (0, -1)),
        (crafter_core::Action::MoveDown, (0, 1)),
        (crafter_core::Action::MoveLeft, (-1, 0)),
        (crafter_core::Action::MoveRight, (1, 0)),
    ];
    for (action, (dx, dy)) in dirs {
        let next = (pos.0 + dx, pos.1 + dy);
        if world.is_walkable(next) {
            return Some(action);
        }
    }
    None
}

fn hostile_nearby(session: &Session, radius: i32) -> bool {
    let state = session.get_state();
    let pos = state.player_pos;
    for obj in session.world.objects.values() {
        if obj.is_hostile() {
            let dist = (obj.position().0 - pos.0).abs() + (obj.position().1 - pos.1).abs();
            if dist <= radius {
                return true;
            }
        }
    }
    false
}

fn flee_action(session: &Session, radius: i32) -> Option<crafter_core::Action> {
    let state = session.get_state();
    let pos = state.player_pos;
    let mut nearest: Option<((i32, i32), i32)> = None;

    for obj in session.world.objects.values() {
        if obj.is_hostile() {
            let dist = (obj.position().0 - pos.0).abs() + (obj.position().1 - pos.1).abs();
            if dist <= radius {
                if nearest.map(|(_, d)| dist < d).unwrap_or(true) {
                    nearest = Some((obj.position(), dist));
                }
            }
        }
    }

    let (mob_pos, _) = nearest?;
    let dx = pos.0 - mob_pos.0;
    let dy = pos.1 - mob_pos.1;

    let candidates = if dx.abs() >= dy.abs() {
        if dx >= 0 {
            [crafter_core::Action::MoveRight, crafter_core::Action::MoveLeft]
        } else {
            [crafter_core::Action::MoveLeft, crafter_core::Action::MoveRight]
        }
    } else if dy >= 0 {
        [crafter_core::Action::MoveDown, crafter_core::Action::MoveUp]
    } else {
        [crafter_core::Action::MoveUp, crafter_core::Action::MoveDown]
    };

    for action in candidates {
        if let Some((dx, dy)) = action.movement_delta() {
            let next = (pos.0 + dx, pos.1 + dy);
            if session.world.is_walkable(next) {
                return Some(action);
            }
        }
    }

    None
}

fn craft_action_if_available(session: &Session) -> Option<crafter_core::Action> {
    let state = session.get_state();
    let inv = &state.inventory;
    let pos = state.player_pos;
    let world = &session.world;
    let near_table = world.has_adjacent_table(pos);
    let near_furnace = world.has_adjacent_furnace(pos);

    if !near_table {
        return None;
    }

    if inv.wood_sword == 0 && inv.wood >= 1 {
        return Some(crafter_core::Action::MakeWoodSword);
    }
    if inv.wood_pickaxe == 0 && inv.wood >= 1 {
        return Some(crafter_core::Action::MakeWoodPickaxe);
    }
    if inv.stone_sword == 0 && inv.wood >= 1 && inv.stone >= 1 {
        return Some(crafter_core::Action::MakeStoneSword);
    }
    if inv.stone_pickaxe == 0 && inv.wood >= 1 && inv.stone >= 1 {
        return Some(crafter_core::Action::MakeStonePickaxe);
    }
    if near_furnace {
        if inv.iron_sword == 0 && inv.wood >= 1 && inv.coal >= 1 && inv.iron >= 1 {
            return Some(crafter_core::Action::MakeIronSword);
        }
        if inv.iron_pickaxe == 0 && inv.wood >= 1 && inv.coal >= 1 && inv.iron >= 1 {
            return Some(crafter_core::Action::MakeIronPickaxe);
        }
    }
    if inv.diamond_pickaxe == 0 && inv.wood >= 1 && inv.diamond >= 1 {
        return Some(crafter_core::Action::MakeDiamondPickaxe);
    }
    if inv.diamond_sword == 0 && inv.wood >= 1 && inv.diamond >= 2 {
        return Some(crafter_core::Action::MakeDiamondSword);
    }
    if inv.bow == 0 && inv.wood >= 2 {
        return Some(crafter_core::Action::MakeBow);
    }
    if inv.arrows < 3 && inv.wood >= 1 && inv.stone >= 1 {
        return Some(crafter_core::Action::MakeArrow);
    }
    if inv.armor_helmet < 2 && inv.diamond >= 3 {
        return Some(crafter_core::Action::MakeDiamondArmor);
    }
    if near_furnace && inv.armor_helmet == 0 && inv.iron >= 3 && inv.coal >= 3 {
        return Some(crafter_core::Action::MakeIronArmor);
    }

    None
}

fn find_targets(session: &Session) -> std::collections::HashSet<(i32, i32)> {
    let state = session.get_state();
    let inv = &state.inventory;
    let world = &session.world;
    let mut targets = std::collections::HashSet::new();

    let mut push_mat = |mat: Material| {
        for y in 0..world.area.1 {
            for x in 0..world.area.0 {
                let pos = (x as i32, y as i32);
                if world.get_material(pos) == Some(mat) {
                    targets.insert(pos);
                }
            }
        }
    };

    if inv.diamond_pickaxe > 0 {
        push_mat(Material::Sapphire);
        push_mat(Material::Ruby);
    }
    if inv.iron_pickaxe > 0 {
        push_mat(Material::Diamond);
    }
    if inv.stone_pickaxe > 0 {
        push_mat(Material::Iron);
    }
    if inv.wood_pickaxe > 0 {
        push_mat(Material::Coal);
        push_mat(Material::Stone);
    }
    if inv.wood < 6 {
        push_mat(Material::Tree);
    }
    push_mat(Material::Chest);
    if inv.drink <= 6 {
        push_mat(Material::Water);
    }
    if inv.wood >= 4 && state.achievements.place_table == 0 && !world.has_adjacent_table(state.player_pos) {
        push_mat(Material::Grass);
    }
    if inv.stone >= 5
        && world.has_adjacent_table(state.player_pos)
        && !world.has_adjacent_furnace(state.player_pos)
        && state.achievements.place_furnace == 0
    {
        push_mat(Material::Grass);
    }
    if inv.stone > 0 && state.achievements.place_stone == 0 {
        push_mat(Material::Grass);
    }
    if inv.sapling > 0 && state.achievements.place_plant == 0 {
        push_mat(Material::Grass);
    }

    if inv.best_sword_tier() >= 2 || (inv.bow > 0 && inv.arrows > 0) {
        for obj in world.objects.values() {
            if obj.is_hostile() {
                targets.insert(obj.position());
            }
        }
    }

    if inv.food <= 6 {
        for obj in world.objects.values() {
            match obj {
                GameObject::Cow(cow) => {
                    targets.insert(cow.pos);
                }
                GameObject::Plant(plant) => {
                    if plant.is_ripe() {
                        targets.insert(plant.pos);
                    }
                }
                _ => {}
            }
        }
    }

    if state.achievements.eat_cow == 0 || state.achievements.eat_plant == 0 {
        for obj in world.objects.values() {
            match obj {
                GameObject::Cow(cow) if state.achievements.eat_cow == 0 => {
                    targets.insert(cow.pos);
                }
                GameObject::Plant(plant) if state.achievements.eat_plant == 0 && plant.is_ripe() => {
                    targets.insert(plant.pos);
                }
                _ => {}
            }
        }
    }

    targets
}

fn find_path_to_face_any(
    world: &crafter_core::world::World,
    start_pos: (i32, i32),
    start_facing: (i8, i8),
    targets: &std::collections::HashSet<(i32, i32)>,
) -> Option<Vec<crafter_core::Action>> {
    use std::collections::{HashMap, VecDeque};
    let dirs = [
        (crafter_core::Action::MoveUp, (0, -1)),
        (crafter_core::Action::MoveDown, (0, 1)),
        (crafter_core::Action::MoveLeft, (-1, 0)),
        (crafter_core::Action::MoveRight, (1, 0)),
    ];
    let mut queue = VecDeque::new();
    let mut came_from: HashMap<((i32, i32), (i8, i8)), (((i32, i32), (i8, i8)), crafter_core::Action)> = HashMap::new();
    let start = (start_pos, start_facing);
    queue.push_back(start);
    let mut visited = std::collections::HashSet::new();
    visited.insert(start);

    while let Some((pos, facing)) = queue.pop_front() {
        let facing_pos = (pos.0 + facing.0 as i32, pos.1 + facing.1 as i32);
        if targets.contains(&facing_pos) {
            let mut actions = Vec::new();
            let mut current = (pos, facing);
            while current != start {
                if let Some((prev, action)) = came_from.get(&current) {
                    actions.push(*action);
                    current = *prev;
                } else {
                    break;
                }
            }
            actions.reverse();
            return Some(actions);
        }

        for (action, (dx, dy)) in dirs {
            let next_pos = (pos.0 + dx, pos.1 + dy);
            let next_facing = (dx as i8, dy as i8);
            if !world.is_walkable(next_pos) {
                continue;
            }
            let next_state = (next_pos, next_facing);
            if visited.insert(next_state) {
                came_from.insert(next_state, ((pos, facing), action));
                queue.push_back(next_state);
            }
        }
    }

    None
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
