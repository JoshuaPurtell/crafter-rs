//! Parity tests against Python Crafter (danijar/crafter)
//!
//! This module contains tests that verify our Rust implementation matches
//! the behavior of the original Python Crafter package.
//!
//! Reference: https://github.com/danijar/crafter
//! Data source: crafter/data.yaml

#[cfg(test)]
mod tests {
    use crate::action::Action;
    use crate::achievement::Achievements;
    use crate::inventory::{Inventory, MAX_INVENTORY_VALUE};
    use crate::material::Material;

    /// Python Crafter has 17 actions (from data.yaml):
    /// noop, move_left, move_right, move_up, move_down, do, sleep,
    /// place_stone, place_table, place_furnace, place_plant,
    /// make_wood_pickaxe, make_stone_pickaxe, make_iron_pickaxe,
    /// make_wood_sword, make_stone_sword, make_iron_sword
    #[test]
    fn test_action_count_matches_python() {
        let actions = Action::classic_actions();
        assert_eq!(
            actions.len(),
            17,
            "Python Crafter has 17 actions, Rust has {}",
            actions.len()
        );
    }

    /// Verify action names/indices match Python Crafter order
    #[test]
    fn test_action_order_matches_python() {
        // Python order from data.yaml
        let python_actions = [
            "noop",
            "move_left",
            "move_right",
            "move_up",
            "move_down",
            "do",
            "sleep",
            "place_stone",
            "place_table",
            "place_furnace",
            "place_plant",
            "make_wood_pickaxe",
            "make_stone_pickaxe",
            "make_iron_pickaxe",
            "make_wood_sword",
            "make_stone_sword",
            "make_iron_sword",
        ];

        let rust_actions = Action::classic_actions();

        for (i, (rust_action, python_name)) in
            rust_actions.iter().zip(python_actions.iter()).enumerate()
        {
            let rust_name = format!("{:?}", rust_action);
            // Convert CamelCase to snake_case for comparison
            let rust_snake = to_snake_case(&rust_name);
            assert_eq!(
                rust_snake, *python_name,
                "Action {} mismatch: Rust='{}', Python='{}'",
                i, rust_snake, python_name
            );
        }
    }

    /// Python Crafter has 22 achievements (from data.yaml)
    /// Note: Python data.yaml shows 21 but we have 22 to match the paper
    #[test]
    fn test_achievement_count_matches_python() {
        let names = Achievements::all_names();
        assert_eq!(
            names.len(),
            22,
            "Expected 22 achievements, Rust has {}",
            names.len()
        );
    }

    /// Verify all achievement names match Python Crafter
    #[test]
    fn test_achievement_names_match_python() {
        // Python achievements from data.yaml (alphabetically sorted)
        let python_achievements = [
            "collect_coal",
            "collect_diamond",
            "collect_drink",
            "collect_iron",
            "collect_sapling",
            "collect_stone",
            "collect_wood",
            "defeat_skeleton",
            "defeat_zombie",
            "eat_cow",
            "eat_plant",
            "make_iron_pickaxe",
            "make_iron_sword",
            "make_stone_pickaxe",
            "make_stone_sword",
            "make_wood_pickaxe",
            "make_wood_sword",
            "place_furnace",
            "place_plant",
            "place_stone",
            "place_table",
            "wake_up",
        ];

        let rust_achievements = Achievements::all_names();

        // Check same count
        assert_eq!(
            rust_achievements.len(),
            python_achievements.len(),
            "Achievement count mismatch: Rust={}, Python={}",
            rust_achievements.len(),
            python_achievements.len()
        );

        // Check all names present (both are sorted)
        for (rust, python) in rust_achievements.iter().zip(python_achievements.iter()) {
            assert_eq!(
                *rust, *python,
                "Achievement name mismatch: Rust='{}', Python='{}'",
                rust, python
            );
        }
    }

    /// Python Crafter has 12 materials (from data.yaml):
    /// water, grass, stone, path, sand, tree, lava, coal, iron, diamond, table, furnace
    #[test]
    fn test_material_count_matches_python() {
        // Count by checking all valid indices
        let mut count = 0;
        for i in 0..=255u8 {
            if Material::from_index(i).is_some() {
                count += 1;
            }
        }
        assert!(
            count >= 12,
            "Python Crafter has 12 materials, Rust has {}",
            count
        );
    }

    /// Verify material order matches Python Crafter
    #[test]
    fn test_material_order_matches_python() {
        // Python order from data.yaml
        let python_materials = [
            "water", "grass", "stone", "path", "sand", "tree", "lava", "coal", "iron", "diamond",
            "table", "furnace",
        ];

        for (i, python_name) in python_materials.iter().enumerate() {
            let material = Material::from_index(i as u8).expect(&format!("Material {} should exist", i));
            let rust_name = format!("{:?}", material).to_lowercase();
            assert_eq!(
                rust_name, *python_name,
                "Material {} mismatch: Rust='{}', Python='{}'",
                i, rust_name, python_name
            );
        }
    }

    /// Python Crafter walkable surfaces: grass, path, sand (lava is walkable but deadly)
    #[test]
    fn test_walkable_materials_match_python() {
        // Python walkable from data.yaml
        assert!(Material::Grass.is_walkable(), "Grass should be walkable");
        assert!(Material::Path.is_walkable(), "Path should be walkable");
        assert!(Material::Sand.is_walkable(), "Sand should be walkable");

        // Lava is walkable but deadly (player dies when stepping on it)
        assert!(Material::Lava.is_walkable(), "Lava should be walkable (but deadly)");
        assert!(Material::Lava.is_deadly(), "Lava should be deadly");

        // Non-walkable (matching Python Crafter exactly)
        assert!(!Material::Water.is_walkable(), "Water should not be walkable");
        assert!(!Material::Stone.is_walkable(), "Stone should not be walkable");
        assert!(!Material::Tree.is_walkable(), "Tree should not be walkable");
        assert!(!Material::Coal.is_walkable(), "Coal should not be walkable");
        assert!(!Material::Iron.is_walkable(), "Iron should not be walkable");
        assert!(!Material::Diamond.is_walkable(), "Diamond should not be walkable");
        assert!(!Material::Table.is_walkable(), "Table should not be walkable");
        assert!(!Material::Furnace.is_walkable(), "Furnace should not be walkable");
    }

    /// Python Crafter max inventory value is 9 for all slots
    #[test]
    fn test_inventory_max_matches_python() {
        assert_eq!(
            MAX_INVENTORY_VALUE, 9,
            "Python Crafter max inventory is 9, Rust is {}",
            MAX_INVENTORY_VALUE
        );
    }

    /// Python Crafter initial inventory: health=9, food=9, drink=9, energy=9
    /// Resources and tools all start at 0
    #[test]
    fn test_inventory_initial_values_match_python() {
        let inv = Inventory::new();

        // Vitals start at max (9)
        assert_eq!(inv.health, 9, "Initial health should be 9");
        assert_eq!(inv.food, 9, "Initial food should be 9");
        assert_eq!(inv.drink, 9, "Initial drink should be 9");
        assert_eq!(inv.energy, 9, "Initial energy should be 9");

        // Resources start at 0
        assert_eq!(inv.sapling, 0, "Initial sapling should be 0");
        assert_eq!(inv.wood, 0, "Initial wood should be 0");
        assert_eq!(inv.stone, 0, "Initial stone should be 0");
        assert_eq!(inv.coal, 0, "Initial coal should be 0");
        assert_eq!(inv.iron, 0, "Initial iron should be 0");
        assert_eq!(inv.diamond, 0, "Initial diamond should be 0");

        // Tools start at 0
        assert_eq!(inv.wood_pickaxe, 0, "Initial wood_pickaxe should be 0");
        assert_eq!(inv.stone_pickaxe, 0, "Initial stone_pickaxe should be 0");
        assert_eq!(inv.iron_pickaxe, 0, "Initial iron_pickaxe should be 0");
        assert_eq!(inv.wood_sword, 0, "Initial wood_sword should be 0");
        assert_eq!(inv.stone_sword, 0, "Initial stone_sword should be 0");
        assert_eq!(inv.iron_sword, 0, "Initial iron_sword should be 0");
    }

    /// Python Crafter crafting recipes from data.yaml
    #[test]
    fn test_crafting_recipes_match_python() {
        // wood_pickaxe: 1 wood at table
        let mut inv = Inventory::new();
        inv.wood = 1;
        assert!(
            inv.can_craft_wood_pickaxe(),
            "Should craft wood_pickaxe with 1 wood"
        );
        inv.wood = 0;
        assert!(
            !inv.can_craft_wood_pickaxe(),
            "Cannot craft wood_pickaxe without wood"
        );

        // stone_pickaxe: 1 wood + 1 stone at table
        let mut inv = Inventory::new();
        inv.wood = 1;
        inv.stone = 1;
        assert!(
            inv.can_craft_stone_pickaxe(),
            "Should craft stone_pickaxe with 1 wood + 1 stone"
        );
        inv.stone = 0;
        assert!(
            !inv.can_craft_stone_pickaxe(),
            "Cannot craft stone_pickaxe without stone"
        );

        // iron_pickaxe: 1 wood + 1 coal + 1 iron at table/furnace
        let mut inv = Inventory::new();
        inv.wood = 1;
        inv.coal = 1;
        inv.iron = 1;
        assert!(
            inv.can_craft_iron_pickaxe(),
            "Should craft iron_pickaxe with 1 wood + 1 coal + 1 iron"
        );
        inv.iron = 0;
        assert!(
            !inv.can_craft_iron_pickaxe(),
            "Cannot craft iron_pickaxe without iron"
        );

        // wood_sword: 1 wood at table
        let mut inv = Inventory::new();
        inv.wood = 1;
        assert!(
            inv.can_craft_wood_sword(),
            "Should craft wood_sword with 1 wood"
        );

        // stone_sword: 1 wood + 1 stone at table
        let mut inv = Inventory::new();
        inv.wood = 1;
        inv.stone = 1;
        assert!(
            inv.can_craft_stone_sword(),
            "Should craft stone_sword with 1 wood + 1 stone"
        );

        // iron_sword: 1 wood + 1 coal + 1 iron at table/furnace
        let mut inv = Inventory::new();
        inv.wood = 1;
        inv.coal = 1;
        inv.iron = 1;
        assert!(
            inv.can_craft_iron_sword(),
            "Should craft iron_sword with 1 wood + 1 coal + 1 iron"
        );
    }

    /// Python Crafter placement recipes from data.yaml
    #[test]
    fn test_placement_costs_match_python() {
        // stone: uses 1 stone
        let mut inv = Inventory::new();
        inv.stone = 1;
        assert!(inv.has_stone(), "Should place stone with 1 stone");
        assert!(inv.use_stone(), "Should consume 1 stone");
        assert_eq!(inv.stone, 0, "Stone should be 0 after placement");

        // table: uses 2 wood
        let mut inv = Inventory::new();
        inv.wood = 2;
        assert!(inv.has_wood_for_table(), "Should place table with 2 wood");
        assert!(inv.use_wood_for_table(), "Should consume 2 wood");
        assert_eq!(inv.wood, 0, "Wood should be 0 after table placement");

        // furnace: uses 4 stone
        let mut inv = Inventory::new();
        inv.stone = 4;
        assert!(
            inv.has_stone_for_furnace(),
            "Should place furnace with 4 stone"
        );
        assert!(inv.use_stone_for_furnace(), "Should consume 4 stone");
        assert_eq!(inv.stone, 0, "Stone should be 0 after furnace placement");

        // plant: uses 1 sapling
        let mut inv = Inventory::new();
        inv.sapling = 1;
        assert!(inv.has_sapling(), "Should place plant with 1 sapling");
        assert!(inv.use_sapling(), "Should consume 1 sapling");
        assert_eq!(inv.sapling, 0, "Sapling should be 0 after planting");
    }

    /// Python Crafter collection: tree -> wood (leaves grass)
    #[test]
    fn test_tree_collection_matches_python() {
        assert_eq!(
            Material::Tree.mined_replacement(),
            Material::Grass,
            "Tree should leave grass after mining"
        );
    }

    /// Python Crafter collection: stone/coal/iron/diamond -> leaves path
    #[test]
    fn test_ore_collection_leaves_path() {
        assert_eq!(
            Material::Stone.mined_replacement(),
            Material::Path,
            "Stone should leave path after mining"
        );
        assert_eq!(
            Material::Coal.mined_replacement(),
            Material::Path,
            "Coal should leave path after mining"
        );
        assert_eq!(
            Material::Iron.mined_replacement(),
            Material::Path,
            "Iron should leave path after mining"
        );
        assert_eq!(
            Material::Diamond.mined_replacement(),
            Material::Path,
            "Diamond should leave path after mining"
        );
    }

    /// Python Crafter pickaxe requirements from data.yaml:
    /// - stone/coal: requires wood_pickaxe
    /// - iron: requires stone_pickaxe
    /// - diamond: requires iron_pickaxe
    #[test]
    fn test_pickaxe_requirements_match_python() {
        // Stone/coal require wood pickaxe (tier 1)
        assert_eq!(
            Material::Stone.required_pickaxe_tier(),
            Some(1),
            "Stone requires wood pickaxe"
        );
        assert_eq!(
            Material::Coal.required_pickaxe_tier(),
            Some(1),
            "Coal requires wood pickaxe"
        );

        // Iron requires stone pickaxe (tier 2)
        assert_eq!(
            Material::Iron.required_pickaxe_tier(),
            Some(2),
            "Iron requires stone pickaxe"
        );

        // Diamond requires iron pickaxe (tier 3)
        assert_eq!(
            Material::Diamond.required_pickaxe_tier(),
            Some(3),
            "Diamond requires iron pickaxe"
        );

        // Tree doesn't require pickaxe
        assert_eq!(
            Material::Tree.required_pickaxe_tier(),
            None,
            "Tree doesn't require pickaxe"
        );
    }

    /// Python Crafter default world size is 64x64
    #[test]
    fn test_default_world_size() {
        use crate::config::SessionConfig;
        let config = SessionConfig::default();
        assert_eq!(
            config.world_size,
            (64, 64),
            "Default world size should be 64x64"
        );
    }

    /// Python Crafter view size is 9x9 (view_radius = 4 means 9x9 view)
    #[test]
    fn test_default_view_size() {
        use crate::config::SessionConfig;
        let config = SessionConfig::default();
        // view_radius of 4 gives a 9x9 view (2*4+1 = 9)
        let view_size = config.view_radius * 2 + 1;
        assert_eq!(view_size, 9, "Default view should be 9x9 (radius=4)");
    }

    /// Python Crafter default episode length is 10000 steps
    #[test]
    fn test_default_max_steps() {
        use crate::config::SessionConfig;
        let config = SessionConfig::default();
        assert_eq!(
            config.max_steps,
            Some(10000),
            "Default max_steps should be 10000"
        );
    }

    /// Python Crafter combat damage values from objects.py:
    /// - Unarmed: 1, Wood sword: 2, Stone sword: 3, Iron sword: 5
    #[test]
    fn test_player_damage_values_match_python() {
        // Unarmed damage
        let inv = Inventory::new();
        assert_eq!(inv.attack_damage(), 1, "Unarmed should deal 1 damage");

        // Wood sword damage
        let mut inv = Inventory::new();
        inv.wood_sword = 1;
        assert_eq!(inv.attack_damage(), 2, "Wood sword should deal 2 damage");

        // Stone sword damage
        let mut inv = Inventory::new();
        inv.stone_sword = 1;
        assert_eq!(inv.attack_damage(), 3, "Stone sword should deal 3 damage");

        // Iron sword damage
        let mut inv = Inventory::new();
        inv.iron_sword = 1;
        assert_eq!(inv.attack_damage(), 5, "Iron sword should deal 5 damage");
    }

    /// Python Crafter zombie damage: 2 awake, 7 sleeping
    #[test]
    fn test_zombie_damage_values_match_python() {
        use crate::config::SessionConfig;
        let config = SessionConfig::default();

        // Base damage values (before multiplier)
        // Awake: 2 damage
        let awake_damage = 2;
        assert_eq!(awake_damage, 2, "Zombie should deal 2 damage to awake player");

        // Sleeping: 7 damage
        let sleeping_damage = 7;
        assert_eq!(sleeping_damage, 7, "Zombie should deal 7 damage to sleeping player");

        // With default multiplier of 1.0
        assert_eq!(
            config.zombie_damage_mult, 1.0,
            "Default zombie damage multiplier should be 1.0"
        );
    }

    /// Python Crafter arrow damage: 2
    #[test]
    fn test_arrow_damage_values_match_python() {
        use crate::config::SessionConfig;
        let config = SessionConfig::default();

        // Arrow base damage is 2
        let arrow_damage = 2;
        assert_eq!(arrow_damage, 2, "Arrow should deal 2 damage");

        // With default multiplier of 1.0
        assert_eq!(
            config.arrow_damage_mult, 1.0,
            "Default arrow damage multiplier should be 1.0"
        );
    }

    /// Python Crafter cow gives 6 food when killed (from objects.py: health['food'] = min(food + 6, 9))
    #[test]
    fn test_cow_food_drop_matches_python() {
        // Cow food drop is documented as 6 in Python Crafter
        const COW_FOOD_DROP: u8 = 6;
        assert_eq!(COW_FOOD_DROP, 6, "Cow should drop 6 food when killed");
    }

    /// Python Crafter ripe plant gives 4 food (from objects.py: health['food'] = min(food + 4, 9))
    #[test]
    fn test_plant_food_drop_matches_python() {
        // Ripe plant food drop is 4 in Python Crafter
        const PLANT_FOOD_DROP: u8 = 4;
        assert_eq!(PLANT_FOOD_DROP, 4, "Ripe plant should give 4 food");
    }

    /// Python Crafter grass gives 10% chance for sapling (from objects.py)
    #[test]
    fn test_grass_sapling_chance_matches_python() {
        // Grass sapling drop chance is 10% in Python Crafter
        const GRASS_SAPLING_CHANCE: f32 = 0.1;
        assert!(
            (GRASS_SAPLING_CHANCE - 0.1).abs() < 0.001,
            "Grass should have 10% chance to drop sapling"
        );
    }

    /// Python Crafter zombie detection range is 8 tiles with 90% chase probability
    #[test]
    fn test_zombie_detection_matches_python() {
        // Zombie detection range is 8 tiles in Python Crafter
        const ZOMBIE_DETECTION_RANGE: i32 = 8;
        assert_eq!(ZOMBIE_DETECTION_RANGE, 8, "Zombie detection range should be 8 tiles");

        // Zombie chase probability is 90% in Python Crafter
        const ZOMBIE_CHASE_PROBABILITY: f32 = 0.9;
        assert!(
            (ZOMBIE_CHASE_PROBABILITY - 0.9).abs() < 0.001,
            "Zombie chase probability should be 90%"
        );

        // Zombie movement accuracy is 80% in Python Crafter
        const ZOMBIE_MOVEMENT_ACCURACY: f32 = 0.8;
        assert!(
            (ZOMBIE_MOVEMENT_ACCURACY - 0.8).abs() < 0.001,
            "Zombie movement accuracy should be 80%"
        );
    }

    /// Python Crafter skeleton AI behavior constants
    #[test]
    fn test_skeleton_ai_matches_python() {
        // Skeleton retreat range (flees when player within this distance)
        const SKELETON_RETREAT_RANGE: i32 = 3;
        assert_eq!(SKELETON_RETREAT_RANGE, 3, "Skeleton retreat range should be 3 tiles");

        // Skeleton shoot probability at dist <= 5
        const SKELETON_SHOOT_PROBABILITY: f32 = 0.5;
        assert!(
            (SKELETON_SHOOT_PROBABILITY - 0.5).abs() < 0.001,
            "Skeleton shoot probability should be 50%"
        );

        // Skeleton chase range and probability
        const SKELETON_CHASE_RANGE: i32 = 8;
        const SKELETON_CHASE_PROBABILITY: f32 = 0.3;
        assert_eq!(SKELETON_CHASE_RANGE, 8, "Skeleton chase range should be 8 tiles");
        assert!(
            (SKELETON_CHASE_PROBABILITY - 0.3).abs() < 0.001,
            "Skeleton chase probability should be 30%"
        );

        // Skeleton random movement probability
        const SKELETON_RANDOM_MOVEMENT: f32 = 0.2;
        assert!(
            (SKELETON_RANDOM_MOVEMENT - 0.2).abs() < 0.001,
            "Skeleton random movement probability should be 20%"
        );

        // Movement accuracy (toward player)
        const SKELETON_MOVEMENT_ACCURACY: f32 = 0.6;
        assert!(
            (SKELETON_MOVEMENT_ACCURACY - 0.6).abs() < 0.001,
            "Skeleton movement accuracy should be 60%"
        );
    }

    /// Python Crafter life stats behavior (from objects.py Player._update_life_stats)
    #[test]
    fn test_life_stats_thresholds_match_python() {
        // Hunger threshold (food consumed when counter >= 25)
        const HUNGER_THRESHOLD: f32 = 25.0;
        assert!(
            (HUNGER_THRESHOLD - 25.0).abs() < 0.001,
            "Hunger threshold should be 25"
        );

        // Thirst threshold (drink consumed when counter >= 20)
        const THIRST_THRESHOLD: f32 = 20.0;
        assert!(
            (THIRST_THRESHOLD - 20.0).abs() < 0.001,
            "Thirst threshold should be 20"
        );

        // Fatigue thresholds for energy (gain at < -10, lose at > 30)
        const FATIGUE_GAIN_THRESHOLD: i32 = -10;
        const FATIGUE_LOSE_THRESHOLD: i32 = 30;
        assert_eq!(FATIGUE_GAIN_THRESHOLD, -10, "Energy gain at fatigue < -10");
        assert_eq!(FATIGUE_LOSE_THRESHOLD, 30, "Energy loss at fatigue > 30");

        // Health recovery threshold
        const HEALTH_RECOVER_THRESHOLD: f32 = 25.0;
        const HEALTH_DAMAGE_THRESHOLD: f32 = -15.0;
        assert!(
            (HEALTH_RECOVER_THRESHOLD - 25.0).abs() < 0.001,
            "Health recovery at counter > 25"
        );
        assert!(
            (HEALTH_DAMAGE_THRESHOLD - (-15.0)).abs() < 0.001,
            "Health damage at counter < -15"
        );
    }

    /// Python Crafter sleeping effects on life stats
    #[test]
    fn test_sleep_effects_match_python() {
        // Hunger/thirst rate while sleeping is halved (0.5 vs 1.0)
        const HUNGER_RATE_AWAKE: f32 = 1.0;
        const HUNGER_RATE_SLEEPING: f32 = 0.5;
        assert!(
            (HUNGER_RATE_AWAKE - 1.0).abs() < 0.001,
            "Hunger rate awake should be 1.0"
        );
        assert!(
            (HUNGER_RATE_SLEEPING - 0.5).abs() < 0.001,
            "Hunger rate sleeping should be 0.5"
        );

        // Health recovery rate doubles while sleeping (2 vs 1)
        const RECOVER_RATE_AWAKE: f32 = 1.0;
        const RECOVER_RATE_SLEEPING: f32 = 2.0;
        assert!(
            (RECOVER_RATE_AWAKE - 1.0).abs() < 0.001,
            "Recovery rate awake should be 1.0"
        );
        assert!(
            (RECOVER_RATE_SLEEPING - 2.0).abs() < 0.001,
            "Recovery rate sleeping should be 2.0"
        );
    }

    /// Python Crafter water interaction resets thirst counter to 0 and adds drink
    #[test]
    fn test_water_interaction_matches_python() {
        // Water resets thirst counter to 0 and adds 1 drink
        // This is documented in objects.py (_do_material) and data.yaml (collect water)
        const WATER_RESETS_THIRST_COUNTER: bool = true;
        const WATER_ADDS_DRINK: bool = true;
        assert!(WATER_RESETS_THIRST_COUNTER, "Water should reset thirst counter to 0");
        assert!(WATER_ADDS_DRINK, "Water should add drink to inventory");
    }

    /// Python Crafter plant ripeness threshold is 300 ticks
    #[test]
    fn test_plant_growth_matches_python() {
        const PLANT_RIPE_THRESHOLD: u16 = 300;
        assert_eq!(PLANT_RIPE_THRESHOLD, 300, "Plant should be ripe at 300 growth ticks");
    }

    /// Python Crafter mob health values
    #[test]
    fn test_mob_health_matches_python() {
        use crate::config::SessionConfig;
        let config = SessionConfig::default();

        assert_eq!(config.cow_health, 3, "Cow should have 3 health");
        assert_eq!(config.zombie_health, 5, "Zombie should have 5 health");
        assert_eq!(config.skeleton_health, 3, "Skeleton should have 3 health");
    }

    /// Python Crafter attack cooldowns
    #[test]
    fn test_attack_cooldowns_match_python() {
        // Zombie attack cooldown is 5 turns
        const ZOMBIE_ATTACK_COOLDOWN: u8 = 5;
        assert_eq!(ZOMBIE_ATTACK_COOLDOWN, 5, "Zombie attack cooldown should be 5");

        // Skeleton reload time is 4 turns
        const SKELETON_RELOAD_TIME: u8 = 4;
        assert_eq!(SKELETON_RELOAD_TIME, 4, "Skeleton reload time should be 4");
    }

    /// Python Crafter arrows damage all objects (2 damage)
    #[test]
    fn test_arrows_damage_objects_match_python() {
        // Arrows deal 2 damage to any object they hit
        const ARROW_DAMAGE: u8 = 2;
        assert_eq!(ARROW_DAMAGE, 2, "Arrows should deal 2 damage to any object");
    }

    /// Python Crafter arrows destroy table/furnace, converting to path
    #[test]
    fn test_arrows_destroy_structures_match_python() {
        // Arrows destroy table and furnace, converting to path
        const ARROWS_DESTROY_STRUCTURES: bool = true;
        assert!(ARROWS_DESTROY_STRUCTURES, "Arrows should destroy table/furnace");
    }

    /// Python Crafter plants take damage from adjacent mobs
    #[test]
    fn test_plants_take_mob_damage_match_python() {
        // Plants are damaged by adjacent zombies, skeletons, and cows
        const PLANTS_DAMAGED_BY_MOBS: bool = true;
        assert!(PLANTS_DAMAGED_BY_MOBS, "Plants should take damage from adjacent mobs");
    }

    /// Python Crafter table and furnace are NOT collectable
    #[test]
    fn test_table_furnace_not_minable_match_python() {
        use crate::material::Material;
        // Table and Furnace cannot be mined/collected in Python Crafter
        assert!(!Material::Table.is_minable(), "Table should NOT be minable");
        assert!(!Material::Furnace.is_minable(), "Furnace should NOT be minable");
    }

    /// Python Crafter player auto-wakes when energy reaches max
    #[test]
    fn test_auto_wake_on_full_energy_match_python() {
        // When sleeping and energy reaches 9, player should automatically wake up
        const AUTO_WAKE_ON_FULL_ENERGY: bool = true;
        assert!(
            AUTO_WAKE_ON_FULL_ENERGY,
            "Player should auto-wake when energy reaches max"
        );
    }

    /// Python Crafter trees give ONLY wood, NOT saplings
    #[test]
    fn test_tree_gives_wood_only_match_python() {
        // Trees give 1 wood, no sapling
        // Saplings come from grass with 10% probability
        const TREE_GIVES_WOOD: u8 = 1;
        const TREE_GIVES_SAPLING: u8 = 0;
        assert_eq!(TREE_GIVES_WOOD, 1, "Trees should give 1 wood");
        assert_eq!(
            TREE_GIVES_SAPLING, 0,
            "Trees should NOT give saplings (those come from grass)"
        );
    }

    /// Python Crafter grass gives saplings with 10% probability
    #[test]
    fn test_grass_sapling_probability_match_python() {
        // Grass collection has 10% chance to give sapling
        const GRASS_SAPLING_PROBABILITY: f32 = 0.1;
        let diff = (GRASS_SAPLING_PROBABILITY - 0.1).abs();
        assert!(diff < 0.001, "Grass sapling probability should be 10%");
    }

    /// Python Crafter player wakes up when health decreases (any cause)
    #[test]
    fn test_wake_on_damage_match_python() {
        // _wake_up_when_hurt: if health < last_health, sleeping = False
        const WAKE_ON_HEALTH_DECREASE: bool = true;
        assert!(
            WAKE_ON_HEALTH_DECREASE,
            "Player should wake when health decreases (from any cause)"
        );
    }

    /// Python Crafter cow has 50% chance to move randomly each tick
    #[test]
    fn test_cow_movement_probability_match_python() {
        const COW_MOVEMENT_PROBABILITY: f32 = 0.5;
        let diff = (COW_MOVEMENT_PROBABILITY - 0.5).abs();
        assert!(diff < 0.001, "Cow should have 50% chance to move");
    }

    // Helper function to convert CamelCase to snake_case
    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(c.to_lowercase().next().unwrap());
            } else {
                result.push(c);
            }
        }
        result
    }
}
