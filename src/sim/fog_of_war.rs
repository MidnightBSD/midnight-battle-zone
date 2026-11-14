use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::types::{FogTileState, TeamId};

pub const DEFAULT_FOG_CELL_SIZE: f32 = 2.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FogOfWar {
    pub width_cells: usize,
    pub height_cells: usize,
    pub cell_size: f32,
    pub grids: [Vec<FogTileState>; 2],
}

impl FogOfWar {
    pub fn new(map_width: f32, map_height: f32, cell_size: f32) -> Self {
        let actual_cell_size = if cell_size > 0.0 { cell_size } else { DEFAULT_FOG_CELL_SIZE };
        let width_cells = ((map_width / actual_cell_size).ceil() as usize).max(1);
        let height_cells = ((map_height / actual_cell_size).ceil() as usize).max(1);
        let total_cells = width_cells * height_cells;

        Self {
            width_cells,
            height_cells,
            cell_size: actual_cell_size,
            grids: [
                vec![FogTileState::Unexplored; total_cells],
                vec![FogTileState::Unexplored; total_cells],
            ],
        }
    }

    pub fn width_cells(&self) -> usize {
        self.width_cells
    }

    pub fn height_cells(&self) -> usize {
        self.height_cells
    }

    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    pub fn begin_tick(&mut self) {
        for team_idx in 0..self.grids.len() {
            for state in self.grids[team_idx].iter_mut() {
                if *state == FogTileState::Visible {
                    *state = FogTileState::Explored;
                }
            }
        }
    }

    pub fn reveal_circle(&mut self, team: TeamId, pos: Vec2, radius: f32) {
        let team_idx = team as usize;
        if team_idx >= self.grids.len() || radius <= 0.0 {
            return;
        }

        let min_cx = ((pos.x - radius) / self.cell_size).floor().max(0.0) as usize;
        let max_cx = (((pos.x + radius) / self.cell_size).ceil().max(0.0) as usize)
            .min(self.width_cells.saturating_sub(1));
        let min_cy = ((pos.y - radius) / self.cell_size).floor().max(0.0) as usize;
        let max_cy = (((pos.y + radius) / self.cell_size).ceil().max(0.0) as usize)
            .min(self.height_cells.saturating_sub(1));

        let r2 = radius * radius;

        for cy in min_cy..=max_cy {
            let y_min = cy as f32 * self.cell_size;
            let y_max = (cy + 1) as f32 * self.cell_size;
            let clamped_y = pos.y.clamp(y_min, y_max);
            let dy = pos.y - clamped_y;
            let dy2 = dy * dy;

            for cx in min_cx..=max_cx {
                let x_min = cx as f32 * self.cell_size;
                let x_max = (cx + 1) as f32 * self.cell_size;
                let clamped_x = pos.x.clamp(x_min, x_max);
                let dx = pos.x - clamped_x;

                if dx * dx + dy2 <= r2 {
                    let idx = cy * self.width_cells + cx;
                    self.grids[team_idx][idx] = FogTileState::Visible;
                }
            }
        }
    }

    pub fn tile_state(&self, team: TeamId, world_pos: Vec2) -> FogTileState {
        let team_idx = team as usize;
        if team_idx >= self.grids.len() {
            return FogTileState::Visible;
        }

        let cx = (world_pos.x / self.cell_size).floor() as isize;
        let cy = (world_pos.y / self.cell_size).floor() as isize;

        if cx < 0 || cy < 0 || cx >= self.width_cells as isize || cy >= self.height_cells as isize {
            return FogTileState::Unexplored;
        }

        let idx = (cy as usize) * self.width_cells + (cx as usize);
        self.grids[team_idx][idx]
    }

    pub fn is_pos_visible(&self, viewer_team: TeamId, world_pos: Vec2) -> bool {
        let team_idx = viewer_team as usize;
        if team_idx >= self.grids.len() {
            return true;
        }
        self.tile_state(viewer_team, world_pos) == FogTileState::Visible
    }

    pub fn rgba_texture_data(&self, viewer_team: TeamId) -> Vec<u8> {
        let total_cells = self.width_cells * self.height_cells;
        let mut buffer = Vec::with_capacity(total_cells * 4);
        let team_idx = viewer_team as usize;

        if team_idx < self.grids.len() {
            for &state in &self.grids[team_idx] {
                let val = match state {
                    FogTileState::Unexplored => 105u8,
                    FogTileState::Explored => 200u8,
                    FogTileState::Visible => 255u8,
                };
                buffer.extend_from_slice(&[val, val, val, 255]);
            }
        } else {
            for _ in 0..total_cells {
                buffer.extend_from_slice(&[255, 255, 255, 255]);
            }
        }

        buffer
    }
}
