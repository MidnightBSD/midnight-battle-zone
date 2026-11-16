use std::collections::HashMap;
use std::fs;
use std::path::Path;

use mlua::{Lua, Table, Value};

use crate::sim::registry::BlueprintRegistry;
use crate::sim::world::SimWorld;
use crate::types::{
    AiDifficulty, BuildingBlueprint, GameCommand, MapDefinition, ObstacleSpawn,
    ResourceNodeSpawn, TeamId, UnitBlueprint, UnitOrderState,
};

pub const FALLBACK_UNITS_LUA: &str = r#"
units = {
    worker = {
        id = "worker",
        display_name = "Worker Cat",
        mesh_archetype = "worker",
        scale = 1.0,
        cost = 50,
        supply_cost = 1,
        build_time_ticks = 100,
        max_hp = 60.0,
        armor = 0.0,
        armor_piercing = 0.0,
        move_speed = 3.0,
        vision_range = 16.0,
        attack_damage = 5.0,
        attack_range = 2.0,
        attack_cooldown_ticks = 30,
        projectile_speed = 0.0,
        can_harvest = true,
        can_build = true,
        can_repair = true,
        producer_building = "command_center",
        hotkey = "W",
    },
    soldier = {
        id = "soldier",
        display_name = "Soldier Cat",
        mesh_archetype = "soldier",
        scale = 1.0,
        cost = 100,
        supply_cost = 2,
        build_time_ticks = 90,
        max_hp = 160.0,
        armor = 4.0,
        armor_piercing = 1.0,
        move_speed = 3.8,
        vision_range = 11.0,
        attack_damage = 14.0,
        attack_range = 6.0,
        attack_cooldown_ticks = 14,
        projectile_speed = 24.0,
        can_harvest = false,
        can_build = false,
        can_repair = false,
        producer_building = "barracks",
        hotkey = "S",
    },
    sniper = {
        id = "sniper",
        display_name = "Sniper Cat",
        mesh_archetype = "sniper",
        scale = 0.95,
        cost = 200,
        supply_cost = 2,
        build_time_ticks = 130,
        max_hp = 55.0,
        armor = 0.0,
        armor_piercing = 5.0,
        move_speed = 3.5,
        vision_range = 16.0,
        attack_damage = 42.0,
        attack_range = 14.0,
        attack_cooldown_ticks = 32,
        projectile_speed = 42.0,
        can_harvest = false,
        can_build = false,
        can_repair = false,
        producer_building = "barracks",
        hotkey = "N",
    },
}
return units
"#;

pub const FALLBACK_BUILDINGS_LUA: &str = r#"
buildings = {
    command_center = {
        id = "command_center",
        display_name = "Command Center",
        mesh_archetype = "command_center",
        footprint_radius = 3.5,
        cost = 400,
        build_time_ticks = 600,
        max_hp = 2000.0,
        armor = 5.0,
        vision_range = 30.0,
        supply_provided = 10,
        requires_resource_node = false,
        is_resource_dropoff = true,
        is_mine_extractor = false,
        trainable_units = { "worker" },
        hotkey = "C",
    },
    house = {
        id = "house",
        display_name = "House",
        mesh_archetype = "house",
        footprint_radius = 2.0,
        cost = 100,
        build_time_ticks = 200,
        max_hp = 700.0,
        armor = 2.0,
        vision_range = 16.0,
        supply_provided = 8,
        requires_resource_node = false,
        is_resource_dropoff = false,
        is_mine_extractor = false,
        trainable_units = {},
        hotkey = "H",
    },
    mine = {
        id = "mine",
        display_name = "Resource Extractor",
        mesh_archetype = "mine",
        footprint_radius = 2.2,
        cost = 150,
        build_time_ticks = 250,
        max_hp = 900.0,
        armor = 3.0,
        vision_range = 18.0,
        supply_provided = 0,
        requires_resource_node = true,
        is_resource_dropoff = true,
        is_mine_extractor = true,
        trainable_units = {},
        hotkey = "M",
    },
    barracks = {
        id = "barracks",
        display_name = "Barracks",
        mesh_archetype = "barracks",
        footprint_radius = 3.0,
        cost = 150,
        build_time_ticks = 350,
        max_hp = 1300.0,
        armor = 3.0,
        vision_range = 20.0,
        supply_provided = 0,
        requires_resource_node = false,
        is_resource_dropoff = false,
        is_mine_extractor = false,
        trainable_units = { "soldier", "sniper" },
        hotkey = "B",
    },
    turret = {
        id = "turret",
        display_name = "Defense Turret",
        mesh_archetype = "turret",
        footprint_radius = 2.0,
        cost = 1000,
        build_time_ticks = 140,
        max_hp = 1100.0,
        armor = 4.0,
        armor_piercing = 2.0,
        vision_range = 14.0,
        supply_provided = 0,
        requires_resource_node = false,
        is_resource_dropoff = false,
        is_mine_extractor = false,
        attack_damage = 11.0,
        attack_range = 11.5,
        attack_cooldown_ticks = 18,
        projectile_speed = 32.0,
        trainable_units = {},
        hotkey = "T",
    },
}
return buildings
"#;

pub const FALLBACK_GAMEPLAY_LUA: &str = r#"
function calculate_damage(base_damage, armor, armor_piercing)
    local effective_armor = math.max(0.0, armor - armor_piercing)
    local damage = math.max(1.0, base_damage - effective_armor)
    return damage
end
"#;

pub const FALLBACK_AI_LUA: &str = r#"
function compute_ai_commands(state)
    local commands = {}
    local team = state.team
    local resources = state.resources
    local supply_used = state.supply_used
    local supply_cap = state.supply_cap
    local difficulty = state.difficulty or 2

    local my_workers = {}
    local my_combat_units = {}
    local idle_workers = {}

    for _, u in ipairs(state.my_units) do
        if u.unit_type == "worker" then
            table.insert(my_workers, u)
            if u.order == "idle" then
                table.insert(idle_workers, u)
            end
        else
            table.insert(my_combat_units, u)
        end
    end

    local command_center = nil
    local barracks_list = {}
    local houses = {}
    local mines = {}

    for _, b in ipairs(state.my_buildings) do
        if b.building_type == "command_center" then
            command_center = b
        elseif b.building_type == "barracks" then
            table.insert(barracks_list, b)
        elseif b.building_type == "house" then
            table.insert(houses, b)
        elseif b.building_type == "mine" then
            table.insert(mines, b)
        end
    end

    -- 1. Supply check: construct House if near cap
    if supply_used + 2 >= supply_cap and resources >= 100 and #idle_workers > 0 and #houses < 6 then
        local w = table.remove(idle_workers)
        local base_x = command_center and command_center.x or w.x
        local base_y = command_center and command_center.y or w.y
        local offset_x = (#houses + 1) * 6.0
        local offset_y = 6.0
        if team == 1 then
            offset_x = -offset_x
            offset_y = -offset_y
        end
        table.insert(commands, {
            action = "construct_building",
            worker_ids = { w.id },
            building_type = "house",
            position = { base_x + offset_x, base_y + offset_y }
        })
        resources = resources - 100
    end

    -- 2. Build or harvest Mine
    for _, node in ipairs(state.resource_nodes) do
        if not node.has_mine and resources >= 150 and #idle_workers > 0 then
            local w = table.remove(idle_workers)
            table.insert(commands, {
                action = "harvest_mine",
                worker_ids = { w.id },
                target_id = node.id
            })
            resources = resources - 150
            break
        end
    end

    -- 3. Assign idle workers to harvest
    if #mines > 0 then
        for _, w in ipairs(idle_workers) do
            table.insert(commands, {
                action = "harvest_mine",
                worker_ids = { w.id },
                target_id = mines[1].id
            })
        end
    end

    -- 4. Build Barracks
    local max_barracks = (difficulty >= 3) and 2 or 1
    if #barracks_list < max_barracks and resources >= 150 and #my_workers > 0 then
        local builder = my_workers[1]
        local base_x = command_center and command_center.x or builder.x
        local base_y = command_center and command_center.y or builder.y
        local offset_x = 8.0 * (#barracks_list + 1)
        local offset_y = -8.0
        if team == 1 then
            offset_x = -offset_x
            offset_y = -offset_y
        end
        table.insert(commands, {
            action = "construct_building",
            worker_ids = { builder.id },
            building_type = "barracks",
            position = { base_x + offset_x, base_y + offset_y }
        })
        resources = resources - 150
    end

    -- 5. Train workers
    local target_workers = (difficulty >= 2) and 6 or 4
    if command_center and command_center.completed and #my_workers < target_workers and resources >= 50 and supply_used < supply_cap then
        if command_center.queue_length == 0 then
            table.insert(commands, {
                action = "train_unit",
                building_id = command_center.id,
                unit_type = "worker"
            })
            resources = resources - 50
            supply_used = supply_used + 1
        end
    end

    -- 6. Train combat units
    for _, bar in ipairs(barracks_list) do
        if bar.completed and bar.queue_length == 0 and supply_used < supply_cap then
            local u_type = "soldier"
            local cost = 100
            local s_cost = 1
            if difficulty >= 2 and (#my_combat_units % 2 == 1) and resources >= 200 and supply_used + 2 <= supply_cap then
                u_type = "sniper"
                cost = 200
                s_cost = 2
            end
            if resources >= cost and supply_used + s_cost <= supply_cap then
                table.insert(commands, {
                    action = "train_unit",
                    building_id = bar.id,
                    unit_type = u_type
                })
                resources = resources - cost
                supply_used = supply_used + s_cost
            end
        end
    end

    -- 7. Combat unit tactics
    if #my_combat_units > 0 then
        local army_ids = {}
        for _, u in ipairs(my_combat_units) do
            table.insert(army_ids, u.id)
        end

        local attack_threshold = (difficulty == 1) and 6 or ((difficulty == 2) and 4 or 3)

        if #state.visible_enemies > 0 then
            local target_enemy = state.visible_enemies[1]
            table.insert(commands, {
                action = "attack_target",
                unit_ids = army_ids,
                target_id = target_enemy.id
            })
        elseif #my_combat_units >= attack_threshold then
            local enemy_target_x = (team == 0) and 108.0 or 20.0
            local enemy_target_y = (team == 0) and 108.0 or 20.0
            table.insert(commands, {
                action = "move_units",
                unit_ids = army_ids,
                target = { enemy_target_x, enemy_target_y },
                attack_move = true
            })
        end
    end

    return commands
end
"#;

pub const FALLBACK_MAP_CROSSFIRE_LUA: &str = r#"
map = {
    id = "crossfire",
    name = "Crossfire Valley",
    description = "A symmetrical 128x128 valley with rich mineral deposits in the center.",
    width = 128.0,
    height = 128.0,
    team_spawns = {
        { 20.0, 20.0 },
        { 108.0, 108.0 },
    },
    resource_nodes = {
        { position = { 24.0, 16.0 }, amount = 1500, side_hint = 0 },
        { position = { 16.0, 26.0 }, amount = 1500, side_hint = 0 },
        { position = { 38.0, 28.0 }, amount = 2000, side_hint = 0 },
        { position = { 104.0, 112.0 }, amount = 1500, side_hint = 1 },
        { position = { 112.0, 102.0 }, amount = 1500, side_hint = 1 },
        { position = { 90.0, 100.0 }, amount = 2000, side_hint = 1 },
        { position = { 64.0, 64.0 }, amount = 3000, side_hint = 255 },
    },
    obstacles = {
        { position = { 64.0, 32.0 }, radius = 6.0, height = 4.0 },
        { position = { 64.0, 96.0 }, radius = 6.0, height = 4.0 },
        { position = { 32.0, 64.0 }, radius = 6.0, height = 4.0 },
        { position = { 96.0, 64.0 }, radius = 6.0, height = 4.0 },
    },
}
return map
"#;

pub const FALLBACK_MAP_MIDNIGHT_RIDGE_LUA: &str = r#"
map = {
    id = "midnight_ridge",
    name = "Midnight Ridge",
    description = "A rugged 160x160 mountain pass with tight choke points and high ground advantages.",
    width = 160.0,
    height = 160.0,
    team_spawns = {
        { 24.0, 24.0 },
        { 136.0, 136.0 },
    },
    resource_nodes = {
        { position = { 28.0, 18.0 }, amount = 1600, side_hint = 0 },
        { position = { 18.0, 28.0 }, amount = 1600, side_hint = 0 },
        { position = { 46.0, 36.0 }, amount = 2200, side_hint = 0 },
        { position = { 132.0, 142.0 }, amount = 1600, side_hint = 1 },
        { position = { 142.0, 132.0 }, amount = 1600, side_hint = 1 },
        { position = { 114.0, 124.0 }, amount = 2200, side_hint = 1 },
        { position = { 70.0, 90.0 }, amount = 2500, side_hint = 255 },
        { position = { 90.0, 70.0 }, amount = 2500, side_hint = 255 },
    },
    obstacles = {
        { position = { 80.0, 80.0 }, radius = 8.0, height = 6.0 },
        { position = { 50.0, 75.0 }, radius = 5.0, height = 4.0 },
        { position = { 110.0, 85.0 }, radius = 5.0, height = 4.0 },
        { position = { 75.0, 45.0 }, radius = 5.0, height = 4.0 },
        { position = { 85.0, 115.0 }, radius = 5.0, height = 4.0 },
    },
}
return map
"#;

pub struct LuaScriptingEngine {
    pub lua: Lua,
    pub maps: HashMap<String, MapDefinition>,
    pub map_order: Vec<String>,
}

impl LuaScriptingEngine {
    pub fn new() -> Result<Self, mlua::Error> {
        let lua = Lua::new();
        let mut engine = Self {
            lua,
            maps: HashMap::new(),
            map_order: Vec::new(),
        };

        let mut default_registry = BlueprintRegistry::new();
        engine.load_scripts(&mut default_registry)?;
        Ok(engine)
    }

    pub fn load_scripts(&mut self, registry: &mut BlueprintRegistry) -> Result<(), mlua::Error> {
        // 1. units.lua
        let units_code = fs::read_to_string("assets/scripts/units.lua")
            .unwrap_or_else(|_| FALLBACK_UNITS_LUA.to_string());
        let units_val: Value = self.lua.load(&units_code).eval()?;
        let units_table = match units_val {
            Value::Table(t) => t,
            _ => self.lua.globals().get::<Table>("units")?,
        };
        self.parse_units_table(&units_table, registry)?;

        // 2. buildings.lua
        let buildings_code = fs::read_to_string("assets/scripts/buildings.lua")
            .unwrap_or_else(|_| FALLBACK_BUILDINGS_LUA.to_string());
        let buildings_val: Value = self.lua.load(&buildings_code).eval()?;
        let buildings_table = match buildings_val {
            Value::Table(t) => t,
            _ => self.lua.globals().get::<Table>("buildings")?,
        };
        self.parse_buildings_table(&buildings_table, registry)?;

        // 3. gameplay.lua
        let gameplay_code = fs::read_to_string("assets/scripts/gameplay.lua")
            .unwrap_or_else(|_| FALLBACK_GAMEPLAY_LUA.to_string());
        self.lua.load(&gameplay_code).exec()?;

        // If gameplay.lua defines get_economy_config(), update registry
        if let Ok(func) = self.lua.globals().get::<mlua::Function>("get_economy_config") {
            if let Ok(econ_tab) = func.call::<Table>(()) {
                if let Ok(sr) = econ_tab.get("starting_resources") {
                    registry.economy.starting_resources = sr;
                }
                if let Ok(amt) = econ_tab.get("harvest_amount_per_trip") {
                    registry.economy.harvest_amount_per_trip = amt;
                }
                if let Ok(ht) = econ_tab.get("harvest_ticks") {
                    registry.economy.harvest_ticks = ht;
                }
                if let Ok(rhp) = econ_tab.get("repair_hp_per_tick") {
                    registry.economy.repair_hp_per_tick = rhp;
                }
                if let Ok(rcp) = econ_tab.get("repair_cost_per_tick") {
                    registry.economy.repair_cost_per_tick = rcp;
                }
            }
        }

        // 4. ai.lua
        let ai_code = fs::read_to_string("assets/scripts/ai.lua")
            .unwrap_or_else(|_| FALLBACK_AI_LUA.to_string());
        self.lua.load(&ai_code).exec()?;

        // 5. Maps in assets/maps/*.lua (or fallbacks)
        self.load_all_maps()?;

        Ok(())
    }

    fn parse_units_table(&self, table: &Table, registry: &mut BlueprintRegistry) -> Result<(), mlua::Error> {
        for pair in table.pairs::<Value, Value>() {
            let (_, val) = pair?;
            let u_tab = match val {
                Value::Table(t) => t,
                _ => continue,
            };
            let id: String = match u_tab.get("id") {
                Ok(s) => s,
                Err(_) => continue,
            };
            let display_name: String = u_tab.get("display_name").unwrap_or_else(|_| id.clone());
            let mesh_archetype: String = u_tab.get("mesh_archetype").unwrap_or_else(|_| id.clone());
            let scale: f32 = u_tab.get("scale").unwrap_or(1.0);
            let cost: u32 = u_tab.get("cost").unwrap_or(50);
            let supply_cost: u32 = u_tab.get("supply_cost").unwrap_or(1);
            let build_time_ticks: u32 = u_tab.get("build_time_ticks").unwrap_or(100);
            let max_hp: f32 = u_tab.get("max_hp").unwrap_or(100.0);
            let armor: f32 = u_tab.get("armor").unwrap_or(0.0);
            let armor_piercing: f32 = u_tab.get("armor_piercing").unwrap_or(0.0);
            let move_speed: f32 = u_tab.get("move_speed").unwrap_or(2.5);
            let vision_range: f32 = u_tab.get("vision_range").unwrap_or(16.0);
            let attack_damage: f32 = u_tab.get("attack_damage").unwrap_or(10.0);
            let attack_range: f32 = u_tab.get("attack_range").unwrap_or(2.0);
            let attack_cooldown_ticks: u32 = u_tab.get("attack_cooldown_ticks").unwrap_or(30);
            let projectile_speed: f32 = u_tab.get("projectile_speed").unwrap_or(0.0);
            let can_harvest: bool = u_tab.get("can_harvest").unwrap_or(false);
            let can_build: bool = u_tab.get("can_build").unwrap_or(false);
            let can_repair: bool = u_tab.get("can_repair").unwrap_or(false);
            let producer_building: String = u_tab.get("producer_building").unwrap_or_default();
            let hotkey: String = u_tab.get("hotkey").unwrap_or_default();

            registry.register_unit(UnitBlueprint {
                id,
                display_name,
                mesh_archetype,
                scale,
                cost,
                supply_cost,
                build_time_ticks,
                max_hp,
                armor,
                armor_piercing,
                move_speed,
                vision_range,
                attack_damage,
                attack_range,
                attack_cooldown_ticks,
                projectile_speed,
                can_harvest,
                can_build,
                can_repair,
                producer_building,
                hotkey,
            });
        }
        Ok(())
    }

    fn parse_buildings_table(&self, table: &Table, registry: &mut BlueprintRegistry) -> Result<(), mlua::Error> {
        for pair in table.pairs::<Value, Value>() {
            let (_, val) = pair?;
            let b_tab = match val {
                Value::Table(t) => t,
                _ => continue,
            };
            let id: String = match b_tab.get("id") {
                Ok(s) => s,
                Err(_) => continue,
            };
            let display_name: String = b_tab.get("display_name").unwrap_or_else(|_| id.clone());
            let mesh_archetype: String = b_tab.get("mesh_archetype").unwrap_or_else(|_| id.clone());
            let footprint_radius: f32 = b_tab.get("footprint_radius").unwrap_or(2.5);
            let cost: u32 = b_tab.get("cost").unwrap_or(100);
            let build_time_ticks: u32 = b_tab.get("build_time_ticks").unwrap_or(200);
            let max_hp: f32 = b_tab.get("max_hp").unwrap_or(500.0);
            let armor: f32 = b_tab.get("armor").unwrap_or(2.0);
            let vision_range: f32 = b_tab.get("vision_range").unwrap_or(20.0);
            let supply_provided: u32 = b_tab.get("supply_provided").unwrap_or(0);
            let requires_resource_node: bool = b_tab.get("requires_resource_node").unwrap_or(false);
            let is_resource_dropoff: bool = b_tab.get("is_resource_dropoff").unwrap_or(false);
            let is_mine_extractor: bool = b_tab.get("is_mine_extractor").unwrap_or(false);
            let hotkey: String = b_tab.get("hotkey").unwrap_or_default();
            let attack_damage: f32 = b_tab.get("attack_damage").unwrap_or(0.0);
            let attack_range: f32 = b_tab.get("attack_range").unwrap_or(0.0);
            let attack_cooldown_ticks: u32 = b_tab.get("attack_cooldown_ticks").unwrap_or(20);
            let armor_piercing: f32 = b_tab.get("armor_piercing").unwrap_or(0.0);
            let projectile_speed: f32 = b_tab.get("projectile_speed").unwrap_or(28.0);

            let mut trainable_units = Vec::new();
            if let Ok(t_tab) = b_tab.get::<Table>("trainable_units") {
                for u_str in t_tab.sequence_values::<String>() {
                    if let Ok(u) = u_str {
                        trainable_units.push(u);
                    }
                }
            }

            registry.register_building(BuildingBlueprint {
                id,
                display_name,
                mesh_archetype,
                footprint_radius,
                cost,
                build_time_ticks,
                max_hp,
                armor,
                vision_range,
                supply_provided,
                requires_resource_node,
                is_resource_dropoff,
                is_mine_extractor,
                trainable_units,
                hotkey,
                attack_damage,
                attack_range,
                attack_cooldown_ticks,
                armor_piercing,
                projectile_speed,
            });
        }
        Ok(())
    }

    fn parse_map_table(&self, table: &Table) -> Result<MapDefinition, mlua::Error> {
        let id: String = table.get("id")?;
        let name: String = table.get("name").unwrap_or_else(|_| id.clone());
        let description: String = table.get("description").unwrap_or_default();
        let width: f32 = table.get("width").unwrap_or(128.0);
        let height: f32 = table.get("height").unwrap_or(128.0);

        let mut team_spawns = [[20.0, 20.0], [width - 20.0, height - 20.0]];
        if let Ok(spawns_tab) = table.get::<Table>("team_spawns") {
            for (idx, spawn_val) in spawns_tab.sequence_values::<Table>().enumerate() {
                if idx < 2 {
                    if let Ok(st) = spawn_val {
                        let x: f32 = st.get(1).unwrap_or(20.0);
                        let y: f32 = st.get(2).unwrap_or(20.0);
                        team_spawns[idx] = [x, y];
                    }
                }
            }
        }

        let mut resource_nodes = Vec::new();
        if let Ok(nodes_tab) = table.get::<Table>("resource_nodes") {
            for node_val in nodes_tab.sequence_values::<Table>() {
                if let Ok(nt) = node_val {
                    let mut pos = [width * 0.5, height * 0.5];
                    if let Ok(pos_tab) = nt.get::<Table>("position") {
                        let px: f32 = pos_tab.get(1).unwrap_or(pos[0]);
                        let py: f32 = pos_tab.get(2).unwrap_or(pos[1]);
                        pos = [px, py];
                    }
                    let amount: u32 = nt.get("amount").unwrap_or(1500);
                    let side_hint: TeamId = nt.get("side_hint").unwrap_or(255);
                    resource_nodes.push(ResourceNodeSpawn {
                        position: pos,
                        amount,
                        side_hint,
                    });
                }
            }
        }

        let mut obstacles = Vec::new();
        if let Ok(obs_tab) = table.get::<Table>("obstacles") {
            for obs_val in obs_tab.sequence_values::<Table>() {
                if let Ok(ot) = obs_val {
                    let mut pos = [width * 0.5, height * 0.5];
                    if let Ok(pos_tab) = ot.get::<Table>("position") {
                        let px: f32 = pos_tab.get(1).unwrap_or(pos[0]);
                        let py: f32 = pos_tab.get(2).unwrap_or(pos[1]);
                        pos = [px, py];
                    }
                    let radius: f32 = ot.get("radius").unwrap_or(4.0);
                    let height_val: f32 = ot.get("height").unwrap_or(3.0);
                    let max_hp: f32 = ot
                        .get("max_hp")
                        .unwrap_or_else(|_| (radius * 240.0).max(480.0));
                    obstacles.push(ObstacleSpawn {
                        position: pos,
                        radius,
                        height: height_val,
                        max_hp,
                    });
                }
            }
        }

        Ok(MapDefinition {
            id,
            name,
            description,
            width,
            height,
            team_spawns,
            resource_nodes,
            obstacles,
        })
    }

    pub fn load_all_maps(&mut self) -> Result<(), mlua::Error> {
        let mut loaded_from_disk = false;
        let maps_dir = Path::new("assets/maps");

        if maps_dir.exists() && maps_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(maps_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                        if let Ok(code) = fs::read_to_string(&path) {
                            let val: Value = self.lua.load(&code).eval()?;
                            let map_table = match val {
                                Value::Table(t) => t,
                                _ => self.lua.globals().get::<Table>("map")?,
                            };
                            if let Ok(map_def) = self.parse_map_table(&map_table) {
                                if !self.map_order.contains(&map_def.id) {
                                    self.map_order.push(map_def.id.clone());
                                }
                                self.maps.insert(map_def.id.clone(), map_def);
                                loaded_from_disk = true;
                            }
                        }
                    }
                }
            }
        }

        if !loaded_from_disk {
            // Load embedded fallback maps
            let val1: Value = self.lua.load(FALLBACK_MAP_CROSSFIRE_LUA).eval()?;
            let t1 = match val1 {
                Value::Table(t) => t,
                _ => self.lua.globals().get::<Table>("map")?,
            };
            let crossfire = self.parse_map_table(&t1)?;
            if !self.map_order.contains(&crossfire.id) {
                self.map_order.push(crossfire.id.clone());
            }
            self.maps.insert(crossfire.id.clone(), crossfire);

            let val2: Value = self.lua.load(FALLBACK_MAP_MIDNIGHT_RIDGE_LUA).eval()?;
            let t2 = match val2 {
                Value::Table(t) => t,
                _ => self.lua.globals().get::<Table>("map")?,
            };
            let ridge = self.parse_map_table(&t2)?;
            if !self.map_order.contains(&ridge.id) {
                self.map_order.push(ridge.id.clone());
            }
            self.maps.insert(ridge.id.clone(), ridge);
        }

        Ok(())
    }

    pub fn maps(&self) -> &HashMap<String, MapDefinition> {
        &self.maps
    }

    pub fn map_order(&self) -> &[String] {
        &self.map_order
    }

    pub fn get_map(&self, id: &str) -> Option<&MapDefinition> {
        self.maps.get(id)
    }

    pub fn calculate_damage(&self, base_damage: f32, armor: f32, armor_piercing: f32) -> f32 {
        if let Ok(func) = self.lua.globals().get::<mlua::Function>("calculate_damage") {
            if let Ok(dmg) = func.call::<f32>((base_damage, armor, armor_piercing)) {
                return dmg;
            }
        }
        let eff_armor = (armor - armor_piercing).max(0.0);
        (base_damage - eff_armor).max(1.0)
    }

    pub fn compute_ai_commands(
        &self,
        world: &SimWorld,
        ai_team: TeamId,
        difficulty: AiDifficulty,
    ) -> Result<Vec<GameCommand>, mlua::Error> {
        let compute_fn: mlua::Function = match self.lua.globals().get("compute_ai_commands") {
            Ok(f) => f,
            Err(_) => return Ok(Vec::new()),
        };

        let state = self.lua.create_table()?;
        state.set("team", ai_team)?;
        state.set("tick", world.tick)?;
        state.set("resources", world.team_resources[ai_team as usize])?;
        state.set("supply_used", world.team_supply_used[ai_team as usize])?;
        state.set("supply_cap", world.team_supply_cap[ai_team as usize])?;
        state.set("difficulty", difficulty.as_u8())?;

        // Enemy base position
        let other_team_idx = if ai_team == 0 { 1 } else { 0 };
        let enemy_spawn = world.map.team_spawns[other_team_idx];
        let enemy_base_tab = self.lua.create_table()?;
        enemy_base_tab.set("x", enemy_spawn[0])?;
        enemy_base_tab.set("y", enemy_spawn[1])?;
        state.set("enemy_base_pos", enemy_base_tab)?;

        // 1. My Units
        let my_units = self.lua.create_table()?;
        let mut u_idx = 1;
        for unit in world.units.values() {
            if unit.team == ai_team {
                let u_tab = self.lua.create_table()?;
                u_tab.set("id", unit.id)?;
                u_tab.set("unit_type", unit.unit_type.as_str())?;
                u_tab.set("x", unit.pos.x)?;
                u_tab.set("y", unit.pos.y)?;
                u_tab.set("hp", unit.hp)?;
                u_tab.set("max_hp", unit.max_hp)?;
                u_tab.set("carrying_resources", unit.carrying_resources)?;

                let order_str = match unit.order {
                    UnitOrderState::Idle => "Idle",
                    UnitOrderState::Moving => "Moving",
                    UnitOrderState::AttackMoving => "AttackMoving",
                    UnitOrderState::Attacking(_) => "Attacking",
                    UnitOrderState::Constructing(_) => "Constructing",
                    UnitOrderState::Repairing(_) => "Repairing",
                    UnitOrderState::Harvesting { .. } => "Harvesting",
                };
                u_tab.set("order", order_str)?;

                my_units.set(u_idx, u_tab)?;
                u_idx += 1;
            }
        }
        state.set("my_units", my_units.clone())?;
        state.set("own_units", my_units)?;

        // 2. My Buildings
        let my_buildings = self.lua.create_table()?;
        let mut b_idx = 1;
        for b in world.buildings.values() {
            if b.team == ai_team {
                let b_tab = self.lua.create_table()?;
                b_tab.set("id", b.id)?;
                b_tab.set("building_type", b.building_type.as_str())?;
                b_tab.set("x", b.pos.x)?;
                b_tab.set("y", b.pos.y)?;
                b_tab.set("hp", b.hp)?;
                b_tab.set("max_hp", b.max_hp)?;
                b_tab.set("completed", b.completed)?;
                b_tab.set("queue_length", b.production_queue.len())?;
                b_tab.set("queue_len", b.production_queue.len())?;

                my_buildings.set(b_idx, b_tab)?;
                b_idx += 1;
            }
        }
        state.set("my_buildings", my_buildings.clone())?;
        state.set("own_buildings", my_buildings)?;

        // 3. Visible Enemies (strictly respecting Fog of War!)
        let visible_enemies = self.lua.create_table()?;
        let mut e_idx = 1;

        for unit in world.units.values() {
            if unit.team != ai_team && world.fog.is_pos_visible(ai_team, unit.pos) {
                let e_tab = self.lua.create_table()?;
                e_tab.set("id", unit.id)?;
                e_tab.set("is_building", false)?;
                e_tab.set("unit_type", unit.unit_type.as_str())?;
                e_tab.set("x", unit.pos.x)?;
                e_tab.set("y", unit.pos.y)?;
                e_tab.set("hp", unit.hp)?;
                e_tab.set("max_hp", unit.max_hp)?;
                visible_enemies.set(e_idx, e_tab)?;
                e_idx += 1;
            }
        }

        for b in world.buildings.values() {
            if b.team != ai_team && world.fog.is_pos_visible(ai_team, b.pos) {
                let e_tab = self.lua.create_table()?;
                e_tab.set("id", b.id)?;
                e_tab.set("is_building", true)?;
                e_tab.set("building_type", b.building_type.as_str())?;
                e_tab.set("x", b.pos.x)?;
                e_tab.set("y", b.pos.y)?;
                e_tab.set("hp", b.hp)?;
                e_tab.set("max_hp", b.max_hp)?;
                visible_enemies.set(e_idx, e_tab)?;
                e_idx += 1;
            }
        }
        state.set("visible_enemies", visible_enemies)?;

        // 4. Resource Nodes
        let resource_nodes = self.lua.create_table()?;
        let mut rn_idx = 1;
        for node in world.resource_nodes.values() {
            let rn_tab = self.lua.create_table()?;
            rn_tab.set("id", node.id)?;
            rn_tab.set("x", node.pos.x)?;
            rn_tab.set("y", node.pos.y)?;
            rn_tab.set("remaining", node.remaining)?;
            rn_tab.set("has_mine", node.occupied_mine_id.is_some())?;
            rn_tab.set("side_hint", node.side_hint)?;
            if let Some(m_id) = node.occupied_mine_id {
                rn_tab.set("occupied_mine_id", m_id)?;
            }
            resource_nodes.set(rn_idx, rn_tab)?;
            rn_idx += 1;
        }
        state.set("resource_nodes", resource_nodes)?;

        // Call compute_ai_commands(state)
        let action_tables: Table = compute_fn.call(state)?;
        let mut commands = Vec::new();

        let parse_id_list = |tab: &Table, key: &str| -> Vec<u32> {
            let mut list = Vec::new();
            if let Ok(sub) = tab.get::<Table>(key) {
                for val in sub.sequence_values::<u32>() {
                    if let Ok(id) = val {
                        list.push(id);
                    }
                }
            } else if let Ok(single) = tab.get::<u32>(key) {
                list.push(single);
            }
            list
        };

        let parse_pos = |tab: &Table, key: &str| -> Option<[f32; 2]> {
            if let Ok(sub) = tab.get::<Table>(key) {
                let x: f32 = sub.get(1).or_else(|_| sub.get("x")).unwrap_or(0.0);
                let y: f32 = sub.get(2).or_else(|_| sub.get("y")).unwrap_or(0.0);
                Some([x, y])
            } else if let (Ok(x), Ok(y)) = (tab.get::<f32>("x"), tab.get::<f32>("y")) {
                Some([x, y])
            } else {
                None
            }
        };

        for act_val in action_tables.sequence_values::<Table>() {
            let act_tab = match act_val {
                Ok(t) => t,
                Err(_) => continue,
            };

            let action: String = match act_tab.get("action") {
                Ok(a) => a,
                Err(_) => continue,
            };

            match action.as_str() {
                "train_unit" | "train" => {
                    if let (Ok(building_id), Ok(unit_type)) =
                        (act_tab.get::<u32>("building_id"), act_tab.get::<String>("unit_type"))
                    {
                        commands.push(GameCommand::TrainUnit {
                            team: ai_team,
                            building_id,
                            unit_type,
                        });
                    }
                }

                "construct_building" | "build" => {
                    let mut worker_ids = parse_id_list(&act_tab, "worker_ids");
                    if worker_ids.is_empty() {
                        worker_ids = parse_id_list(&act_tab, "worker_id");
                    }
                    let building_type: String = act_tab.get("building_type").unwrap_or_default();
                    let position = parse_pos(&act_tab, "position")
                        .or_else(|| parse_pos(&act_tab, "pos"))
                        .or_else(|| {
                            if let (Ok(x), Ok(y)) = (act_tab.get::<f32>("x"), act_tab.get::<f32>("y")) {
                                Some([x, y])
                            } else {
                                None
                            }
                        })
                        .unwrap_or([0.0, 0.0]);
                    let resource_node_id = act_tab
                        .get::<u32>("resource_node_id")
                        .or_else(|_| act_tab.get::<u32>("node_id"))
                        .ok();

                    if !building_type.is_empty() && !worker_ids.is_empty() {
                        commands.push(GameCommand::ConstructBuilding {
                            team: ai_team,
                            worker_ids,
                            building_type,
                            position,
                            resource_node_id,
                        });
                    }
                }

                "repair_building" | "repair" => {
                    let mut worker_ids = parse_id_list(&act_tab, "worker_ids");
                    if worker_ids.is_empty() {
                        worker_ids = parse_id_list(&act_tab, "worker_id");
                    }
                    if let Ok(building_id) = act_tab.get::<u32>("building_id") {
                        if !worker_ids.is_empty() {
                            commands.push(GameCommand::RepairBuilding {
                                team: ai_team,
                                worker_ids,
                                building_id,
                            });
                        }
                    }
                }

                "harvest_mine" | "harvest" => {
                    let mut worker_ids = parse_id_list(&act_tab, "worker_ids");
                    if worker_ids.is_empty() {
                        worker_ids = parse_id_list(&act_tab, "worker_id");
                    }
                    if let Ok(target_id) = act_tab.get::<u32>("target_id") {
                        if !worker_ids.is_empty() {
                            commands.push(GameCommand::HarvestMine {
                                team: ai_team,
                                worker_ids,
                                target_id,
                            });
                        }
                    }
                }

                "attack_move" => {
                    let unit_ids = parse_id_list(&act_tab, "unit_ids");
                    let target = parse_pos(&act_tab, "target").unwrap_or_else(|| {
                        let x: f32 = act_tab.get("x").unwrap_or(0.0);
                        let y: f32 = act_tab.get("y").unwrap_or(0.0);
                        [x, y]
                    });
                    if !unit_ids.is_empty() {
                        commands.push(GameCommand::MoveUnits {
                            team: ai_team,
                            unit_ids,
                            target,
                            attack_move: true,
                        });
                    }
                }

                "move_units" | "move" => {
                    let unit_ids = parse_id_list(&act_tab, "unit_ids");
                    if let Some(target) = parse_pos(&act_tab, "target") {
                        let attack_move: bool = act_tab.get("attack_move").unwrap_or(false);
                        if !unit_ids.is_empty() {
                            commands.push(GameCommand::MoveUnits {
                                team: ai_team,
                                unit_ids,
                                target,
                                attack_move,
                            });
                        }
                    }
                }

                "attack_target" | "attack" => {
                    let unit_ids = parse_id_list(&act_tab, "unit_ids");
                    if let Ok(target_id) = act_tab.get::<u32>("target_id") {
                        if !unit_ids.is_empty() {
                            commands.push(GameCommand::AttackTarget {
                                team: ai_team,
                                unit_ids,
                                target_id,
                            });
                        }
                    }
                }

                "stop_units" | "stop" => {
                    let unit_ids = parse_id_list(&act_tab, "unit_ids");
                    if !unit_ids.is_empty() {
                        commands.push(GameCommand::StopUnits {
                            team: ai_team,
                            unit_ids,
                        });
                    }
                }

                _ => {}
            }
        }

        Ok(commands)
    }
}
