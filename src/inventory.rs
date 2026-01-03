//! Player inventory system

use serde::{Deserialize, Serialize};

/// Maximum value for any inventory slot
pub const MAX_INVENTORY_VALUE: u8 = 9;

/// Helper function to add to a value, capping at max
fn add_capped(slot: &mut u8, amount: u8) {
    *slot = (*slot + amount).min(MAX_INVENTORY_VALUE);
}

/// Player inventory containing vitals, resources, and tools
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    // Vitals (all start at 9, max 9)
    pub health: u8,
    pub food: u8,
    pub drink: u8,
    pub energy: u8,

    // Resources (all start at 0, max 9)
    pub sapling: u8,
    pub wood: u8,
    pub stone: u8,
    pub coal: u8,
    pub iron: u8,
    pub diamond: u8,

    // Tools (all start at 0, max 9)
    pub wood_pickaxe: u8,
    pub stone_pickaxe: u8,
    pub iron_pickaxe: u8,
    pub wood_sword: u8,
    pub stone_sword: u8,
    pub iron_sword: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory {
    /// Create a new inventory with default starting values
    pub fn new() -> Self {
        Self {
            // Vitals start at max
            health: MAX_INVENTORY_VALUE,
            food: MAX_INVENTORY_VALUE,
            drink: MAX_INVENTORY_VALUE,
            energy: MAX_INVENTORY_VALUE,

            // Resources start at 0
            sapling: 0,
            wood: 0,
            stone: 0,
            coal: 0,
            iron: 0,
            diamond: 0,

            // Tools start at 0
            wood_pickaxe: 0,
            stone_pickaxe: 0,
            iron_pickaxe: 0,
            wood_sword: 0,
            stone_sword: 0,
            iron_sword: 0,
        }
    }

    /// Add wood
    pub fn add_wood(&mut self, amount: u8) {
        add_capped(&mut self.wood, amount);
    }

    /// Add stone
    pub fn add_stone(&mut self, amount: u8) {
        add_capped(&mut self.stone, amount);
    }

    /// Add coal
    pub fn add_coal(&mut self, amount: u8) {
        add_capped(&mut self.coal, amount);
    }

    /// Add iron
    pub fn add_iron(&mut self, amount: u8) {
        add_capped(&mut self.iron, amount);
    }

    /// Add diamond
    pub fn add_diamond(&mut self, amount: u8) {
        add_capped(&mut self.diamond, amount);
    }

    /// Add sapling
    pub fn add_sapling(&mut self, amount: u8) {
        add_capped(&mut self.sapling, amount);
    }

    /// Add food
    pub fn add_food(&mut self, amount: u8) {
        add_capped(&mut self.food, amount);
    }

    /// Add drink
    pub fn add_drink(&mut self, amount: u8) {
        add_capped(&mut self.drink, amount);
    }

    /// Add energy
    pub fn add_energy(&mut self, amount: u8) {
        add_capped(&mut self.energy, amount);
    }

    /// Add health
    pub fn add_health(&mut self, amount: u8) {
        add_capped(&mut self.health, amount);
    }

    /// Take damage, returns true if still alive
    pub fn take_damage(&mut self, amount: u8) -> bool {
        if self.health > amount {
            self.health -= amount;
            true
        } else {
            self.health = 0;
            false
        }
    }

    /// Check if player is alive
    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    /// Get the best pickaxe tier (0 = none, 1 = wood, 2 = stone, 3 = iron)
    pub fn best_pickaxe_tier(&self) -> u8 {
        if self.iron_pickaxe > 0 {
            3
        } else if self.stone_pickaxe > 0 {
            2
        } else if self.wood_pickaxe > 0 {
            1
        } else {
            0
        }
    }

    /// Get the best sword tier (0 = none, 1 = wood, 2 = stone, 3 = iron)
    pub fn best_sword_tier(&self) -> u8 {
        if self.iron_sword > 0 {
            3
        } else if self.stone_sword > 0 {
            2
        } else if self.wood_sword > 0 {
            1
        } else {
            0
        }
    }

    /// Get damage dealt by player based on sword
    /// Python Crafter values: unarmed=1, wood=2, stone=3, iron=5
    pub fn attack_damage(&self) -> u8 {
        match self.best_sword_tier() {
            3 => 5, // Iron sword
            2 => 3, // Stone sword
            1 => 2, // Wood sword
            _ => 1, // Bare hands
        }
    }

    /// Check if player can craft wood pickaxe (needs table nearby, 1 wood)
    pub fn can_craft_wood_pickaxe(&self) -> bool {
        self.wood >= 1
    }

    /// Check if player can craft stone pickaxe (needs table nearby, 1 wood, 1 stone)
    pub fn can_craft_stone_pickaxe(&self) -> bool {
        self.wood >= 1 && self.stone >= 1
    }

    /// Check if player can craft iron pickaxe (needs table+furnace nearby, 1 wood, 1 coal, 1 iron)
    pub fn can_craft_iron_pickaxe(&self) -> bool {
        self.wood >= 1 && self.coal >= 1 && self.iron >= 1
    }

    /// Check if player can craft wood sword (needs table nearby, 1 wood)
    pub fn can_craft_wood_sword(&self) -> bool {
        self.wood >= 1
    }

    /// Check if player can craft stone sword (needs table nearby, 1 wood, 1 stone)
    pub fn can_craft_stone_sword(&self) -> bool {
        self.wood >= 1 && self.stone >= 1
    }

    /// Check if player can craft iron sword (needs table+furnace nearby, 1 wood, 1 coal, 1 iron)
    pub fn can_craft_iron_sword(&self) -> bool {
        self.wood >= 1 && self.coal >= 1 && self.iron >= 1
    }

    /// Consume materials for wood pickaxe
    pub fn craft_wood_pickaxe(&mut self) -> bool {
        if self.can_craft_wood_pickaxe() {
            self.wood -= 1;
            add_capped(&mut self.wood_pickaxe, 1);
            true
        } else {
            false
        }
    }

    /// Consume materials for stone pickaxe
    pub fn craft_stone_pickaxe(&mut self) -> bool {
        if self.can_craft_stone_pickaxe() {
            self.wood -= 1;
            self.stone -= 1;
            add_capped(&mut self.stone_pickaxe, 1);
            true
        } else {
            false
        }
    }

    /// Consume materials for iron pickaxe
    pub fn craft_iron_pickaxe(&mut self) -> bool {
        if self.can_craft_iron_pickaxe() {
            self.wood -= 1;
            self.coal -= 1;
            self.iron -= 1;
            add_capped(&mut self.iron_pickaxe, 1);
            true
        } else {
            false
        }
    }

    /// Consume materials for wood sword
    pub fn craft_wood_sword(&mut self) -> bool {
        if self.can_craft_wood_sword() {
            self.wood -= 1;
            add_capped(&mut self.wood_sword, 1);
            true
        } else {
            false
        }
    }

    /// Consume materials for stone sword
    pub fn craft_stone_sword(&mut self) -> bool {
        if self.can_craft_stone_sword() {
            self.wood -= 1;
            self.stone -= 1;
            add_capped(&mut self.stone_sword, 1);
            true
        } else {
            false
        }
    }

    /// Consume materials for iron sword
    pub fn craft_iron_sword(&mut self) -> bool {
        if self.can_craft_iron_sword() {
            self.wood -= 1;
            self.coal -= 1;
            self.iron -= 1;
            add_capped(&mut self.iron_sword, 1);
            true
        } else {
            false
        }
    }

    /// Check if has stone to place
    pub fn has_stone(&self) -> bool {
        self.stone > 0
    }

    /// Check if has wood for table
    pub fn has_wood_for_table(&self) -> bool {
        self.wood >= 2
    }

    /// Check if has stone for furnace
    pub fn has_stone_for_furnace(&self) -> bool {
        self.stone >= 4
    }

    /// Check if has sapling to plant
    pub fn has_sapling(&self) -> bool {
        self.sapling > 0
    }

    /// Use stone for placement
    pub fn use_stone(&mut self) -> bool {
        if self.stone > 0 {
            self.stone -= 1;
            true
        } else {
            false
        }
    }

    /// Use wood for table
    pub fn use_wood_for_table(&mut self) -> bool {
        if self.wood >= 2 {
            self.wood -= 2;
            true
        } else {
            false
        }
    }

    /// Use stone for furnace
    pub fn use_stone_for_furnace(&mut self) -> bool {
        if self.stone >= 4 {
            self.stone -= 4;
            true
        } else {
            false
        }
    }

    /// Use sapling for planting
    pub fn use_sapling(&mut self) -> bool {
        if self.sapling > 0 {
            self.sapling -= 1;
            true
        } else {
            false
        }
    }
}
