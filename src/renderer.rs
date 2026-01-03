//! Rendering system for different output formats

use crate::entity::GameObject;
use crate::session::GameState;
use crate::world::WorldView;

/// Trait for rendering game state to various formats
pub trait Renderer {
    type Output;
    type Error;

    fn render(&self, state: &GameState) -> Result<Self::Output, Self::Error>;
}

/// Text-based renderer for LLM agents and debugging
pub struct TextRenderer {
    /// Include full inventory details
    pub show_inventory: bool,
    /// Include achievements
    pub show_achievements: bool,
    /// Include surrounding terrain legend
    pub show_legend: bool,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self {
            show_inventory: true,
            show_achievements: true,
            show_legend: true,
        }
    }
}

impl TextRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn minimal() -> Self {
        Self {
            show_inventory: false,
            show_achievements: false,
            show_legend: false,
        }
    }

    /// Render a world view to text
    fn render_view(&self, view: &WorldView) -> String {
        let size = view.size();
        let mut lines = Vec::new();

        // Create object position lookup
        let mut object_chars = std::collections::HashMap::new();
        for (x, y, obj) in &view.objects {
            object_chars.insert((*x, *y), obj.display_char());
        }

        // Render grid
        for y in 0..size {
            let mut line = String::new();
            for x in 0..size {
                let char = if let Some(&ch) = object_chars.get(&(x as i32, y as i32)) {
                    ch
                } else if let Some(mat) = view.get_material(x as i32, y as i32) {
                    mat.display_char()
                } else {
                    ' '
                };
                line.push(char);
            }
            lines.push(line);
        }

        lines.join("\n")
    }
}

impl Renderer for TextRenderer {
    type Output = String;
    type Error = std::convert::Infallible;

    fn render(&self, state: &GameState) -> Result<String, Self::Error> {
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "Step: {} | Episode: {} | Daylight: {:.1}%\n",
            state.step,
            state.episode,
            state.daylight * 100.0
        ));
        output.push_str(&format!(
            "Position: ({}, {}) | Facing: ({}, {}){}\n",
            state.player_pos.0,
            state.player_pos.1,
            state.player_facing.0,
            state.player_facing.1,
            if state.player_sleeping { " [SLEEPING]" } else { "" }
        ));
        output.push('\n');

        // View
        if let Some(ref view) = state.view {
            output.push_str("=== VIEW ===\n");
            output.push_str(&self.render_view(view));
            output.push_str("\n\n");
        }

        // Inventory
        if self.show_inventory {
            output.push_str("=== VITALS ===\n");
            output.push_str(&format!(
                "Health: {} | Food: {} | Drink: {} | Energy: {}\n",
                state.inventory.health,
                state.inventory.food,
                state.inventory.drink,
                state.inventory.energy
            ));

            output.push_str("\n=== RESOURCES ===\n");
            output.push_str(&format!(
                "Wood: {} | Stone: {} | Coal: {} | Iron: {} | Diamond: {} | Sapling: {}\n",
                state.inventory.wood,
                state.inventory.stone,
                state.inventory.coal,
                state.inventory.iron,
                state.inventory.diamond,
                state.inventory.sapling
            ));

            output.push_str("\n=== TOOLS ===\n");
            output.push_str(&format!(
                "Pickaxes: Wood={} Stone={} Iron={}\n",
                state.inventory.wood_pickaxe,
                state.inventory.stone_pickaxe,
                state.inventory.iron_pickaxe
            ));
            output.push_str(&format!(
                "Swords: Wood={} Stone={} Iron={}\n",
                state.inventory.wood_sword,
                state.inventory.stone_sword,
                state.inventory.iron_sword
            ));
            output.push('\n');
        }

        // Achievements
        if self.show_achievements {
            let unlocked = state.achievements.total_unlocked();
            output.push_str(&format!(
                "=== ACHIEVEMENTS ({}/22) ===\n",
                unlocked
            ));

            let ach = &state.achievements;
            if ach.collect_wood > 0 {
                output.push_str(&format!("  collect_wood: {}\n", ach.collect_wood));
            }
            if ach.collect_stone > 0 {
                output.push_str(&format!("  collect_stone: {}\n", ach.collect_stone));
            }
            if ach.collect_coal > 0 {
                output.push_str(&format!("  collect_coal: {}\n", ach.collect_coal));
            }
            if ach.collect_iron > 0 {
                output.push_str(&format!("  collect_iron: {}\n", ach.collect_iron));
            }
            if ach.collect_diamond > 0 {
                output.push_str(&format!("  collect_diamond: {}\n", ach.collect_diamond));
            }
            if ach.collect_sapling > 0 {
                output.push_str(&format!("  collect_sapling: {}\n", ach.collect_sapling));
            }
            if ach.collect_drink > 0 {
                output.push_str(&format!("  collect_drink: {}\n", ach.collect_drink));
            }
            if ach.make_wood_pickaxe > 0 {
                output.push_str(&format!("  make_wood_pickaxe: {}\n", ach.make_wood_pickaxe));
            }
            if ach.make_stone_pickaxe > 0 {
                output.push_str(&format!("  make_stone_pickaxe: {}\n", ach.make_stone_pickaxe));
            }
            if ach.make_iron_pickaxe > 0 {
                output.push_str(&format!("  make_iron_pickaxe: {}\n", ach.make_iron_pickaxe));
            }
            if ach.make_wood_sword > 0 {
                output.push_str(&format!("  make_wood_sword: {}\n", ach.make_wood_sword));
            }
            if ach.make_stone_sword > 0 {
                output.push_str(&format!("  make_stone_sword: {}\n", ach.make_stone_sword));
            }
            if ach.make_iron_sword > 0 {
                output.push_str(&format!("  make_iron_sword: {}\n", ach.make_iron_sword));
            }
            if ach.place_stone > 0 {
                output.push_str(&format!("  place_stone: {}\n", ach.place_stone));
            }
            if ach.place_table > 0 {
                output.push_str(&format!("  place_table: {}\n", ach.place_table));
            }
            if ach.place_furnace > 0 {
                output.push_str(&format!("  place_furnace: {}\n", ach.place_furnace));
            }
            if ach.place_plant > 0 {
                output.push_str(&format!("  place_plant: {}\n", ach.place_plant));
            }
            if ach.defeat_zombie > 0 {
                output.push_str(&format!("  defeat_zombie: {}\n", ach.defeat_zombie));
            }
            if ach.defeat_skeleton > 0 {
                output.push_str(&format!("  defeat_skeleton: {}\n", ach.defeat_skeleton));
            }
            if ach.eat_cow > 0 {
                output.push_str(&format!("  eat_cow: {}\n", ach.eat_cow));
            }
            if ach.eat_plant > 0 {
                output.push_str(&format!("  eat_plant: {}\n", ach.eat_plant));
            }
            if ach.wake_up > 0 {
                output.push_str(&format!("  wake_up: {}\n", ach.wake_up));
            }
            output.push('\n');
        }

        // Legend
        if self.show_legend {
            output.push_str("=== LEGEND ===\n");
            output.push_str("Terrain: ");
            output.push_str(". grass  ~ water  # stone  _ path  : sand  T tree  % lava\n");
            output.push_str("         c coal   i iron   d diamond  t table  f furnace\n");
            output.push_str("Entities: @ player  C cow  Z zombie  S skeleton  * arrow  P/p plant\n");
        }

        Ok(output)
    }
}

/// JSON renderer for structured output
pub struct JsonRenderer;

impl Renderer for JsonRenderer {
    type Output = String;
    type Error = serde_json::Error;

    fn render(&self, state: &GameState) -> Result<String, Self::Error> {
        serde_json::to_string_pretty(state)
    }
}

/// Compact JSON renderer (no pretty printing)
pub struct CompactJsonRenderer;

impl Renderer for CompactJsonRenderer {
    type Output = String;
    type Error = serde_json::Error;

    fn render(&self, state: &GameState) -> Result<String, Self::Error> {
        serde_json::to_string(state)
    }
}

/// Semantic map renderer - produces a grid of material/object indices
pub struct SemanticRenderer {
    /// Size of the output grid (default: same as view)
    pub output_size: Option<(usize, usize)>,
}

impl Default for SemanticRenderer {
    fn default() -> Self {
        Self { output_size: None }
    }
}

impl SemanticRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render to a flat vector of bytes where each byte is a semantic category
    pub fn render_to_bytes(&self, state: &GameState) -> Option<Vec<u8>> {
        let view = state.view.as_ref()?;
        let size = view.size();

        let mut output = vec![0u8; size * size];

        // Create object position lookup
        let mut object_types = std::collections::HashMap::new();
        for (x, y, obj) in &view.objects {
            let type_id = match obj {
                GameObject::Player(_) => 20,
                GameObject::Cow(_) => 21,
                GameObject::Zombie(_) => 22,
                GameObject::Skeleton(_) => 23,
                GameObject::Arrow(_) => 24,
                GameObject::Plant(p) => {
                    if p.is_ripe() {
                        26
                    } else {
                        25
                    }
                }
            };
            object_types.insert((*x, *y), type_id);
        }

        // Fill grid
        for y in 0..size {
            for x in 0..size {
                let idx = y * size + x;
                output[idx] = if let Some(&type_id) = object_types.get(&(x as i32, y as i32)) {
                    type_id
                } else if let Some(mat) = view.get_material(x as i32, y as i32) {
                    mat as u8
                } else {
                    0 // Water/unknown
                };
            }
        }

        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SessionConfig;
    use crate::session::Session;

    #[test]
    fn test_text_renderer() {
        let config = SessionConfig {
            world_size: (32, 32),
            seed: Some(42),
            ..Default::default()
        };

        let session = Session::new(config);
        let state = session.get_state();

        let renderer = TextRenderer::new();
        let output = renderer.render(&state).unwrap();

        assert!(output.contains("Step:"));
        assert!(output.contains("VITALS"));
    }

    #[test]
    fn test_json_renderer() {
        let config = SessionConfig {
            world_size: (32, 32),
            seed: Some(42),
            ..Default::default()
        };

        let session = Session::new(config);
        let state = session.get_state();

        let renderer = JsonRenderer;
        let output = renderer.render(&state).unwrap();

        assert!(output.contains("\"step\""));
        assert!(output.contains("\"inventory\""));
    }
}
