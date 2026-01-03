use crafter_core::{Action, Session, SessionConfig};
use crafter_core::material::Material;
use crafter_core::entity::GameObject;

fn main() {
    let config = SessionConfig {
        world_size: (64, 64),
        seed: Some(777),
        ..Default::default()
    };
    let mut session = Session::new(config);

    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let repl_mode = if let Some(pos) = args.iter().position(|arg| arg == "--repl") {
        args.remove(pos);
        true
    } else {
        false
    };

    // Execute all actions
    for arg in &args {
        if let Some(action) = parse_action(arg) {
            if step_action(&mut session, action) {
                return;
            }
        }
    }

    if repl_mode {
        run_repl(&mut session);
    } else {
        print_state(&session);
    }
}

fn run_repl(session: &mut Session) {
    println!("Crafter headless REPL");
    println!("Actions: u d l r do sleep table stone furnace plant pick sword spick ssword ipick isword");
    println!("Commands: state, help, q");

    let mut line = String::new();
    loop {
        line.clear();
        if std::io::stdin().read_line(&mut line).is_err() {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            print_state(session);
            continue;
        }
        if trimmed == "q" || trimmed == "quit" {
            break;
        }
        if trimmed == "help" {
            println!("Actions: u d l r do sleep table stone furnace plant pick sword spick ssword ipick isword");
            println!("Commands: state, help, q");
            continue;
        }
        if trimmed == "state" {
            print_state(session);
            continue;
        }

        for token in trimmed.split_whitespace() {
            if let Some(action) = parse_action(token) {
                if step_action(session, action) {
                    return;
                }
            } else {
                println!("Unknown token: {}", token);
            }
        }
    }
}

fn parse_action(token: &str) -> Option<Action> {
    match token {
        "u" => Some(Action::MoveUp),
        "d" => Some(Action::MoveDown),
        "l" => Some(Action::MoveLeft),
        "r" => Some(Action::MoveRight),
        "do" => Some(Action::Do),
        "sleep" => Some(Action::Sleep),
        "table" => Some(Action::PlaceTable),
        "stone" => Some(Action::PlaceStone),
        "furnace" => Some(Action::PlaceFurnace),
        "plant" => Some(Action::PlacePlant),
        "sword" => Some(Action::MakeWoodSword),
        "pick" => Some(Action::MakeWoodPickaxe),
        "ssword" => Some(Action::MakeStoneSword),
        "spick" => Some(Action::MakeStonePickaxe),
        "isword" => Some(Action::MakeIronSword),
        "ipick" => Some(Action::MakeIronPickaxe),
        _ => None,
    }
}

fn step_action(session: &mut Session, action: Action) -> bool {
    let result = session.step(action);
    for ach in &result.newly_unlocked {
        println!(">>> {} <<<", ach);
    }
    if result.done {
        println!("GAME OVER: {:?}", result.done_reason);
        print_state(session);
        return true;
    }
    false
}

fn print_state(session: &Session) {
    let state = session.get_state();
    let pos = state.player_pos;
    let inv = &state.inventory;

    println!("\n=== Step {} ===", state.step);
    println!("HP:{} Food:{} Drink:{} Energy:{}", inv.health, inv.food, inv.drink, inv.energy);
    println!("Wood:{} Stone:{} Coal:{} Iron:{}", inv.wood, inv.stone, inv.coal, inv.iron);
    println!(
        "WPick:{} SPick:{} | WSword:{} SSword:{}",
        inv.wood_pickaxe, inv.stone_pickaxe, inv.wood_sword, inv.stone_sword
    );

    // Show 9x9 map
    for dy in -4..=4i32 {
        let mut row = String::new();
        for dx in -4..=4i32 {
            let check = (pos.0 + dx, pos.1 + dy);
            if dx == 0 && dy == 0 {
                row.push('@');
                continue;
            }
            let mat = session.world.get_material(check);
            let mob = session.world.objects.values().any(|o| match o {
                GameObject::Cow(c) => c.pos == check,
                GameObject::Zombie(z) => z.pos == check,
                GameObject::Skeleton(s) => s.pos == check,
                _ => false,
            });
            row.push(if mob {
                'M'
            } else {
                match mat {
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
                    Some(Material::Sapphire) => 'S',
                    Some(Material::Ruby) => 'R',
                    Some(Material::Chest) => 'H',
                    None => ' ',
                }
            });
        }
        println!("{}", row);
    }
}
