use std::collections::BTreeMap;
use glam::{Vec2, Vec3};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::sim::fog_of_war::FogOfWar;
use crate::sim::pathfinding::{compute_separation, find_path};
use crate::sim::registry::BlueprintRegistry;
use crate::types::{
    vec2_from_arr, BuildingBlueprint, EntityId, FogTileState, GameCommand, MapDefinition,
    ObstacleSpawn, RenderInstance, TeamId, UnitBlueprint, UnitOrderState, TEAM_NEUTRAL,
};

#[derive(Copy, Clone, PartialEq, Eq)]
enum CombatTargetKind {
    Unit,
    Building,
    Rock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RockEntity {
    pub id: EntityId,
    pub pos: Vec2,
    pub radius: f32,
    pub height: f32,
    pub hp: f32,
    pub max_hp: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitEntity {
    pub id: EntityId,
    pub team: TeamId,
    pub unit_type: String,
    pub pos: Vec2,
    pub rotation_yaw: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub order: UnitOrderState,
    pub waypoints: Vec<Vec2>,
    pub attack_cooldown: u32,
    pub harvest_progress: u32,
    pub carrying_resources: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingEntity {
    pub id: EntityId,
    pub team: TeamId,
    pub building_type: String,
    pub pos: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub completed: bool,
    pub build_progress_ticks: u32,
    pub build_total_ticks: u32,
    pub production_queue: Vec<String>,
    pub production_progress_ticks: u32,
    pub linked_node_id: Option<EntityId>,
    #[serde(default)]
    pub attack_cooldown: u32,
    #[serde(default)]
    pub rotation_yaw: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNodeEntity {
    pub id: EntityId,
    pub pos: Vec2,
    pub remaining: u32,
    pub side_hint: TeamId,
    pub occupied_mine_id: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileVisual {
    pub start: Vec3,
    pub end: Vec3,
    pub pos: Vec3,
    pub speed: f32,
    pub team: TeamId,
    pub ttl_ticks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimWorld {
    pub tick: u64,
    pub map: MapDefinition,
    pub registry: BlueprintRegistry,
    pub fog: FogOfWar,
    pub team_resources: [u32; 2],
    pub team_supply_used: [u32; 2],
    pub team_supply_cap: [u32; 2],
    pub winner: Option<TeamId>,
    pub paused: bool,
    pub units: BTreeMap<EntityId, UnitEntity>,
    pub buildings: BTreeMap<EntityId, BuildingEntity>,
    pub resource_nodes: BTreeMap<EntityId, ResourceNodeEntity>,
    pub rocks: BTreeMap<EntityId, RockEntity>,
    pub projectiles: Vec<ProjectileVisual>,
    pub next_entity_id: EntityId,
}

impl SimWorld {
    pub fn new(map: MapDefinition, registry: BlueprintRegistry) -> Self {
        Self::new_seeded(map, registry, rand::random())
    }

    /// Creates a world whose starting positions and breakable rock structures are
    /// selected reproducibly from the match seed. LAN peers must use the same seed.
    pub fn new_seeded(mut map: MapDefinition, registry: BlueprintRegistry, seed: u64) -> Self {
        Self::select_starting_spawns(&mut map, seed);
        Self::populate_breakable_rocks(&mut map, seed);
        let starting_resources = registry.economy.starting_resources;
        let fog = FogOfWar::new(map.width, map.height, 2.0);

        let mut world = Self {
            tick: 0,
            map,
            registry,
            fog,
            team_resources: [starting_resources, starting_resources],
            team_supply_used: [0, 0],
            team_supply_cap: [0, 0],
            winner: None,
            paused: false,
            units: BTreeMap::new(),
            buildings: BTreeMap::new(),
            resource_nodes: BTreeMap::new(),
            rocks: BTreeMap::new(),
            projectiles: Vec::new(),
            next_entity_id: 1,
        };

        world.initialize_from_map();
        world
    }

    /// Ensures a large central breakable rock barrier sits between the two map halves
    /// and adds seeded random rock structures partially shielding resource mines and
    /// creating tactical defensive positions across the map.
    fn populate_breakable_rocks(map: &mut MapDefinition, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed ^ 0x524F_434B_5F47_454E);
        let center = Vec2::new(map.width * 0.5, map.height * 0.5);
        let spawn0 = vec2_from_arr(map.team_spawns[0]);
        let spawn1 = vec2_from_arr(map.team_spawns[1]);
        let node_positions: Vec<Vec2> = map
            .resource_nodes
            .iter()
            .map(|n| vec2_from_arr(n.position))
            .collect();

        let is_clear_for_rock =
            |pos: Vec2, radius: f32, existing: &[ObstacleSpawn]| -> bool {
                let margin = radius + 3.0;
                if pos.x < margin
                    || pos.x > map.width - margin
                    || pos.y < margin
                    || pos.y > map.height - margin
                {
                    return false;
                }
                // Keep starting Command Centers and worker rings clear
                if pos.distance(spawn0) < radius + 6.5 || pos.distance(spawn1) < radius + 6.5 {
                    return false;
                }
                // Keep resource nodes buildable and harvestable from the base side
                for &np in &node_positions {
                    if pos.distance(np) < radius + 3.2 {
                        return false;
                    }
                }
                // Also ensure we never block the direct segment between a starting CC and its closest mine
                for &cc in &[spawn0, spawn1] {
                    if let Some(&start_mine) = node_positions.iter().min_by(|a, b| {
                        a.distance_squared(cc)
                            .partial_cmp(&b.distance_squared(cc))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }) {
                        let seg = start_mine - cc;
                        let len_sq = seg.length_squared();
                        if len_sq > 0.01 {
                            let t = ((pos - cc).dot(seg) / len_sq).clamp(0.0, 1.0);
                            let proj = cc + seg * t;
                            if pos.distance(proj) < radius + 3.5 {
                                return false;
                            }
                        }
                    }
                }
                for obs in existing {
                    let op = vec2_from_arr(obs.position);
                    if pos.distance(op) < obs.radius + radius + 0.8 {
                        return false;
                    }
                }
                true
            };

        // 1. Ensure a large breakable rock structure sits in the middle between the two halves
        let has_central_massif = map
            .obstacles
            .iter()
            .any(|o| vec2_from_arr(o.position).distance(center) <= 6.0 && o.radius >= 6.5);
        if !has_central_massif {
            if let Some(existing_mid) = map
                .obstacles
                .iter_mut()
                .find(|o| vec2_from_arr(o.position).distance(center) <= 6.0)
            {
                existing_mid.radius = existing_mid.radius.max(7.8);
                existing_mid.height = existing_mid.height.max(11.0);
                existing_mid.max_hp = existing_mid.max_hp.max(2400.0);
            } else {
                map.obstacles.push(ObstacleSpawn {
                    position: [center.x, center.y],
                    radius: 8.0,
                    height: 11.0,
                    max_hp: 2400.0,
                });
            }
        }

        // 2. Add randomized protective rock structures partially shielding mines from the center/enemy side
        for &node_pos in &node_positions {
            let to_center = (center - node_pos).normalize_or_zero();
            if to_center == Vec2::ZERO {
                continue;
            }
            let base_angle = to_center.y.atan2(to_center.x);
            // Try placing 1-2 protective rock outcrops on the exposed side of each mine
            for slot in [-1.0_f32, 1.0_f32] {
                let angle_jitter: f32 = rng.gen_range(-0.28..0.28);
                let angle = base_angle + slot * 0.48 + angle_jitter;
                let dist: f32 = rng.gen_range(5.6..7.0);
                let radius: f32 = rng.gen_range(2.4..3.3);
                let height: f32 = rng.gen_range(4.4..6.4);
                let max_hp: f32 = (radius * 240.0).round();
                let candidate = node_pos + Vec2::new(angle.cos() * dist, angle.sin() * dist);

                if is_clear_for_rock(candidate, radius, &map.obstacles) {
                    map.obstacles.push(ObstacleSpawn {
                        position: [candidate.x, candidate.y],
                        radius,
                        height,
                        max_hp,
                    });
                }
            }
        }

        // 3. Add random breakable rock structures across midfield / flank lanes
        for _ in 0..18 {
            let rx: f32 = rng.gen_range((map.width * 0.16)..(map.width * 0.84));
            let ry: f32 = rng.gen_range((map.height * 0.16)..(map.height * 0.84));
            let candidate = Vec2::new(rx, ry);
            let radius: f32 = rng.gen_range(2.4..3.6);
            let height: f32 = rng.gen_range(4.8..7.2);
            let max_hp: f32 = (radius * 250.0).round();

            if is_clear_for_rock(candidate, radius, &map.obstacles) {
                map.obstacles.push(ObstacleSpawn {
                    position: [candidate.x, candidate.y],
                    radius,
                    height,
                    max_hp,
                });
            }
        }
    }

    fn select_starting_spawns(map: &mut MapDefinition, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);

        for team in 0..2 {
            let home = vec2_from_arr(map.team_spawns[team]);
            let candidates: Vec<Vec2> = map
                .resource_nodes
                .iter()
                .filter(|node| node.side_hint == team as TeamId)
                .map(|node| vec2_from_arr(node.position))
                .collect();

            if candidates.is_empty() {
                continue;
            }
            let node_pos = candidates[rng.gen_range(0..candidates.len())];

            // Keep the command center clear of the mine while placing it on
            // the team's homeward side of the chosen node.
            let direction = (home - node_pos).normalize_or_zero();
            let direction = if direction == Vec2::ZERO {
                if team == 0 {
                    Vec2::splat(-1.0).normalize()
                } else {
                    Vec2::ONE.normalize()
                }
            } else {
                direction
            };
            let spawn = (node_pos + direction * 7.0).clamp(
                Vec2::splat(4.0),
                Vec2::new(map.width - 4.0, map.height - 4.0),
            );
            map.team_spawns[team] = spawn.to_array();
        }
    }

    pub fn next_id(&mut self) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id = self.next_entity_id.wrapping_add(1);
        id
    }

    fn initialize_from_map(&mut self) {
        // Spawn breakable rock structures from MapDefinition obstacles
        let obs_spawns = self.map.obstacles.clone();
        for obs in &obs_spawns {
            let id = self.next_id();
            let max_hp = if obs.max_hp > 0.0 {
                obs.max_hp
            } else {
                (obs.radius * 240.0).max(480.0)
            };
            self.rocks.insert(
                id,
                RockEntity {
                    id,
                    pos: vec2_from_arr(obs.position),
                    radius: obs.radius,
                    height: obs.height,
                    hp: max_hp,
                    max_hp,
                },
            );
        }

        // Spawn resource nodes from MapDefinition
        let node_spawns = self.map.resource_nodes.clone();
        for node_spawn in &node_spawns {
            let id = self.next_id();
            self.resource_nodes.insert(
                id,
                ResourceNodeEntity {
                    id,
                    pos: vec2_from_arr(node_spawn.position),
                    remaining: node_spawn.amount,
                    side_hint: node_spawn.side_hint,
                    occupied_mine_id: None,
                },
            );
        }

        // Spawn starting command_center, 3 workers, and 1 starting mine for Team 0 and Team 1
        for team in 0..=1 {
            let spawn_coord = self.map.team_spawns[team as usize];
            let cc_pos = vec2_from_arr(spawn_coord);

            let cc_bp = self
                .registry
                .get_building("command_center")
                .cloned()
                .unwrap_or_else(|| BuildingBlueprint {
                    id: "command_center".to_string(),
                    display_name: "Command Center".to_string(),
                    mesh_archetype: "command_center".to_string(),
                    footprint_radius: 3.5,
                    cost: 400,
                    build_time_ticks: 600,
                    max_hp: 2000.0,
                    armor: 5.0,
                    vision_range: 30.0,
                    supply_provided: 10,
                    requires_resource_node: false,
                    is_resource_dropoff: true,
                    is_mine_extractor: false,
                    trainable_units: vec!["worker".to_string()],
                    hotkey: "C".to_string(),
                    attack_damage: 0.0,
                    attack_range: 0.0,
                    attack_cooldown_ticks: 0,
                    armor_piercing: 0.0,
                    projectile_speed: 0.0,
                });

            let cc_id = self.next_id();
            self.buildings.insert(
                cc_id,
                BuildingEntity {
                    id: cc_id,
                    team: team as TeamId,
                    building_type: "command_center".to_string(),
                    pos: cc_pos,
                    hp: cc_bp.max_hp,
                    max_hp: cc_bp.max_hp,
                    completed: true,
                    build_progress_ticks: cc_bp.build_time_ticks,
                    build_total_ticks: cc_bp.build_time_ticks,
                    production_queue: Vec::new(),
                    production_progress_ticks: 0,
                    linked_node_id: None,
                    attack_cooldown: 0,
                    rotation_yaw: 0.0,
                },
            );

            // The randomized spawn sits beside one of this team's nodes; retain
            // side ownership when selecting its starting mine.
            let closest_node_id = self
                .resource_nodes
                .values()
                .filter(|n| n.occupied_mine_id.is_none() && n.side_hint == team as TeamId)
                .min_by(|a, b| {
                    let d_a = (a.pos - cc_pos).length_squared();
                    let d_b = (b.pos - cc_pos).length_squared();
                    d_a.partial_cmp(&d_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|n| n.id);

            if let Some(node_id) = closest_node_id {
                let node_pos = self.resource_nodes[&node_id].pos;
                let mine_bp = self
                    .registry
                    .get_building("mine")
                    .cloned()
                    .unwrap_or_else(|| BuildingBlueprint {
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
                        trainable_units: Vec::new(),
                        hotkey: "M".to_string(),
                        attack_damage: 0.0,
                        attack_range: 0.0,
                        attack_cooldown_ticks: 0,
                        armor_piercing: 0.0,
                        projectile_speed: 0.0,
                    });

                let mine_id = self.next_id();
                self.buildings.insert(
                    mine_id,
                    BuildingEntity {
                        id: mine_id,
                        team: team as TeamId,
                        building_type: "mine".to_string(),
                        pos: node_pos,
                        hp: mine_bp.max_hp,
                        max_hp: mine_bp.max_hp,
                        completed: true,
                        build_progress_ticks: mine_bp.build_time_ticks,
                        build_total_ticks: mine_bp.build_time_ticks,
                        production_queue: Vec::new(),
                        production_progress_ticks: 0,
                        linked_node_id: Some(node_id),
                        attack_cooldown: 0,
                        rotation_yaw: 0.0,
                    },
                );
                if let Some(node) = self.resource_nodes.get_mut(&node_id) {
                    node.occupied_mine_id = Some(mine_id);
                }
            }

            // Spawn 3 starting workers
            let worker_bp = self
                .registry
                .get_unit("worker")
                .cloned()
                .unwrap_or_else(|| UnitBlueprint {
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
                    attack_range: 2.0,
                    attack_cooldown_ticks: 30,
                    projectile_speed: 0.0,
                    can_harvest: true,
                    can_build: true,
                    can_repair: true,
                    producer_building: "command_center".to_string(),
                    hotkey: "W".to_string(),
                });

            for i in 0..3 {
                let angle = (i as f32) * (std::f32::consts::TAU / 3.0);
                let offset = Vec2::new(angle.cos() * 6.2, angle.sin() * 6.2);
                let w_pos = (cc_pos + offset).clamp(
                    Vec2::new(2.0, 2.0),
                    Vec2::new(self.map.width - 2.0, self.map.height - 2.0),
                );

                let w_id = self.next_id();
                self.units.insert(
                    w_id,
                    UnitEntity {
                        id: w_id,
                        team: team as TeamId,
                        unit_type: "worker".to_string(),
                        pos: w_pos,
                        rotation_yaw: angle,
                        hp: worker_bp.max_hp,
                        max_hp: worker_bp.max_hp,
                        order: UnitOrderState::Idle,
                        waypoints: Vec::new(),
                        attack_cooldown: 0,
                        harvest_progress: 0,
                        carrying_resources: 0,
                    },
                );
            }
        }

        self.recompute_supplies();

        // Pre-explore each player's own side of the map (spawn area & own-side resource mines)
        for team in 0..=1 {
            let spawn_pos = vec2_from_arr(self.map.team_spawns[team]);
            self.fog.reveal_circle(team as TeamId, spawn_pos, 36.0);
            for node in self.resource_nodes.values() {
                if node.side_hint == team as TeamId {
                    self.fog.reveal_circle(team as TeamId, node.pos, 14.0);
                }
            }
        }

        self.update_fog();
    }

    pub fn recompute_supplies(&mut self) {
        self.team_supply_cap = [0, 0];
        self.team_supply_used = [0, 0];

        for building in self.buildings.values() {
            if building.completed {
                if let Some(bp) = self.registry.get_building(&building.building_type) {
                    let team_idx = building.team as usize;
                    if team_idx < 2 {
                        self.team_supply_cap[team_idx] += bp.supply_provided;
                    }
                }
            }
        }

        for unit in self.units.values() {
            if let Some(bp) = self.registry.get_unit(&unit.unit_type) {
                let team_idx = unit.team as usize;
                if team_idx < 2 {
                    self.team_supply_used[team_idx] += bp.supply_cost;
                }
            }
        }

        for building in self.buildings.values() {
            if building.completed {
                let team_idx = building.team as usize;
                if team_idx < 2 {
                    for queued_id in &building.production_queue {
                        if let Some(bp) = self.registry.get_unit(queued_id) {
                            self.team_supply_used[team_idx] += bp.supply_cost;
                        }
                    }
                }
            }
        }
    }

    pub fn update_fog(&mut self) {
        self.fog.begin_tick();

        for unit in self.units.values() {
            let vision = self
                .registry
                .get_unit(&unit.unit_type)
                .map(|bp| bp.vision_range)
                .unwrap_or(16.0);
            self.fog.reveal_circle(unit.team, unit.pos, vision);
        }

        for b in self.buildings.values() {
            let vision = self
                .registry
                .get_building(&b.building_type)
                .map(|bp| bp.vision_range)
                .unwrap_or(20.0);
            self.fog.reveal_circle(b.team, b.pos, vision);
        }
    }

    fn collect_path_obstacles(&self) -> Vec<(Vec2, f32)> {
        let mut obstacles = Vec::with_capacity(self.rocks.len() + self.buildings.len());
        for rock in self.rocks.values() {
            obstacles.push((rock.pos, rock.radius));
        }
        for b in self.buildings.values() {
            let radius = self
                .registry
                .get_building(&b.building_type)
                .map(|bp| bp.footprint_radius)
                .unwrap_or(2.5);
            obstacles.push((b.pos, radius));
        }
        obstacles
    }

    pub fn apply_command(&mut self, cmd: GameCommand) {
        match cmd {
            GameCommand::MoveUnits {
                team,
                unit_ids,
                target,
                attack_move,
            } => {
                let target_pos = vec2_from_arr(target);
                let valid_ids: Vec<EntityId> = unit_ids
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.units
                            .get(id)
                            .map(|u| u.team == team)
                            .unwrap_or(false)
                    })
                    .collect();

                let n = valid_ids.len();
                if n == 0 {
                    return;
                }

                let obstacles = self.collect_path_obstacles();
                let cols = (n as f32).sqrt().ceil() as usize;
                let spacing = 2.0;

                for (idx, id) in valid_ids.iter().enumerate() {
                    let col = idx % cols;
                    let row = idx / cols;
                    let offset = Vec2::new(
                        (col as f32 - (cols - 1) as f32 * 0.5) * spacing,
                        (row as f32) * spacing,
                    );
                    let dest = (target_pos + offset).clamp(
                        Vec2::new(1.0, 1.0),
                        Vec2::new(self.map.width - 1.0, self.map.height - 1.0),
                    );

                    if let Some(unit) = self.units.get_mut(id) {
                        let path = find_path(
                            unit.pos,
                            dest,
                            self.map.width,
                            self.map.height,
                            &obstacles,
                        );
                        unit.waypoints = path;
                        unit.order = if attack_move {
                            UnitOrderState::AttackMoving
                        } else {
                            UnitOrderState::Moving
                        };
                    }
                }
            }

            GameCommand::AttackTarget {
                team,
                unit_ids,
                target_id,
            } => {
                let (target_pos, requires_fog_visibility) =
                    if let Some(target_unit) = self.units.get(&target_id) {
                        if target_unit.team == team {
                            return;
                        }
                        (target_unit.pos, true)
                    } else if let Some(target_b) = self.buildings.get(&target_id) {
                        if target_b.team == team {
                            return;
                        }
                        (target_b.pos, true)
                    } else if let Some(target_rock) = self.rocks.get(&target_id) {
                        (target_rock.pos, false)
                    } else {
                        return;
                    };

                // Enemy units and buildings must be visible in Fog of War; terrain rocks can always be targeted
                if requires_fog_visibility && !self.fog.is_pos_visible(team, target_pos) {
                    return;
                }

                for id in unit_ids {
                    if let Some(unit) = self.units.get_mut(&id) {
                        if unit.team == team {
                            unit.order = UnitOrderState::Attacking(target_id);
                            unit.waypoints.clear();
                        }
                    }
                }
            }

            GameCommand::ConstructBuilding {
                team,
                worker_ids,
                building_type,
                position,
                resource_node_id,
            } => {
                let bp = match self.registry.get_building(&building_type).cloned() {
                    Some(b) => b,
                    None => return,
                };

                let team_idx = team as usize;
                if team_idx >= 2 || self.team_resources[team_idx] < bp.cost {
                    return;
                }

                let mut b_pos = vec2_from_arr(position);
                let mut linked_id = resource_node_id;

                if building_type == "mine" {
                    if linked_id.is_none() {
                        linked_id = self
                            .resource_nodes
                            .values()
                            .filter(|n| n.occupied_mine_id.is_none())
                            .min_by(|a, b| {
                                let da = (a.pos - b_pos).length_squared();
                                let db = (b.pos - b_pos).length_squared();
                                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .filter(|n| (n.pos - b_pos).length() <= 5.0)
                            .map(|n| n.id);
                    }

                    if let Some(nid) = linked_id {
                        if let Some(node) = self.resource_nodes.get(&nid) {
                            if node.occupied_mine_id.is_some() {
                                return;
                            }
                            b_pos = node.pos;
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                } else {
                    // Check bounds and collision
                    let margin = bp.footprint_radius + 0.5;
                    if b_pos.x < margin
                        || b_pos.x > self.map.width - margin
                        || b_pos.y < margin
                        || b_pos.y > self.map.height - margin
                    {
                        return;
                    }

                    for rock in self.rocks.values() {
                        if (b_pos - rock.pos).length() < rock.radius + bp.footprint_radius {
                            return;
                        }
                    }

                    for existing_b in self.buildings.values() {
                        let e_bp_radius = self
                            .registry
                            .get_building(&existing_b.building_type)
                            .map(|b| b.footprint_radius)
                            .unwrap_or(2.5);
                        if (b_pos - existing_b.pos).length() < bp.footprint_radius + e_bp_radius {
                            return;
                        }
                    }
                }

                // Deduct cost and spawn
                self.team_resources[team_idx] -= bp.cost;
                let b_id = self.next_id();
                let initial_hp = bp.max_hp * 0.15;

                self.buildings.insert(
                    b_id,
                    BuildingEntity {
                        id: b_id,
                        team,
                        building_type: building_type.clone(),
                        pos: b_pos,
                        hp: initial_hp,
                        max_hp: bp.max_hp,
                        completed: false,
                        build_progress_ticks: 0,
                        build_total_ticks: bp.build_time_ticks,
                        production_queue: Vec::new(),
                        production_progress_ticks: 0,
                        linked_node_id: linked_id,
                        attack_cooldown: 0,
                        rotation_yaw: 0.0,
                    },
                );

                if let Some(nid) = linked_id {
                    if let Some(node) = self.resource_nodes.get_mut(&nid) {
                        node.occupied_mine_id = Some(b_id);
                    }
                }

                // Order workers to construct
                for wid in worker_ids {
                    if let Some(w) = self.units.get_mut(&wid) {
                        if w.team == team {
                            let can_build = self
                                .registry
                                .get_unit(&w.unit_type)
                                .map(|ub| ub.can_build)
                                .unwrap_or(false);
                            if can_build {
                                w.order = UnitOrderState::Constructing(b_id);
                                w.waypoints.clear();
                            }
                        }
                    }
                }
            }

            GameCommand::RepairBuilding {
                team,
                worker_ids,
                building_id,
            } => {
                if let Some(b) = self.buildings.get(&building_id) {
                    if b.team != team {
                        return;
                    }
                } else {
                    return;
                }

                for wid in worker_ids {
                    if let Some(w) = self.units.get_mut(&wid) {
                        if w.team == team {
                            let can_repair = self
                                .registry
                                .get_unit(&w.unit_type)
                                .map(|ub| ub.can_repair)
                                .unwrap_or(false);
                            if can_repair {
                                w.order = UnitOrderState::Repairing(building_id);
                                w.waypoints.clear();
                            }
                        }
                    }
                }
            }

            GameCommand::HarvestMine {
                team,
                worker_ids,
                target_id,
            } => {
                let mut resolved_mine_id = None;

                if let Some(node) = self.resource_nodes.get(&target_id) {
                    if let Some(occupied_id) = node.occupied_mine_id {
                        resolved_mine_id = Some(occupied_id);
                    } else {
                        // Construct mine if team can afford it
                        let mine_cost = self
                            .registry
                            .get_building("mine")
                            .map(|b| b.cost)
                            .unwrap_or(150);
                        let team_idx = team as usize;
                        if team_idx < 2 && self.team_resources[team_idx] >= mine_cost {
                            self.apply_command(GameCommand::ConstructBuilding {
                                team,
                                worker_ids: worker_ids.clone(),
                                building_type: "mine".to_string(),
                                position: [node.pos.x, node.pos.y],
                                resource_node_id: Some(target_id),
                            });
                            return;
                        }
                    }
                } else if let Some(b) = self.buildings.get(&target_id) {
                    if b.building_type == "mine" {
                        resolved_mine_id = Some(target_id);
                    }
                }

                if let Some(mine_id) = resolved_mine_id {
                    for wid in worker_ids {
                        if let Some(w) = self.units.get_mut(&wid) {
                            if w.team == team {
                                let can_harvest = self
                                    .registry
                                    .get_unit(&w.unit_type)
                                    .map(|ub| ub.can_harvest)
                                    .unwrap_or(false);
                                if can_harvest {
                                    w.order = UnitOrderState::Harvesting {
                                        mine_id,
                                        returning_to_base: false,
                                    };
                                    w.waypoints.clear();
                                }
                            }
                        }
                    }
                }
            }

            GameCommand::TrainUnit {
                team,
                building_id,
                unit_type,
            } => {
                let team_idx = team as usize;
                if team_idx >= 2 {
                    return;
                }

                let unit_bp = match self.registry.get_unit(&unit_type).cloned() {
                    Some(ub) => ub,
                    None => return,
                };

                let building = match self.buildings.get_mut(&building_id) {
                    Some(b) => b,
                    None => return,
                };

                if building.team != team || !building.completed {
                    return;
                }

                let building_bp = match self.registry.get_building(&building.building_type) {
                    Some(bb) => bb,
                    None => return,
                };

                if !building_bp.trainable_units.contains(&unit_type) {
                    return;
                }

                if self.team_resources[team_idx] < unit_bp.cost {
                    return;
                }

                if self.team_supply_used[team_idx] + unit_bp.supply_cost
                    > self.team_supply_cap[team_idx]
                {
                    return;
                }

                self.team_resources[team_idx] -= unit_bp.cost;
                self.team_supply_used[team_idx] += unit_bp.supply_cost;
                building.production_queue.push(unit_type);
            }

            GameCommand::StopUnits { team, unit_ids } => {
                for id in unit_ids {
                    if let Some(unit) = self.units.get_mut(&id) {
                        if unit.team == team {
                            unit.order = UnitOrderState::Idle;
                            unit.waypoints.clear();
                        }
                    }
                }
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn step_tick(&mut self, damage_fn: Option<&dyn Fn(f32, f32, f32) -> f32>) {
        if self.paused {
            return;
        }

        self.tick += 1;

        // 1. Update Fog of War
        self.update_fog();

        // 2. Step building production queues
        let mut newly_trained = Vec::new();
        let b_keys: Vec<EntityId> = self.buildings.keys().copied().collect();
        for b_id in b_keys {
            if let Some(b) = self.buildings.get_mut(&b_id) {
                if !b.completed || b.production_queue.is_empty() {
                    continue;
                }

                let unit_type = b.production_queue[0].clone();
                let build_time = self
                    .registry
                    .get_unit(&unit_type)
                    .map(|u| u.build_time_ticks)
                    .unwrap_or(100);

                b.production_progress_ticks += 1;
                if b.production_progress_ticks >= build_time {
                    b.production_queue.remove(0);
                    b.production_progress_ticks = 0;

                    let b_pos = b.pos;
                    let b_team = b.team;
                    let b_footprint = self
                        .registry
                        .get_building(&b.building_type)
                        .map(|bb| bb.footprint_radius)
                        .unwrap_or(2.5);

                    let angle = (self.tick % 16) as f32 * (std::f32::consts::TAU / 16.0);
                    let spawn_dist = b_footprint + 1.6;
                    let spawn_pos = (b_pos + Vec2::new(angle.cos() * spawn_dist, angle.sin() * spawn_dist))
                        .clamp(
                            Vec2::new(1.0, 1.0),
                            Vec2::new(self.map.width - 1.0, self.map.height - 1.0),
                        );

                    newly_trained.push((b_team, unit_type, spawn_pos, angle));
                }
            }
        }

        for (team, unit_type, spawn_pos, angle) in newly_trained {
            let id = self.next_id();
            let hp = self
                .registry
                .get_unit(&unit_type)
                .map(|u| u.max_hp)
                .unwrap_or(100.0);
            self.units.insert(
                id,
                UnitEntity {
                    id,
                    team,
                    unit_type,
                    pos: spawn_pos,
                    rotation_yaw: angle,
                    hp,
                    max_hp: hp,
                    order: UnitOrderState::Idle,
                    waypoints: Vec::new(),
                    attack_cooldown: 0,
                    harvest_progress: 0,
                    carrying_resources: 0,
                },
            );
        }

        // 3. Step worker orders (Constructing, Repairing, Harvesting)
        let economy = self.registry.economy.clone();
        let u_keys: Vec<EntityId> = self.units.keys().copied().collect();

        for u_id in u_keys {
            let (order, u_team, u_pos) = match self.units.get(&u_id) {
                Some(u) => (u.order, u.team, u.pos),
                None => continue,
            };

            match order {
                UnitOrderState::Constructing(target_b_id) => {
                    let mut completed_mine = false;
                    let mut finished = false;

                    if let Some(target_b) = self.buildings.get_mut(&target_b_id) {
                        if target_b.completed {
                            finished = true;
                            if target_b.building_type == "mine" {
                                completed_mine = true;
                            }
                        } else {
                            let footprint = self
                                .registry
                                .get_building(&target_b.building_type)
                                .map(|bb| bb.footprint_radius)
                                .unwrap_or(2.5);
                            let dist = (u_pos - target_b.pos).length();

                            if dist > footprint + 2.2 {
                                let dir = (target_b.pos - u_pos).normalize_or_zero();
                                if let Some(u) = self.units.get_mut(&u_id) {
                                    let speed = self
                                        .registry
                                        .get_unit(&u.unit_type)
                                        .map(|ub| ub.move_speed)
                                        .unwrap_or(3.0);
                                    u.pos += dir * (speed * 0.05);
                                    u.rotation_yaw = dir.y.atan2(dir.x);
                                }
                            } else {
                                target_b.build_progress_ticks += 1;
                                let ratio = (target_b.build_progress_ticks as f32
                                    / target_b.build_total_ticks.max(1) as f32)
                                    .min(1.0);
                                target_b.hp =
                                    target_b.max_hp * 0.15 + target_b.max_hp * 0.85 * ratio;

                                if target_b.build_progress_ticks >= target_b.build_total_ticks {
                                    target_b.completed = true;
                                    target_b.hp = target_b.max_hp;
                                    finished = true;
                                    if target_b.building_type == "mine" {
                                        completed_mine = true;
                                    }
                                }
                            }
                        }
                    } else {
                        finished = true;
                    }

                    if finished {
                        if let Some(u) = self.units.get_mut(&u_id) {
                            if completed_mine {
                                u.order = UnitOrderState::Harvesting {
                                    mine_id: target_b_id,
                                    returning_to_base: false,
                                };
                            } else {
                                u.order = UnitOrderState::Idle;
                            }
                        }
                    }
                }

                UnitOrderState::Repairing(target_b_id) => {
                    let mut finished = false;
                    let team_idx = u_team as usize;

                    if let Some(target_b) = self.buildings.get_mut(&target_b_id) {
                        let footprint = self
                            .registry
                            .get_building(&target_b.building_type)
                            .map(|bb| bb.footprint_radius)
                            .unwrap_or(2.5);
                        let dist = (u_pos - target_b.pos).length();

                        if dist > footprint + 2.2 {
                            let dir = (target_b.pos - u_pos).normalize_or_zero();
                            if let Some(u) = self.units.get_mut(&u_id) {
                                let speed = self
                                    .registry
                                    .get_unit(&u.unit_type)
                                    .map(|ub| ub.move_speed)
                                    .unwrap_or(3.0);
                                u.pos += dir * (speed * 0.05);
                                u.rotation_yaw = dir.y.atan2(dir.x);
                            }
                        } else if target_b.hp < target_b.max_hp
                            && team_idx < 2
                            && self.team_resources[team_idx] >= economy.repair_cost_per_tick
                        {
                            self.team_resources[team_idx] -= economy.repair_cost_per_tick;
                            target_b.hp = (target_b.hp + economy.repair_hp_per_tick)
                                .min(target_b.max_hp);
                            if target_b.hp >= target_b.max_hp {
                                finished = true;
                            }
                        } else {
                            finished = true;
                        }
                    } else {
                        finished = true;
                    }

                    if finished {
                        if let Some(u) = self.units.get_mut(&u_id) {
                            u.order = UnitOrderState::Idle;
                        }
                    }
                }

                UnitOrderState::Harvesting {
                    mine_id,
                    returning_to_base,
                } => {
                    if returning_to_base {
                        // Find closest friendly completed dropoff building
                        let closest_dropoff = self
                            .buildings
                            .values()
                            .filter(|b| {
                                b.team == u_team
                                    && b.completed
                                    && self
                                        .registry
                                        .get_building(&b.building_type)
                                        .map(|bb| bb.is_resource_dropoff)
                                        .unwrap_or(false)
                            })
                            .min_by(|a, b| {
                                let da = (a.pos - u_pos).length_squared();
                                let db = (b.pos - u_pos).length_squared();
                                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|b| (b.pos, self.registry.get_building(&b.building_type).map(|bb| bb.footprint_radius).unwrap_or(2.5)));

                        if let Some((dropoff_pos, footprint)) = closest_dropoff {
                            let dist = (u_pos - dropoff_pos).length();
                            if dist > footprint + 2.0 {
                                let dir = (dropoff_pos - u_pos).normalize_or_zero();
                                if let Some(u) = self.units.get_mut(&u_id) {
                                    let speed = self
                                        .registry
                                        .get_unit(&u.unit_type)
                                        .map(|ub| ub.move_speed)
                                        .unwrap_or(3.0);
                                    u.pos += dir * (speed * 0.05);
                                    u.rotation_yaw = dir.y.atan2(dir.x);
                                }
                            } else {
                                // Deposit
                                if let Some(u) = self.units.get_mut(&u_id) {
                                    let team_idx = u.team as usize;
                                    if team_idx < 2 {
                                        self.team_resources[team_idx] += u.carrying_resources;
                                    }
                                    u.carrying_resources = 0;
                                    u.order = UnitOrderState::Harvesting {
                                        mine_id,
                                        returning_to_base: false,
                                    };
                                }
                            }
                        } else if let Some(u) = self.units.get_mut(&u_id) {
                            u.order = UnitOrderState::Idle;
                        }
                    } else {
                        // Move to mine and harvest
                        let mine_info = self.buildings.get(&mine_id).map(|b| {
                            (
                                b.pos,
                                b.completed,
                                b.linked_node_id,
                                self.registry
                                    .get_building(&b.building_type)
                                    .map(|bb| bb.footprint_radius)
                                    .unwrap_or(2.2),
                            )
                        });

                        if let Some((mine_pos, completed, linked_node_id, footprint)) = mine_info {
                            if !completed {
                                if let Some(u) = self.units.get_mut(&u_id) {
                                    u.order = UnitOrderState::Constructing(mine_id);
                                }
                            } else {
                                let dist = (u_pos - mine_pos).length();
                                if dist > footprint + 2.0 {
                                    let dir = (mine_pos - u_pos).normalize_or_zero();
                                    if let Some(u) = self.units.get_mut(&u_id) {
                                        let speed = self
                                            .registry
                                            .get_unit(&u.unit_type)
                                            .map(|ub| ub.move_speed)
                                            .unwrap_or(3.0);
                                        u.pos += dir * (speed * 0.05);
                                        u.rotation_yaw = dir.y.atan2(dir.x);
                                    }
                                } else if let Some(nid) = linked_node_id {
                                    let mut should_return = false;

                                    if let Some(node) = self.resource_nodes.get_mut(&nid) {
                                        if node.remaining > 0 {
                                            if let Some(u) = self.units.get_mut(&u_id) {
                                                u.harvest_progress += 1;
                                                if u.harvest_progress >= economy.harvest_ticks {
                                                    u.harvest_progress = 0;
                                                    let harvested =
                                                        economy.harvest_amount_per_trip.min(node.remaining);
                                                    node.remaining -= harvested;
                                                    u.carrying_resources = harvested;
                                                    should_return = true;
                                                }
                                            }
                                        } else if let Some(u) = self.units.get_mut(&u_id) {
                                            u.order = UnitOrderState::Idle;
                                        }
                                    }

                                    if should_return {
                                        if let Some(u) = self.units.get_mut(&u_id) {
                                            u.order = UnitOrderState::Harvesting {
                                                mine_id,
                                                returning_to_base: true,
                                            };
                                        }
                                    }
                                }
                            }
                        } else if let Some(u) = self.units.get_mut(&u_id) {
                            u.order = UnitOrderState::Idle;
                        }
                    }
                }

                _ => {}
            }
        }

        // 4. Step Unit Movement & Waypoints
        for unit in self.units.values_mut() {
            if matches!(
                unit.order,
                UnitOrderState::Moving | UnitOrderState::AttackMoving
            ) && !unit.waypoints.is_empty()
            {
                let next_wp = unit.waypoints[0];
                let diff = next_wp - unit.pos;
                let dist = diff.length();
                let speed = self
                    .registry
                    .get_unit(&unit.unit_type)
                    .map(|ub| ub.move_speed)
                    .unwrap_or(2.5);

                if dist < 0.6 {
                    unit.waypoints.remove(0);
                    if unit.waypoints.is_empty() && unit.order == UnitOrderState::Moving {
                        unit.order = UnitOrderState::Idle;
                    }
                } else {
                    let dir = diff / dist;
                    unit.pos += dir * (speed * 0.05);
                    unit.rotation_yaw = dir.y.atan2(dir.x);
                }
            }
        }

        // 5. Local Separation & Rock Structure Collision
        let unit_positions: Vec<(EntityId, Vec2, f32)> = self
            .units
            .values()
            .map(|u| {
                let r = self
                    .registry
                    .get_unit(&u.unit_type)
                    .map(|ub| ub.scale * 0.6)
                    .unwrap_or(0.6);
                (u.id, u.pos, r)
            })
            .collect();

        for (u_id, u_pos, u_r) in &unit_positions {
            let neighbors: Vec<(Vec2, f32)> = unit_positions
                .iter()
                .filter(|(id, _, _)| id != u_id)
                .map(|(_, pos, r)| (*pos, *r))
                .collect();

            let sep = compute_separation(*u_pos, *u_r, &neighbors);
            if let Some(unit) = self.units.get_mut(u_id) {
                unit.pos += sep * 0.05;
                // Push units out of unbroken rock structures
                for rock in self.rocks.values() {
                    let delta = unit.pos - rock.pos;
                    let dist = delta.length();
                    let min_dist = rock.radius + *u_r;
                    if dist < min_dist {
                        let push_dir = if dist > 0.001 {
                            delta / dist
                        } else {
                            Vec2::X
                        };
                        unit.pos = rock.pos + push_dir * min_dist;
                    }
                }
                unit.pos.x = unit.pos.x.clamp(1.0, self.map.width - 1.0);
                unit.pos.y = unit.pos.y.clamp(1.0, self.map.height - 1.0);
            }
        }

        // 6. Combat and Targeting (Units, Buildings, and Breakable Rock Structures)
        let mut attacks_to_apply: Vec<(EntityId, f32, CombatTargetKind, EntityId)> = Vec::new();
        let mut new_projectiles = Vec::new();
        let combat_u_keys: Vec<EntityId> = self.units.keys().copied().collect();

        for u_id in combat_u_keys {
            let (unit_team, unit_pos, unit_type, mut unit_order, mut attack_cd) =
                match self.units.get(&u_id) {
                    Some(u) => (
                        u.team,
                        u.pos,
                        u.unit_type.clone(),
                        u.order,
                        u.attack_cooldown,
                    ),
                    None => continue,
                };

            if attack_cd > 0 {
                attack_cd -= 1;
            }

            let unit_bp = match self.registry.get_unit(&unit_type).cloned() {
                Some(ub) => ub,
                None => {
                    if let Some(u) = self.units.get_mut(&u_id) {
                        u.attack_cooldown = attack_cd;
                    }
                    continue;
                }
            };

            if unit_bp.attack_damage <= 0.0 {
                if let Some(u) = self.units.get_mut(&u_id) {
                    u.attack_cooldown = attack_cd;
                }
                continue;
            }

            // (kind, id, pos, armor, target_radius)
            let mut current_target: Option<(CombatTargetKind, EntityId, Vec2, f32, f32)> = None;

            match unit_order {
                UnitOrderState::Attacking(target_id) => {
                    if let Some(tu) = self.units.get(&target_id) {
                        if tu.team != unit_team && self.fog.is_pos_visible(unit_team, tu.pos) {
                            let arm = self
                                .registry
                                .get_unit(&tu.unit_type)
                                .map(|ub| ub.armor)
                                .unwrap_or(0.0);
                            current_target =
                                Some((CombatTargetKind::Unit, tu.id, tu.pos, arm, 0.0));
                        }
                    } else if let Some(tb) = self.buildings.get(&target_id) {
                        if tb.team != unit_team && self.fog.is_pos_visible(unit_team, tb.pos) {
                            let arm = self
                                .registry
                                .get_building(&tb.building_type)
                                .map(|bb| bb.armor)
                                .unwrap_or(0.0);
                            let radius = self
                                .registry
                                .get_building(&tb.building_type)
                                .map(|bb| bb.footprint_radius)
                                .unwrap_or(2.0);
                            current_target =
                                Some((CombatTargetKind::Building, tb.id, tb.pos, arm, radius));
                        }
                    } else if let Some(trock) = self.rocks.get(&target_id) {
                        current_target = Some((
                            CombatTargetKind::Rock,
                            trock.id,
                            trock.pos,
                            1.0,
                            trock.radius,
                        ));
                    }

                    if current_target.is_none() {
                        unit_order = UnitOrderState::Idle;
                    }
                }

                UnitOrderState::AttackMoving | UnitOrderState::Idle => {
                    // Acquire closest visible enemy within vision range
                    let mut closest_d_sq = unit_bp.vision_range * unit_bp.vision_range;

                    for enemy_u in self.units.values() {
                        if enemy_u.team != unit_team
                            && self.fog.is_pos_visible(unit_team, enemy_u.pos)
                        {
                            let d_sq = (enemy_u.pos - unit_pos).length_squared();
                            if d_sq <= closest_d_sq {
                                closest_d_sq = d_sq;
                                let arm = self
                                    .registry
                                    .get_unit(&enemy_u.unit_type)
                                    .map(|ub| ub.armor)
                                    .unwrap_or(0.0);
                                current_target = Some((
                                    CombatTargetKind::Unit,
                                    enemy_u.id,
                                    enemy_u.pos,
                                    arm,
                                    0.0,
                                ));
                            }
                        }
                    }

                    for enemy_b in self.buildings.values() {
                        if enemy_b.team != unit_team
                            && self.fog.is_pos_visible(unit_team, enemy_b.pos)
                        {
                            let d_sq = (enemy_b.pos - unit_pos).length_squared();
                            if d_sq <= closest_d_sq {
                                closest_d_sq = d_sq;
                                let arm = self
                                    .registry
                                    .get_building(&enemy_b.building_type)
                                    .map(|bb| bb.armor)
                                    .unwrap_or(0.0);
                                let radius = self
                                    .registry
                                    .get_building(&enemy_b.building_type)
                                    .map(|bb| bb.footprint_radius)
                                    .unwrap_or(2.0);
                                current_target = Some((
                                    CombatTargetKind::Building,
                                    enemy_b.id,
                                    enemy_b.pos,
                                    arm,
                                    radius,
                                ));
                            }
                        }
                    }

                    // If AttackMoving and no enemy is in sight, break up any rock structure blocking the path closely
                    if current_target.is_none() && unit_order == UnitOrderState::AttackMoving {
                        let mut closest_rock_edge = 3.5_f32;
                        for rock in self.rocks.values() {
                            let edge_dist = (rock.pos - unit_pos).length() - rock.radius;
                            if edge_dist <= closest_rock_edge {
                                closest_rock_edge = edge_dist;
                                current_target = Some((
                                    CombatTargetKind::Rock,
                                    rock.id,
                                    rock.pos,
                                    1.0,
                                    rock.radius,
                                ));
                            }
                        }
                    }
                }

                _ => {}
            }

            let mut new_pos = unit_pos;
            let mut new_yaw = None;

            if let Some((target_kind, target_id, target_pos, target_armor, target_radius)) =
                current_target
            {
                let center_dist = (unit_pos - target_pos).length();
                let surface_dist = (center_dist - target_radius).max(0.0);

                if surface_dist <= unit_bp.attack_range + 0.6 {
                    new_yaw = Some((target_pos.y - unit_pos.y).atan2(target_pos.x - unit_pos.x));

                    if attack_cd == 0 {
                        attack_cd = unit_bp.attack_cooldown_ticks;

                        let mut dmg = if let Some(df) = damage_fn {
                            df(unit_bp.attack_damage, target_armor, unit_bp.armor_piercing)
                        } else {
                            let eff = (target_armor - unit_bp.armor_piercing).max(0.0);
                            (unit_bp.attack_damage - eff).max(1.0)
                        };

                        // Worker Cats have mining drills/picks that break rock structures 3.5x faster
                        if target_kind == CombatTargetKind::Rock
                            && (unit_bp.can_harvest || unit_bp.id == "worker")
                        {
                            dmg *= 3.5;
                        }

                        attacks_to_apply.push((u_id, dmg, target_kind, target_id));

                        if unit_bp.projectile_speed > 0.0 {
                            let start = Vec3::new(unit_pos.x, 0.8, unit_pos.y);
                            let end = Vec3::new(target_pos.x, 0.8, target_pos.y);
                            let ttl = ((center_dist / (unit_bp.projectile_speed * 0.05)).ceil()
                                as u32)
                                .max(1);
                            new_projectiles.push(ProjectileVisual {
                                start,
                                end,
                                pos: start,
                                speed: unit_bp.projectile_speed,
                                team: unit_team,
                                ttl_ticks: ttl,
                            });
                        }
                    }
                } else if unit_order == UnitOrderState::Attacking(target_id)
                    || unit_order == UnitOrderState::AttackMoving
                {
                    let dir = (target_pos - unit_pos).normalize_or_zero();
                    new_pos = unit_pos + dir * (unit_bp.move_speed * 0.05);
                    new_yaw = Some(dir.y.atan2(dir.x));
                }
            }

            if let Some(u) = self.units.get_mut(&u_id) {
                u.order = unit_order;
                u.attack_cooldown = attack_cd;
                u.pos = new_pos;
                if let Some(yaw) = new_yaw {
                    u.rotation_yaw = yaw;
                }
            }
        }

        // 6b. Defensive Turret / Building Automated Combat
        let combat_b_keys: Vec<EntityId> = self.buildings.keys().copied().collect();
        for b_id in combat_b_keys {
            let (b_team, b_pos, b_type, b_completed, mut b_cd) = match self.buildings.get(&b_id) {
                Some(b) => (
                    b.team,
                    b.pos,
                    b.building_type.clone(),
                    b.completed,
                    b.attack_cooldown,
                ),
                None => continue,
            };

            if !b_completed {
                continue;
            }

            if b_cd > 0 {
                b_cd -= 1;
            }

            let b_bp = match self.registry.get_building(&b_type).cloned() {
                Some(bp) if bp.attack_damage > 0.0 && bp.attack_range > 0.0 => bp,
                _ => {
                    if let Some(b) = self.buildings.get_mut(&b_id) {
                        b.attack_cooldown = b_cd;
                    }
                    continue;
                }
            };

            // Acquire closest visible enemy unit within attack range (or visible enemy building if no unit)
            let mut best_target: Option<(CombatTargetKind, EntityId, Vec2, f32)> = None;
            let mut closest_unit_dist = b_bp.attack_range + 0.6;

            for enemy_u in self.units.values() {
                if enemy_u.team != b_team && self.fog.is_pos_visible(b_team, enemy_u.pos) {
                    let dist = (enemy_u.pos - b_pos).length();
                    if dist <= closest_unit_dist {
                        closest_unit_dist = dist;
                        let arm = self
                            .registry
                            .get_unit(&enemy_u.unit_type)
                            .map(|ub| ub.armor)
                            .unwrap_or(0.0);
                        best_target =
                            Some((CombatTargetKind::Unit, enemy_u.id, enemy_u.pos, arm));
                    }
                }
            }

            if best_target.is_none() {
                let mut closest_b_surface = b_bp.attack_range + 0.6;
                for enemy_b in self.buildings.values() {
                    if enemy_b.team != b_team && self.fog.is_pos_visible(b_team, enemy_b.pos) {
                        let e_radius = self
                            .registry
                            .get_building(&enemy_b.building_type)
                            .map(|eb| eb.footprint_radius)
                            .unwrap_or(2.0);
                        let surface_dist = ((enemy_b.pos - b_pos).length() - e_radius).max(0.0);
                        if surface_dist <= closest_b_surface {
                            closest_b_surface = surface_dist;
                            let arm = self
                                .registry
                                .get_building(&enemy_b.building_type)
                                .map(|eb| eb.armor)
                                .unwrap_or(0.0);
                            best_target = Some((
                                CombatTargetKind::Building,
                                enemy_b.id,
                                enemy_b.pos,
                                arm,
                            ));
                        }
                    }
                }
            }

            let mut new_b_yaw = None;
            if let Some((target_kind, target_id, target_pos, target_armor)) = best_target {
                new_b_yaw = Some((target_pos.y - b_pos.y).atan2(target_pos.x - b_pos.x));

                if b_cd == 0 {
                    b_cd = b_bp.attack_cooldown_ticks.max(1);

                    let dmg = if let Some(df) = damage_fn {
                        df(b_bp.attack_damage, target_armor, b_bp.armor_piercing)
                    } else {
                        let eff = (target_armor - b_bp.armor_piercing).max(0.0);
                        (b_bp.attack_damage - eff).max(1.0)
                    };

                    attacks_to_apply.push((b_id, dmg, target_kind, target_id));

                    let proj_speed = if b_bp.projectile_speed > 0.0 {
                        b_bp.projectile_speed
                    } else {
                        32.0
                    };
                    let center_dist = (target_pos - b_pos).length();
                    let start = Vec3::new(b_pos.x, 1.5, b_pos.y);
                    let end = Vec3::new(target_pos.x, 0.8, target_pos.y);
                    let ttl = ((center_dist / (proj_speed * 0.05)).ceil() as u32).max(1);
                    new_projectiles.push(ProjectileVisual {
                        start,
                        end,
                        pos: start,
                        speed: proj_speed,
                        team: b_team,
                        ttl_ticks: ttl,
                    });
                }
            }

            if let Some(b) = self.buildings.get_mut(&b_id) {
                b.attack_cooldown = b_cd;
                if let Some(yaw) = new_b_yaw {
                    b.rotation_yaw = yaw;
                }
            }
        }

        // Apply combat & rock-breaking damage
        for (_attacker_id, dmg, target_kind, target_id) in attacks_to_apply {
            match target_kind {
                CombatTargetKind::Unit => {
                    if let Some(target_u) = self.units.get_mut(&target_id) {
                        target_u.hp -= dmg;
                    }
                }
                CombatTargetKind::Building => {
                    if let Some(target_b) = self.buildings.get_mut(&target_id) {
                        target_b.hp -= dmg;
                    }
                }
                CombatTargetKind::Rock => {
                    if let Some(target_r) = self.rocks.get_mut(&target_id) {
                        target_r.hp -= dmg;
                    }
                }
            }
        }

        // 7. Step Projectiles
        self.projectiles.extend(new_projectiles);
        for proj in &mut self.projectiles {
            if proj.ttl_ticks > 0 {
                proj.ttl_ticks -= 1;
            }
            let dir = proj.end - proj.pos;
            let dist = dir.length();
            let step = proj.speed * 0.05;
            if dist <= step {
                proj.pos = proj.end;
            } else {
                proj.pos += dir.normalize_or_zero() * step;
            }
        }
        self.projectiles.retain(|p| p.ttl_ticks > 0);

        // 8. Remove dead units, destroyed buildings, and broken rock structures
        self.units.retain(|_, u| u.hp > 0.0);
        self.rocks.retain(|_, r| r.hp > 0.0);

        let destroyed_buildings: Vec<(EntityId, Option<EntityId>)> = self
            .buildings
            .iter()
            .filter(|(_, b)| b.hp <= 0.0)
            .map(|(&id, b)| (id, b.linked_node_id))
            .collect();

        for (b_id, linked_node) in destroyed_buildings {
            if let Some(node_id) = linked_node {
                if let Some(node) = self.resource_nodes.get_mut(&node_id) {
                    if node.occupied_mine_id == Some(b_id) {
                        node.occupied_mine_id = None;
                    }
                }
            }
        }
        self.buildings.retain(|_, b| b.hp > 0.0);

        // 9. Recompute Supplies
        self.recompute_supplies();

        // 10. Check Victory Condition
        let count_t0 = self.buildings.values().filter(|b| b.team == 0).count();
        let count_t1 = self.buildings.values().filter(|b| b.team == 1).count();

        if count_t0 == 0 && count_t1 > 0 {
            self.winner = Some(1);
        } else if count_t1 == 0 && count_t0 > 0 {
            self.winner = Some(0);
        }
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut hash: u32 = 0x811c9dc5;
        let fnv_prime: u32 = 0x01000193;

        let mut feed_u32 = |val: u32| {
            for b in val.to_le_bytes() {
                hash ^= b as u32;
                hash = hash.wrapping_mul(fnv_prime);
            }
        };

        feed_u32(self.tick as u32);
        feed_u32((self.tick >> 32) as u32);
        feed_u32(self.team_resources[0]);
        feed_u32(self.team_resources[1]);
        feed_u32(self.team_supply_used[0]);
        feed_u32(self.team_supply_used[1]);
        feed_u32(self.team_supply_cap[0]);
        feed_u32(self.team_supply_cap[1]);
        feed_u32(if self.paused { 1 } else { 0 });

        for (&id, unit) in &self.units {
            feed_u32(id);
            feed_u32(unit.team as u32);
            feed_u32((unit.pos.x * 100.0).round() as i32 as u32);
            feed_u32((unit.pos.y * 100.0).round() as i32 as u32);
            feed_u32((unit.hp * 10.0).round() as i32 as u32);
        }

        for (&id, b) in &self.buildings {
            feed_u32(id);
            feed_u32(b.team as u32);
            feed_u32((b.hp * 10.0).round() as i32 as u32);
            feed_u32(if b.completed { 1 } else { 0 });
        }

        for (&id, r) in &self.rocks {
            feed_u32(id);
            feed_u32((r.hp * 10.0).round() as i32 as u32);
        }

        hash
    }

    pub fn build_render_instances(
        &self,
        viewer_team: TeamId,
        selected_ids: &[EntityId],
    ) -> Vec<RenderInstance> {
        let mut instances = Vec::new();

        // 1. Breakable Rock Structures
        for rock in self.rocks.values() {
            let hp_ratio = (rock.hp / rock.max_hp.max(1.0)).clamp(0.05, 1.0);
            let height_scale = (rock.height * 0.45).max(0.8) * (0.55 + 0.45 * hp_ratio);
            let color = if rock.radius >= 6.5 {
                // Imposing central granite/basalt massif
                [0.48, 0.44, 0.40, 0.12]
            } else {
                [0.42, 0.45, 0.48, 0.0]
            };
            instances.push(RenderInstance {
                position: Vec3::new(rock.pos.x, 0.0, rock.pos.y),
                scale: Vec3::new(rock.radius * 0.75, height_scale, rock.radius * 0.75),
                rotation_yaw: 0.0,
                color,
                mesh_archetype: "obstacle".to_string(),
                selected: selected_ids.contains(&rock.id),
                hp_ratio,
                progress_ratio: None,
            });
        }

        // 2. Resource nodes
        for node in self.resource_nodes.values() {
            instances.push(RenderInstance {
                position: Vec3::new(node.pos.x, 0.0, node.pos.y),
                scale: Vec3::new(1.25, 1.25, 1.25),
                rotation_yaw: 0.0,
                color: [0.2, 0.90, 1.0, 0.0],
                mesh_archetype: "resource_node".to_string(),
                selected: selected_ids.contains(&node.id),
                hp_ratio: (node.remaining as f32 / 2000.0).clamp(0.1, 1.0),
                progress_ratio: None,
            });
        }

        // 3. Buildings
        for b in self.buildings.values() {
            if viewer_team != TEAM_NEUTRAL
                && b.team != viewer_team
                && self.fog.tile_state(viewer_team, b.pos) == FogTileState::Unexplored
            {
                continue;
            }

            let bp = self.registry.get_building(&b.building_type);
            let mesh = bp
                .map(|x| x.mesh_archetype.clone())
                .unwrap_or_else(|| b.building_type.clone());

            let team_color = if b.team == 0 {
                [0.2, 0.55, 1.0, 0.14]
            } else {
                [0.95, 0.25, 0.25, 0.14]
            };

            let progress = if !b.completed {
                Some(b.build_progress_ticks as f32 / b.build_total_ticks.max(1) as f32)
            } else if !b.production_queue.is_empty() {
                let total = self
                    .registry
                    .get_unit(&b.production_queue[0])
                    .map(|u| u.build_time_ticks)
                    .unwrap_or(100);
                Some(b.production_progress_ticks as f32 / total.max(1) as f32)
            } else {
                None
            };

            instances.push(RenderInstance {
                position: Vec3::new(b.pos.x, 0.0, b.pos.y),
                scale: Vec3::new(1.0, 1.0, 1.0),
                rotation_yaw: b.rotation_yaw,
                color: team_color,
                mesh_archetype: mesh,
                selected: selected_ids.contains(&b.id),
                hp_ratio: (b.hp / b.max_hp.max(1.0)).clamp(0.0, 1.0),
                progress_ratio: progress,
            });
        }

        // 4. Units (strictly hiding enemy units not visible in Fog!)
        for u in self.units.values() {
            if viewer_team != TEAM_NEUTRAL
                && u.team != viewer_team
                && !self.fog.is_pos_visible(viewer_team, u.pos)
            {
                continue;
            }

            let bp = self.registry.get_unit(&u.unit_type);
            let scale = bp.map(|x| x.scale).unwrap_or(1.0) * 1.55;
            let mesh = bp
                .map(|x| x.mesh_archetype.clone())
                .unwrap_or_else(|| u.unit_type.clone());

            let team_color = if u.team == 0 {
                [0.25, 0.65, 1.0, 0.08]
            } else {
                [1.0, 0.30, 0.30, 0.08]
            };

            let progress = if u.carrying_resources > 0 {
                let max_harvest = self.registry.economy.harvest_amount_per_trip.max(1);
                Some(u.carrying_resources as f32 / max_harvest as f32)
            } else {
                None
            };

            instances.push(RenderInstance {
                position: Vec3::new(u.pos.x, 0.0, u.pos.y),
                scale: Vec3::new(scale, scale, scale),
                rotation_yaw: u.rotation_yaw,
                color: team_color,
                mesh_archetype: mesh,
                selected: selected_ids.contains(&u.id),
                hp_ratio: (u.hp / u.max_hp.max(1.0)).clamp(0.0, 1.0),
                progress_ratio: progress,
            });
        }

        // 5. Projectiles
        for proj in &self.projectiles {
            instances.push(RenderInstance {
                position: proj.pos,
                scale: Vec3::new(0.3, 0.3, 0.7),
                rotation_yaw: 0.0,
                color: [1.0, 0.9, 0.2, 1.0],
                mesh_archetype: "projectile".to_string(),
                selected: false,
                hp_ratio: 1.0,
                progress_ratio: None,
            });
        }

        instances
    }
}
