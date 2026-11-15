use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

pub type EntityId = u32;
pub type TeamId = u8;

pub const TEAM_PLAYER_1: TeamId = 0;
pub const TEAM_PLAYER_2: TeamId = 1;
pub const TEAM_NEUTRAL: TeamId = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiDifficulty {
    Easy = 1,
    Medium = 2,
    Hard = 3,
}

impl AiDifficulty {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn label(self) -> &'static str {
        match self {
            AiDifficulty::Easy => "Easy",
            AiDifficulty::Medium => "Medium",
            AiDifficulty::Hard => "Hard",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FogTileState {
    Unexplored = 0,
    Explored = 1,
    Visible = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitBlueprint {
    pub id: String,
    pub display_name: String,
    pub mesh_archetype: String,
    pub scale: f32,
    pub cost: u32,
    pub supply_cost: u32,
    pub build_time_ticks: u32,
    pub max_hp: f32,
    pub armor: f32,
    pub armor_piercing: f32,
    pub move_speed: f32,
    pub vision_range: f32,
    pub attack_damage: f32,
    pub attack_range: f32,
    pub attack_cooldown_ticks: u32,
    pub projectile_speed: f32,
    pub can_harvest: bool,
    pub can_build: bool,
    pub can_repair: bool,
    pub producer_building: String,
    pub hotkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingBlueprint {
    pub id: String,
    pub display_name: String,
    pub mesh_archetype: String,
    pub footprint_radius: f32,
    pub cost: u32,
    pub build_time_ticks: u32,
    pub max_hp: f32,
    pub armor: f32,
    pub vision_range: f32,
    pub supply_provided: u32,
    pub requires_resource_node: bool,
    pub is_resource_dropoff: bool,
    pub is_mine_extractor: bool,
    pub trainable_units: Vec<String>,
    pub hotkey: String,
    #[serde(default)]
    pub attack_damage: f32,
    #[serde(default)]
    pub attack_range: f32,
    #[serde(default)]
    pub attack_cooldown_ticks: u32,
    #[serde(default)]
    pub armor_piercing: f32,
    #[serde(default)]
    pub projectile_speed: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNodeSpawn {
    pub position: [f32; 2],
    pub amount: u32,
    pub side_hint: TeamId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleSpawn {
    pub position: [f32; 2],
    pub radius: f32,
    pub height: f32,
    #[serde(default)]
    pub max_hp: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub width: f32,
    pub height: f32,
    pub team_spawns: [[f32; 2]; 2],
    pub resource_nodes: Vec<ResourceNodeSpawn>,
    pub obstacles: Vec<ObstacleSpawn>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameCommand {
    MoveUnits {
        team: TeamId,
        unit_ids: Vec<EntityId>,
        target: [f32; 2],
        attack_move: bool,
    },
    AttackTarget {
        team: TeamId,
        unit_ids: Vec<EntityId>,
        target_id: EntityId,
    },
    ConstructBuilding {
        team: TeamId,
        worker_ids: Vec<EntityId>,
        building_type: String,
        position: [f32; 2],
        resource_node_id: Option<EntityId>,
    },
    RepairBuilding {
        team: TeamId,
        worker_ids: Vec<EntityId>,
        building_id: EntityId,
    },
    HarvestMine {
        team: TeamId,
        worker_ids: Vec<EntityId>,
        target_id: EntityId,
    },
    TrainUnit {
        team: TeamId,
        building_id: EntityId,
        unit_type: String,
    },
    StopUnits {
        team: TeamId,
        unit_ids: Vec<EntityId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitOrderState {
    Idle,
    Moving,
    AttackMoving,
    Attacking(EntityId),
    Constructing(EntityId),
    Repairing(EntityId),
    Harvesting {
        mine_id: EntityId,
        returning_to_base: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderInstance {
    pub position: Vec3,
    pub scale: Vec3,
    pub rotation_yaw: f32,
    pub color: [f32; 4],
    pub mesh_archetype: String,
    pub selected: bool,
    pub hp_ratio: f32,
    pub progress_ratio: Option<f32>,
}

pub fn vec2_from_arr(a: [f32; 2]) -> Vec2 {
    Vec2::new(a[0], a[1])
}
