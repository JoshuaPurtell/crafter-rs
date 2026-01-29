use crafter_core_modular as modular;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[test]
fn action_smoke() {
    let actions = modular::Action::all();
    assert!(actions.len() >= 17);
    assert_eq!(modular::Action::from_index(0), Some(modular::Action::Noop));
}

#[test]
fn achievement_smoke() {
    let names = modular::Achievements::all_names();
    assert!(!names.is_empty());
}

#[test]
fn config_defaults_smoke() {
    let config = modular::SessionConfig::default();
    assert_eq!(config.world_size, (64, 64));
    assert!(config.day_night_cycle);
}

#[test]
fn entity_smoke() {
    let zombie = modular::Zombie::new((1, 2));
    let obj = modular::GameObject::Zombie(zombie);
    assert!(obj.is_hostile());
}

#[test]
fn image_renderer_config_smoke() {
    let config = modular::ImageRendererConfig::small();
    assert_eq!(config.tile_size, 7);
}

#[test]
fn inventory_defaults_smoke() {
    let inv = modular::Inventory::new();
    assert_eq!(inv.health, modular::inventory::MAX_INVENTORY_VALUE);
}

#[test]
fn material_properties_smoke() {
    assert!(modular::Material::Grass.is_walkable());
    assert!(modular::Material::Lava.is_deadly());
}

#[test]
fn recording_options_smoke() {
    let options = modular::RecordingOptions::full();
    assert!(options.record_state_before);
    assert!(options.record_state_after);
}

#[test]
fn renderer_smoke() {
    let renderer = modular::renderer::TextRenderer::minimal();
    assert!(!renderer.show_inventory);
    assert!(!renderer.show_achievements);
    assert!(!renderer.show_legend);
}

#[test]
fn rewards_smoke() {
    let config = modular::RewardConfig::default();
    assert!(!config.achievement_rewards.is_empty());
}

#[test]
fn saveload_smoke() {
    let config = modular::SessionConfig {
        seed: Some(42),
        world_size: (16, 16),
        ..Default::default()
    };
    let session = modular::Session::new(config);
    let save = modular::SaveData::from_session(&session, Some("test".to_string()));
    let encoded = serde_json::to_string(&save).expect("serialize save");
    let decoded: modular::SaveData = serde_json::from_str(&encoded).expect("deserialize save");
    assert_eq!(decoded.world.area, save.world.area);
}

#[test]
fn snapshot_smoke() {
    let mut manager = modular::SnapshotManager::new();
    let response = manager.process(modular::SnapshotRequest {
        session_id: None,
        seed: Some(1),
        actions: Vec::new(),
        view_size: Some(9),
        config_name: None,
        config_path: None,
        config_toml: None,
    });
    assert!(!response.map_lines.is_empty());
}

#[test]
fn world_smoke() {
    let world = modular::World::new(8, 8, 123);
    assert_eq!(world.area, (8, 8));
    assert!(world.in_bounds((0, 0)));
}

#[test]
fn worldgen_smoke() {
    let config = modular::SessionConfig {
        seed: Some(1),
        world_size: (8, 8),
        ..Default::default()
    };
    let mut gen = modular::worldgen::WorldGenerator::new(config);
    let world = gen.generate();
    assert_eq!(world.area, (8, 8));
}

#[test]
fn craftax_loot_smoke() {
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    let config = modular::config::CraftaxLootConfig::default();
    let loot = modular::craftax::loot::roll_chest_loot(&mut rng, &config);
    let total = loot.arrows
        + loot.potion_red
        + loot.potion_green
        + loot.potion_blue
        + loot.potion_pink
        + loot.potion_cyan
        + loot.potion_yellow
        + loot.sapphire
        + loot.ruby
        + loot.coal
        + loot.iron
        + loot.diamond;
    assert!(total <= 20);
}

#[test]
fn craftax_mobs_smoke() {
    let stats = modular::craftax::mobs::stats(modular::entity::CraftaxMobKind::OrcMage);
    assert!(stats.is_ranged());
}
