use crafter_core::{Session, SessionConfig, Action};
use crafter_core::material::Material;
use crafter_core::entity::GameObject;

fn main() {
    let config = SessionConfig {
        world_size: (64, 64),
        seed: Some(777),
        ..Default::default()
    };
    let mut session = Session::new(config);

    // Parse all actions from command line
    let args: Vec<String> = std::env::args().skip(1).collect();
    
    // Execute all actions
    for arg in &args {
        let action = match arg.as_str() {
            "u" => Action::MoveUp,
            "d" => Action::MoveDown,
            "l" => Action::MoveLeft,
            "r" => Action::MoveRight,
            "do" => Action::Do,
            "sleep" => Action::Sleep,
            "table" => Action::PlaceTable,
            "sword" => Action::MakeWoodSword,
            "pick" => Action::MakeWoodPickaxe,
            "ssword" => Action::MakeStoneSword,
            "spick" => Action::MakeStonePickaxe,
            _ => continue,
        };
        let result = session.step(action);
        for ach in &result.newly_unlocked {
            println!(">>> {} <<<", ach);
        }
        if result.done {
            println!("GAME OVER: {:?}", result.done_reason);
            return;
        }
    }

    // Show current state
    let state = session.get_state();
    let pos = state.player_pos;
    let inv = &state.inventory;

    println!("\n=== Step {} ===", state.step);
    println!("HP:{} Food:{} Drink:{} Energy:{}", inv.health, inv.food, inv.drink, inv.energy);
    println!("Wood:{} Stone:{} Coal:{} Iron:{}", inv.wood, inv.stone, inv.coal, inv.iron);
    println!("WPick:{} SPick:{} | WSword:{} SSword:{}",
        inv.wood_pickaxe, inv.stone_pickaxe, inv.wood_sword, inv.stone_sword);

    // Show 9x9 map
    for dy in -4..=4i32 {
        let mut row = String::new();
        for dx in -4..=4i32 {
            let check = (pos.0 + dx, pos.1 + dy);
            if dx == 0 && dy == 0 { row.push('@'); continue; }
            let mat = session.world.get_material(check);
            let mob = session.world.objects.values().any(|o| match o {
                GameObject::Cow(c) => c.pos == check,
                GameObject::Zombie(z) => z.pos == check,
                _ => false,
            });
            row.push(if mob { 'M' } else {
                match mat {
                    Some(Material::Grass) => '.',
                    Some(Material::Water) => '~',
                    Some(Material::Stone) => '#',
                    Some(Material::Tree) => 'T',
                    Some(Material::Coal) => 'c',
                    Some(Material::Table) => '+',
                    Some(Material::Sand) => ':',
                    _ => ' ',
                }
            });
        }
        println!("{}", row);
    }
}
