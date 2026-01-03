# crafter-core

A Rust implementation of Crafter - a 2D Minecraft-like survival game engine. Designed for game AI research, reinforcement learning experiments, and as a TUI-embeddable game.

## Features

- **Complete game logic**: Survival mechanics, crafting, combat, day/night cycles
- **Procedural world generation**: Biomes, resources, terrain using noise functions
- **Entity system**: Player, mobs (zombies, skeletons, cows), projectiles
- **Crafting system**: Tools, weapons, furnace smelting
- **Achievement system**: Track player progress
- **Session management**: Save/load game state via JSON serialization
- **TUI rendering**: ASCII/Unicode rendering for terminal interfaces
- **Recording/Playback**: Record and replay game sessions

## Installation

```toml
[dependencies]
crafter-core = "0.1"
```

## Quick Start

```rust
use crafter_core::{Session, Action, GameConfig};

// Create a new game session
let config = GameConfig::default();
let mut session = Session::new(config);

// Game loop
loop {
    // Get current game state
    let state = session.get_state();

    // Perform an action
    session.step(Action::MoveUp);

    // Check if game is over
    if session.is_done() {
        break;
    }
}
```

## Interactive TUI

Run the standalone Crafter TUI from this repo:

```bash
cargo run -p crafter-tui
```

OpenTUI requires Zig 0.14.x on your PATH. If you have the Mission Control repo locally, you can use its bundled Zig:

```bash
PATH=/Users/joshpurtell/Documents/GitHub/mission-control/vendor/zig-0.14.0:$PATH cargo run -p crafter-tui
```

## Modules

- `session` - Game session management and main loop
- `action` - Player actions (movement, combat, crafting)
- `material` - World materials and item types
- `achievement` - Achievement tracking system
- `renderer` - TUI rendering utilities
- `recording` - Session recording and playback
- `config` - Game configuration options

## Use Cases

- **AI Research**: Train reinforcement learning agents in a survival environment
- **Game Development**: Embed as a minigame in larger applications
- **TUI Applications**: Add an interactive game to terminal UIs
- **Education**: Learn game development concepts in Rust

## License

MIT License - see [LICENSE](LICENSE) for details.
