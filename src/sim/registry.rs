use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::types::{BuildingBlueprint, UnitBlueprint};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyConfig {
    pub starting_resources: u32,
    pub harvest_amount_per_trip: u32,
    pub harvest_ticks: u32,
    pub repair_hp_per_tick: f32,
    pub repair_cost_per_tick: u32,
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            starting_resources: 500,
            harvest_amount_per_trip: 10,
            harvest_ticks: 60,
            repair_hp_per_tick: 2.0,
            repair_cost_per_tick: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintRegistry {
    pub units: HashMap<String, UnitBlueprint>,
    pub buildings: HashMap<String, BuildingBlueprint>,
    pub unit_order: Vec<String>,
    pub building_order: Vec<String>,
    pub economy: EconomyConfig,
}

impl BlueprintRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            units: HashMap::new(),
            buildings: HashMap::new(),
            unit_order: Vec::new(),
            building_order: Vec::new(),
            economy: EconomyConfig::default(),
        };
        registry.load_default_blueprints();
        registry
    }

    pub fn register_unit(&mut self, bp: UnitBlueprint) {
        if !self.unit_order.contains(&bp.id) {
            self.unit_order.push(bp.id.clone());
        }
        self.units.insert(bp.id.clone(), bp);
    }

    pub fn register_building(&mut self, bp: BuildingBlueprint) {
        if !self.building_order.contains(&bp.id) {
            self.building_order.push(bp.id.clone());
        }
        self.buildings.insert(bp.id.clone(), bp);
    }

    pub fn get_unit(&self, id: &str) -> Option<&UnitBlueprint> {
        self.units.get(id)
    }

    pub fn get_building(&self, id: &str) -> Option<&BuildingBlueprint> {
        self.buildings.get(id)
    }

    pub fn unit_order(&self) -> &[String] {
        &self.unit_order
    }

    pub fn building_order(&self) -> &[String] {
        &self.building_order
    }

    pub fn units(&self) -> &HashMap<String, UnitBlueprint> {
        &self.units
    }

    pub fn buildings(&self) -> &HashMap<String, BuildingBlueprint> {
        &self.buildings
    }

    pub fn economy(&self) -> &EconomyConfig {
        &self.economy
    }

    pub fn economy_mut(&mut self) -> &mut EconomyConfig {
        &mut self.economy
    }

    pub fn load_default_blueprints(&mut self) {
        self.register_unit(UnitBlueprint {
            id: "worker".to_string(),
            display_name: "Worker Cat".to_string(),
            mesh_archetype: "worker".to_string(),
            scale: 1.0,
            cost: 50,
            supply_cost: 1,
            build_time_ticks: 100,
            max_hp: 60.0,
            armor: 0.0,
            armor_piercing: 0.0,
            move_speed: 3.0,
            vision_range: 16.0,
            attack_damage: 5.0,
            building_damage: 8.0,
            attack_range: 2.0,
            attack_cooldown_ticks: 30,
            projectile_speed: 0.0,
            can_harvest: true,
            can_build: true,
            can_repair: true,
            producer_building: "command_center".to_string(),
            hotkey: "W".to_string(),
        });

        self.register_unit(UnitBlueprint {
            id: "soldier".to_string(),
            display_name: "Soldier Cat".to_string(),
            mesh_archetype: "soldier".to_string(),
            scale: 1.0,
            cost: 100,
            supply_cost: 2,
            build_time_ticks: 90,
            max_hp: 160.0,
            armor: 4.0,
            armor_piercing: 1.0,
            move_speed: 3.8,
            vision_range: 11.0,
            attack_damage: 14.0,
            building_damage: 14.0,
            attack_range: 6.0,
            attack_cooldown_ticks: 14,
            projectile_speed: 24.0,
            can_harvest: false,
            can_build: false,
            can_repair: false,
            producer_building: "barracks".to_string(),
            hotkey: "S".to_string(),
        });

        self.register_unit(UnitBlueprint {
            id: "sniper".to_string(),
            display_name: "Sniper Cat".to_string(),
            mesh_archetype: "sniper".to_string(),
            scale: 0.95,
            cost: 200,
            supply_cost: 2,
            build_time_ticks: 130,
            max_hp: 55.0,
            armor: 0.0,
            armor_piercing: 5.0,
            move_speed: 3.5,
            vision_range: 16.0,
            attack_damage: 42.0,
            building_damage: 4.0,
            attack_range: 14.0,
            attack_cooldown_ticks: 32,
            projectile_speed: 42.0,
            can_harvest: false,
            can_build: false,
            can_repair: false,
            producer_building: "barracks".to_string(),
            hotkey: "N".to_string(),
        });

        self.register_unit(UnitBlueprint {
            id: "heavy".to_string(),
            display_name: "Heavy Weapons Cat".to_string(),
            mesh_archetype: "heavy".to_string(),
            scale: 1.15,
            cost: 300,
            supply_cost: 3,
            build_time_ticks: 150,
            max_hp: 220.0,
            armor: 8.0,
            armor_piercing: 6.0,
            move_speed: 2.9,
            vision_range: 13.0,
            attack_damage: 10.0,
            building_damage: 84.0,
            attack_range: 9.0,
            attack_cooldown_ticks: 38,
            projectile_speed: 22.0,
            can_harvest: false,
            can_build: false,
            can_repair: false,
            producer_building: "barracks".to_string(),
            hotkey: "G".to_string(),
        });

        self.register_building(BuildingBlueprint {
            id: "command_center".to_string(),
            display_name: "Command Center".to_string(),
            mesh_archetype: "command_center".to_string(),
            footprint_radius: 3.5,
            cost: 400,
            build_time_ticks: 600,
            max_hp: 2000.0,
            armor: 10.0,
            vision_range: 30.0,
            supply_provided: 10,
            requires_resource_node: false,
            is_resource_dropoff: true,
            is_mine_extractor: false,
            trainable_units: vec!["worker".to_string()],
            hotkey: "C".to_string(),
            attack_damage: 0.0,
            building_damage: 0.0,
            attack_range: 0.0,
            attack_cooldown_ticks: 0,
            armor_piercing: 0.0,
            projectile_speed: 0.0,
        });

        self.register_building(BuildingBlueprint {
            id: "house".to_string(),
            display_name: "House".to_string(),
            mesh_archetype: "house".to_string(),
            footprint_radius: 2.0,
            cost: 100,
            build_time_ticks: 200,
            max_hp: 700.0,
            armor: 2.0,
            vision_range: 16.0,
            supply_provided: 8,
            requires_resource_node: false,
            is_resource_dropoff: false,
            is_mine_extractor: false,
            trainable_units: vec![],
            hotkey: "H".to_string(),
            attack_damage: 0.0,
            building_damage: 0.0,
            attack_range: 0.0,
            attack_cooldown_ticks: 0,
            armor_piercing: 0.0,
            projectile_speed: 0.0,
        });

        self.register_building(BuildingBlueprint {
            id: "mine".to_string(),
            display_name: "Resource Extractor".to_string(),
            mesh_archetype: "mine".to_string(),
            footprint_radius: 2.2,
            cost: 150,
            build_time_ticks: 250,
            max_hp: 900.0,
            armor: 3.0,
            vision_range: 18.0,
            supply_provided: 0,
            requires_resource_node: true,
            is_resource_dropoff: true,
            is_mine_extractor: true,
            trainable_units: vec![],
            hotkey: "M".to_string(),
            attack_damage: 0.0,
            building_damage: 0.0,
            attack_range: 0.0,
            attack_cooldown_ticks: 0,
            armor_piercing: 0.0,
            projectile_speed: 0.0,
        });

        self.register_building(BuildingBlueprint {
            id: "barracks".to_string(),
            display_name: "Barracks".to_string(),
            mesh_archetype: "barracks".to_string(),
            footprint_radius: 3.0,
            cost: 150,
            build_time_ticks: 350,
            max_hp: 1300.0,
            armor: 3.0,
            vision_range: 20.0,
            supply_provided: 0,
            requires_resource_node: false,
            is_resource_dropoff: false,
            is_mine_extractor: false,
            trainable_units: vec![
                "soldier".to_string(),
                "sniper".to_string(),
                "heavy".to_string(),
            ],
            hotkey: "B".to_string(),
            attack_damage: 0.0,
            building_damage: 0.0,
            attack_range: 0.0,
            attack_cooldown_ticks: 0,
            armor_piercing: 0.0,
            projectile_speed: 0.0,
        });

        self.register_building(BuildingBlueprint {
            id: "turret".to_string(),
            display_name: "Defense Turret".to_string(),
            mesh_archetype: "turret".to_string(),
            footprint_radius: 2.0,
            cost: 1000,
            build_time_ticks: 140,
            max_hp: 1100.0,
            armor: 4.0,
            vision_range: 14.0,
            supply_provided: 0,
            requires_resource_node: false,
            is_resource_dropoff: false,
            is_mine_extractor: false,
            trainable_units: vec![],
            hotkey: "T".to_string(),
            attack_damage: 11.0,
            building_damage: 11.0,
            attack_range: 11.5,
            attack_cooldown_ticks: 18,
            armor_piercing: 2.0,
            projectile_speed: 32.0,
        });
    }
}

impl Default for BlueprintRegistry {
    fn default() -> Self {
        Self::new()
    }
}
