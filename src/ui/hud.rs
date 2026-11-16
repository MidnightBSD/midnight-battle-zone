use glam::Vec2;

use crate::platform::x11::{InputState, KeyCode};
use crate::renderer::camera::IsometricCamera;
use crate::renderer::meshes::{
    append_rect, append_rect_outline, append_text_quads, append_triangle, Vertex2D,
};
use crate::renderer::FsrMode;
use crate::types::{AiDifficulty, EntityId, GameCommand, RenderInstance, TeamId, TEAM_PLAYER_1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenMode {
    MainMenu,
    InGame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    SingleplayerAi,
    HostLan,
    JoinLan,
}

impl GameMode {
    pub fn label(self) -> &'static str {
        match self {
            GameMode::SingleplayerAi => "Singleplayer vs AI",
            GameMode::HostLan => "Host LAN Game",
            GameMode::JoinLan => "Join LAN Game",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MapOption {
    pub id: String,
    pub name: String,
    pub description: String,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct LanLobbyInfo {
    pub host_name: String,
    pub map_name: String,
    pub ip_addr: String,
    pub ping_ms: u32,
}

#[derive(Debug, Clone)]
pub struct SelectedUnitInfo {
    pub id: EntityId,
    pub name: String,
    pub archetype: String,
    pub hp: f32,
    pub max_hp: f32,
    pub armor: f32,
    pub damage: f32,
    pub order_desc: String,
    pub production_progress: Option<f32>,
    pub is_building: bool,
    pub is_worker: bool,
    pub is_combat: bool,
    pub is_barracks: bool,
    pub is_command_center: bool,
}

#[derive(Debug, Clone)]
pub enum UiAction {
    StartGame {
        mode: GameMode,
        difficulty: AiDifficulty,
        map_id: String,
    },
    ConnectLan {
        ip: String,
    },
    IssueCommand(GameCommand),
    SetPlacementMode(Option<String>),
    MoveCameraToGround(Vec2),
    TogglePause,
    ToggleMenu,
    ReturnToMenu,
    SetFsrMode(FsrMode),
    CycleMusic,
}

#[derive(Debug, Clone)]
pub struct MeowCallout {
    pub world_pos: glam::Vec3,
    pub text: String,
    pub ttl: f32,
}

pub struct UiState {
    pub current_screen: ScreenMode,
    pub local_team: TeamId,

    // Main Menu State
    pub selected_mode: GameMode,
    pub selected_difficulty: AiDifficulty,
    pub selected_map_idx: usize,
    pub maps: Vec<MapOption>,
    pub lan_ip_input: String,
    pub ip_input_focused: bool,
    pub discovered_lobbies: Vec<LanLobbyInfo>,
    pub selected_lobby_idx: Option<usize>,

    // In-Game State
    pub resources: u32,
    pub supply_used: u32,
    pub supply_cap: u32,
    pub fps: u32,
    pub elapsed_seconds: f32,
    pub game_ticks: u64,
    pub paused: bool,
    pub game_over: Option<bool>, // Some(true) = Victory, Some(false) = Defeat

    // AMD FSR 1.0 (EASU + RCAS) & Music
    pub fsr_mode: FsrMode,
    pub music_label: String,
    pub meow_callouts: Vec<MeowCallout>,

    pub selected_unit: Option<SelectedUnitInfo>,
    pub selected_unit_ids: Vec<EntityId>,
    pub placement_mode: Option<String>,
    pub attack_move_mode: bool,

    // Box-selection
    pub box_drag_start: Option<Vec2>,
    pub box_drag_current: Option<Vec2>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            current_screen: ScreenMode::MainMenu,
            local_team: TEAM_PLAYER_1,
            selected_mode: GameMode::SingleplayerAi,
            selected_difficulty: AiDifficulty::Medium,
            selected_map_idx: 0,
            maps: vec![
                MapOption {
                    id: "midnight_valley".to_string(),
                    name: "Midnight Valley [4 Mines/Side]".to_string(),
                    description: "Balanced canyon valley with 4 mineral nodes per side".to_string(),
                    width: 96.0,
                    height: 96.0,
                },
                MapOption {
                    id: "twin_iron_ridge".to_string(),
                    name: "Twin Iron Ridge [3 Mines/Side]".to_string(),
                    description: "Dual bridge crossings with 3 mineral nodes per side".to_string(),
                    width: 84.0,
                    height: 84.0,
                },
                MapOption {
                    id: "shattered_basin".to_string(),
                    name: "Shattered Basin [6 Mines/Side]".to_string(),
                    description: "Expansive macro basin with 6 mineral nodes per side".to_string(),
                    width: 112.0,
                    height: 112.0,
                },
            ],
            lan_ip_input: "127.0.0.1:28765".to_string(),
            ip_input_focused: false,
            discovered_lobbies: Vec::new(),
            selected_lobby_idx: None,

            resources: 300,
            supply_used: 3,
            supply_cap: 10,
            fps: 60,
            elapsed_seconds: 0.0,
            game_ticks: 0,
            paused: false,
            game_over: None,

            fsr_mode: FsrMode::Off,
            music_label: "Off".to_string(),
            meow_callouts: Vec::new(),

            selected_unit: None,
            selected_unit_ids: Vec::new(),
            placement_mode: None,
            attack_move_mode: false,
            box_drag_start: None,
            box_drag_current: None,
        }
    }
}

impl UiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_map(&self) -> &MapOption {
        &self.maps[self.selected_map_idx.min(self.maps.len() - 1)]
    }

    pub fn handle_input(
        &mut self,
        input: &InputState,
        camera: &IsometricCamera,
        viewport_size: Vec2,
        map_size: Vec2,
    ) -> Vec<UiAction> {
        let mut actions = Vec::new();

        // Scale input mouse coordinates from actual window resolution into the 1280x720 reference UI canvas
        let ref_vp = Vec2::new(1280.0, 720.0);
        let sx = (viewport_size.x / ref_vp.x).max(0.01);
        let sy = (viewport_size.y / ref_vp.y).max(0.01);
        let mut scaled_input = input.clone();
        scaled_input.mouse_pos = Vec2::new(input.mouse_pos.x / sx, input.mouse_pos.y / sy);
        scaled_input.drag_start = input.drag_start.map(|d| Vec2::new(d.x / sx, d.y / sy));

        // Global hotkey: P toggles Pause/Resume
        if scaled_input.is_key_pressed(KeyCode::KeyP) && self.current_screen == ScreenMode::InGame {
            self.paused = !self.paused;
            actions.push(UiAction::TogglePause);
        }

        // Global hotkey: F cycles AMD FSR mode
        if scaled_input.is_key_pressed(KeyCode::KeyF) {
            self.fsr_mode = self.fsr_mode.next();
            actions.push(UiAction::SetFsrMode(self.fsr_mode));
        }

        // Global hotkey: J cycles Jazz/Blues Background Music
        if scaled_input.is_key_pressed(KeyCode::KeyJ) {
            actions.push(UiAction::CycleMusic);
        }

        match self.current_screen {
            ScreenMode::MainMenu => {
                self.handle_menu_input(&scaled_input, &mut actions);
            }
            ScreenMode::InGame => {
                self.handle_in_game_input(&scaled_input, camera, ref_vp, map_size, &mut actions);
            }
        }

        actions
    }

    fn handle_menu_input(&mut self, input: &InputState, actions: &mut Vec<UiAction>) {
        let click = input.left_pressed;
        let mouse = input.mouse_pos;

        // Mode buttons
        let modes = [
            (GameMode::SingleplayerAi, [100.0, 180.0, 240.0, 36.0]),
            (GameMode::HostLan, [100.0, 226.0, 240.0, 36.0]),
            (GameMode::JoinLan, [100.0, 272.0, 240.0, 36.0]),
        ];

        for (mode, rect) in modes {
            if click && is_inside(mouse, rect) {
                self.selected_mode = mode;
            }
        }

        // Difficulty buttons (only if Singleplayer)
        if self.selected_mode == GameMode::SingleplayerAi {
            let diffs = [
                (AiDifficulty::Easy, [100.0, 360.0, 90.0, 32.0]),
                (AiDifficulty::Medium, [200.0, 360.0, 90.0, 32.0]),
                (AiDifficulty::Hard, [300.0, 360.0, 90.0, 32.0]),
            ];
            for (diff, rect) in diffs {
                if click && is_inside(mouse, rect) {
                    self.selected_difficulty = diff;
                }
            }
        }

        // Map selection buttons
        for (i, _map) in self.maps.iter().enumerate() {
            let rect = [400.0, 180.0 + (i as f32) * 54.0, 380.0, 44.0];
            if click && is_inside(mouse, rect) {
                self.selected_map_idx = i;
            }
        }

        // IP input box (when Join LAN)
        if self.selected_mode == GameMode::JoinLan {
            let ip_rect = [400.0, 380.0, 380.0, 36.0];
            if click {
                self.ip_input_focused = is_inside(mouse, ip_rect);
            }

            if self.ip_input_focused {
                if input.is_key_pressed(KeyCode::Backspace) {
                    self.lan_ip_input.pop();
                }
                for key in &input.keys_pressed {
                    let ch = match key {
                        KeyCode::Num0 => Some('0'),
                        KeyCode::Num1 => Some('1'),
                        KeyCode::Num2 => Some('2'),
                        KeyCode::Num3 => Some('3'),
                        KeyCode::Num4 => Some('4'),
                        KeyCode::Num5 => Some('5'),
                        KeyCode::Num6 => Some('6'),
                        KeyCode::Num7 => Some('7'),
                        KeyCode::Num8 => Some('8'),
                        KeyCode::Num9 => Some('9'),
                        _ => None,
                    };
                    if let Some(c) = ch {
                        if self.lan_ip_input.len() < 24 {
                            self.lan_ip_input.push(c);
                        }
                    }
                }
            }
        }

        // FSR selector button in Menu
        let fsr_rect = [100.0, 420.0, 260.0, 36.0];
        if click && is_inside(mouse, fsr_rect) {
            self.fsr_mode = self.fsr_mode.next();
            actions.push(UiAction::SetFsrMode(self.fsr_mode));
        }

        // Music selector button in Menu
        let music_rect = [100.0, 468.0, 260.0, 36.0];
        if click && is_inside(mouse, music_rect) {
            actions.push(UiAction::CycleMusic);
        }

        // Start / Connect button
        let start_rect = [400.0, 480.0, 260.0, 52.0];
        if click && is_inside(mouse, start_rect) {
            match self.selected_mode {
                GameMode::SingleplayerAi | GameMode::HostLan => {
                    self.current_screen = ScreenMode::InGame;
                    actions.push(UiAction::StartGame {
                        mode: self.selected_mode,
                        difficulty: self.selected_difficulty,
                        map_id: self.current_map().id.clone(),
                    });
                }
                GameMode::JoinLan => {
                    self.current_screen = ScreenMode::InGame;
                    actions.push(UiAction::ConnectLan {
                        ip: self.lan_ip_input.clone(),
                    });
                }
            }
        }
    }

    fn handle_in_game_input(
        &mut self,
        input: &InputState,
        camera: &IsometricCamera,
        viewport_size: Vec2,
        map_size: Vec2,
        actions: &mut Vec<UiAction>,
    ) {
        let mouse = input.mouse_pos;
        let click = input.left_pressed;
        let right_click = input.right_pressed;

        // Check Top-bar Menu button
        let menu_btn = [15.0, 8.0, 80.0, 30.0];
        if click && is_inside(mouse, menu_btn) {
            self.current_screen = ScreenMode::MainMenu;
            actions.push(UiAction::ToggleMenu);
            return;
        }

        // Check Top-bar Pause button
        let pause_btn = [105.0, 8.0, 110.0, 30.0];
        if click && is_inside(mouse, pause_btn) {
            self.paused = !self.paused;
            actions.push(UiAction::TogglePause);
            return;
        }

        // Check Top-bar FSR button
        let fsr_btn = [225.0, 8.0, 115.0, 30.0];
        if click && is_inside(mouse, fsr_btn) {
            self.fsr_mode = self.fsr_mode.next();
            actions.push(UiAction::SetFsrMode(self.fsr_mode));
            return;
        }

        // Check Game Over return button (matches render_in_game end_rect + ret_btn)
        if self.game_over.is_some() {
            let end_w = 700.0;
            let end_h = 160.0;
            let end_rect = [
                viewport_size.x * 0.5 - end_w * 0.5,
                viewport_size.y * 0.5 - end_h * 0.5,
                end_w,
                end_h,
            ];
            let ret_btn = [
                viewport_size.x * 0.5 - 120.0,
                end_rect[1] + 90.0,
                240.0,
                42.0,
            ];
            if (click && (is_inside(mouse, ret_btn) || is_inside(mouse, end_rect)))
                || input.is_key_pressed(KeyCode::Escape)
                || input.is_key_pressed(KeyCode::Return)
                || input.is_key_pressed(KeyCode::Space)
            {
                self.current_screen = ScreenMode::MainMenu;
                self.game_over = None;
                actions.push(UiAction::ReturnToMenu);
                return;
            }
        }

        // Minimap bounds
        let minimap_size = 190.0;
        let minimap_rect = [
            20.0,
            viewport_size.y - minimap_size - 20.0,
            minimap_size,
            minimap_size,
        ];

        if is_inside(mouse, minimap_rect) {
            let rel_x = (mouse.x - minimap_rect[0]) / minimap_size;
            let rel_y = (mouse.y - minimap_rect[1]) / minimap_size;
            let target_ground = Vec2::new(rel_x * map_size.x, rel_y * map_size.y);

            if click {
                actions.push(UiAction::MoveCameraToGround(target_ground));
                return;
            }

            if right_click && !self.selected_unit_ids.is_empty() {
                actions.push(UiAction::IssueCommand(GameCommand::MoveUnits {
                    team: self.local_team,
                    unit_ids: self.selected_unit_ids.clone(),
                    target: [target_ground.x, target_ground.y],
                    attack_move: self.attack_move_mode,
                }));
                self.attack_move_mode = false;
                return;
            }
        }

        // Command Card buttons (Bottom-right)
        let cmd_rect = [
            viewport_size.x - 360.0,
            viewport_size.y - 190.0,
            340.0,
            170.0,
        ];

        if let Some(unit) = &self.selected_unit {
            if unit.is_command_center {
                let btn = [cmd_rect[0] + 10.0, cmd_rect[1] + 32.0, 320.0, 36.0];
                if (click && is_inside(mouse, btn)) || input.is_key_pressed(KeyCode::KeyW) {
                    actions.push(UiAction::IssueCommand(GameCommand::TrainUnit {
                        team: self.local_team,
                        building_id: unit.id,
                        unit_type: "worker".to_string(),
                    }));
                }
            } else if unit.is_worker {
                let b_mine = [cmd_rect[0] + 10.0, cmd_rect[1] + 30.0, 150.0, 34.0];
                let b_house = [cmd_rect[0] + 170.0, cmd_rect[1] + 30.0, 150.0, 34.0];
                let b_barracks = [cmd_rect[0] + 10.0, cmd_rect[1] + 70.0, 150.0, 34.0];
                let b_repair = [cmd_rect[0] + 170.0, cmd_rect[1] + 70.0, 150.0, 34.0];
                let b_turret = [cmd_rect[0] + 10.0, cmd_rect[1] + 110.0, 150.0, 34.0];
                let b_stop = [cmd_rect[0] + 170.0, cmd_rect[1] + 110.0, 150.0, 34.0];

                if (click && is_inside(mouse, b_mine)) || input.is_key_pressed(KeyCode::KeyM) {
                    self.placement_mode = Some("mine".to_string());
                    actions.push(UiAction::SetPlacementMode(Some("mine".to_string())));
                } else if (click && is_inside(mouse, b_house)) || input.is_key_pressed(KeyCode::KeyH) {
                    self.placement_mode = Some("house".to_string());
                    actions.push(UiAction::SetPlacementMode(Some("house".to_string())));
                } else if (click && is_inside(mouse, b_barracks)) || input.is_key_pressed(KeyCode::KeyB) {
                    self.placement_mode = Some("barracks".to_string());
                    actions.push(UiAction::SetPlacementMode(Some("barracks".to_string())));
                } else if (click && is_inside(mouse, b_repair)) || input.is_key_pressed(KeyCode::KeyR) {
                    self.placement_mode = Some("repair".to_string());
                    actions.push(UiAction::SetPlacementMode(Some("repair".to_string())));
                } else if (click && is_inside(mouse, b_turret)) || input.is_key_pressed(KeyCode::KeyT) {
                    self.placement_mode = Some("turret".to_string());
                    actions.push(UiAction::SetPlacementMode(Some("turret".to_string())));
                } else if (click && is_inside(mouse, b_stop)) || input.is_key_pressed(KeyCode::KeyX) {
                    actions.push(UiAction::IssueCommand(GameCommand::StopUnits {
                        team: self.local_team,
                        unit_ids: self.selected_unit_ids.clone(),
                    }));
                }
            } else if unit.is_barracks {
                let b_soldier = [cmd_rect[0] + 10.0, cmd_rect[1] + 32.0, 320.0, 36.0];
                let b_sniper = [cmd_rect[0] + 10.0, cmd_rect[1] + 74.0, 320.0, 36.0];

                if (click && is_inside(mouse, b_soldier)) || input.is_key_pressed(KeyCode::KeyS) {
                    actions.push(UiAction::IssueCommand(GameCommand::TrainUnit {
                        team: self.local_team,
                        building_id: unit.id,
                        unit_type: "soldier".to_string(),
                    }));
                } else if (click && is_inside(mouse, b_sniper)) || input.is_key_pressed(KeyCode::KeyN) {
                    actions.push(UiAction::IssueCommand(GameCommand::TrainUnit {
                        team: self.local_team,
                        building_id: unit.id,
                        unit_type: "sniper".to_string(),
                    }));
                }
            } else if unit.is_combat {
                let b_atk = [cmd_rect[0] + 10.0, cmd_rect[1] + 32.0, 320.0, 36.0];
                let b_stop = [cmd_rect[0] + 10.0, cmd_rect[1] + 74.0, 320.0, 36.0];

                if (click && is_inside(mouse, b_atk)) || input.is_key_pressed(KeyCode::KeyA) {
                    self.attack_move_mode = true;
                } else if (click && is_inside(mouse, b_stop)) || input.is_key_pressed(KeyCode::KeyX) {
                    actions.push(UiAction::IssueCommand(GameCommand::StopUnits {
                        team: self.local_team,
                        unit_ids: self.selected_unit_ids.clone(),
                    }));
                }
            }
        }

        // Placement mode building placement or cancel
        if let Some(b_type) = &self.placement_mode {
            if input.is_key_pressed(KeyCode::Escape) || right_click {
                self.placement_mode = None;
                actions.push(UiAction::SetPlacementMode(None));
            } else if click && b_type != "repair" {
                if let Some(ground) = camera.screen_to_ground(mouse, viewport_size) {
                    actions.push(UiAction::IssueCommand(GameCommand::ConstructBuilding {
                        team: self.local_team,
                        worker_ids: self.selected_unit_ids.clone(),
                        building_type: b_type.clone(),
                        position: [ground.x, ground.y],
                        resource_node_id: None,
                    }));
                    self.placement_mode = None;
                    actions.push(UiAction::SetPlacementMode(None));
                }
            }
        }
    }
}

pub struct HudRenderer;

impl HudRenderer {
    pub fn render(
        &self,
        state: &UiState,
        viewport_size: Vec2,
        map_size: Vec2,
        instances: &[RenderInstance],
        camera: &IsometricCamera,
    ) -> Vec<Vertex2D> {
        let mut verts = Vec::with_capacity(8192);
        let ref_vp = Vec2::new(1280.0, 720.0);

        match state.current_screen {
            ScreenMode::MainMenu => {
                self.render_main_menu(state, ref_vp, &mut verts);
            }
            ScreenMode::InGame => {
                self.render_in_game(
                    state,
                    ref_vp,
                    map_size,
                    instances,
                    camera,
                    &mut verts,
                );
            }
        }

        let sx = viewport_size.x / ref_vp.x;
        let sy = viewport_size.y / ref_vp.y;
        if (sx - 1.0).abs() > 1e-4 || (sy - 1.0).abs() > 1e-4 {
            for v in &mut verts {
                v.pos[0] *= sx;
                v.pos[1] *= sy;
            }
        }

        verts
    }

    fn render_main_menu(
        &self,
        state: &UiState,
        viewport_size: Vec2,
        verts: &mut Vec<Vertex2D>,
    ) {
        // Full screen backdrop
        append_rect(
            verts,
            [0.0, 0.0, viewport_size.x, viewport_size.y],
            [0.04, 0.06, 0.09, 0.95],
        );

        // Header Title
        append_text_quads(
            verts,
            "MIDNIGHT BATTLE ZONE",
            100.0,
            60.0,
            3.5,
            [0.2, 0.85, 1.0, 1.0],
        );
        append_text_quads(
            verts,
            "TACTICAL RTS - CAT COMMANDERS & LAN MULTIPLAYER",
            100.0,
            110.0,
            1.6,
            [0.6, 0.7, 0.8, 0.9],
        );

        // Mode Section Title
        append_text_quads(verts, "SELECT GAME MODE", 100.0, 155.0, 1.5, [0.85, 0.85, 0.9, 1.0]);

        let modes = [
            (GameMode::SingleplayerAi, [100.0, 180.0, 260.0, 36.0]),
            (GameMode::HostLan, [100.0, 226.0, 260.0, 36.0]),
            (GameMode::JoinLan, [100.0, 272.0, 260.0, 36.0]),
        ];

        for (mode, rect) in modes {
            let is_selected = state.selected_mode == mode;
            let bg_col = if is_selected {
                [0.15, 0.35, 0.55, 0.9]
            } else {
                [0.10, 0.12, 0.16, 0.8]
            };
            let text_col = if is_selected {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                [0.7, 0.75, 0.8, 0.8]
            };

            append_rect(verts, rect, bg_col);
            append_rect_outline(verts, rect, 1.5, [0.2, 0.5, 0.8, 0.7]);
            append_text_quads(verts, mode.label(), rect[0] + 16.0, rect[1] + 10.0, 1.6, text_col);
        }

        // Difficulty Section (Singleplayer)
        if state.selected_mode == GameMode::SingleplayerAi {
            append_text_quads(verts, "AI DIFFICULTY", 100.0, 335.0, 1.5, [0.85, 0.85, 0.9, 1.0]);
            let diffs = [
                (AiDifficulty::Easy, [100.0, 360.0, 80.0, 32.0]),
                (AiDifficulty::Medium, [190.0, 360.0, 80.0, 32.0]),
                (AiDifficulty::Hard, [280.0, 360.0, 80.0, 32.0]),
            ];
            for (diff, rect) in diffs {
                let is_sel = state.selected_difficulty == diff;
                let bg = if is_sel {
                    [0.2, 0.45, 0.25, 0.9]
                } else {
                    [0.10, 0.12, 0.16, 0.8]
                };
                append_rect(verts, rect, bg);
                append_rect_outline(verts, rect, 1.5, [0.3, 0.7, 0.4, 0.7]);
                append_text_quads(verts, diff.label(), rect[0] + 12.0, rect[1] + 9.0, 1.5, [1.0, 1.0, 1.0, 1.0]);
            }
        }

        // FSR selector in Menu
        let fsr_rect = [100.0, 420.0, 260.0, 36.0];
        append_rect(verts, fsr_rect, [0.12, 0.16, 0.22, 0.85]);
        append_rect_outline(verts, fsr_rect, 1.5, [0.5, 0.2, 0.8, 0.8]);
        append_text_quads(verts, state.fsr_mode.label(), fsr_rect[0] + 12.0, fsr_rect[1] + 10.0, 1.5, [0.85, 0.7, 1.0, 1.0]);

        // Music selector in Menu
        let music_rect = [100.0, 468.0, 260.0, 36.0];
        append_rect(verts, music_rect, [0.12, 0.18, 0.20, 0.85]);
        append_rect_outline(verts, music_rect, 1.5, [0.2, 0.75, 0.65, 0.8]);
        let music_str = format!("MUSIC [J]: {}", state.music_label);
        append_text_quads(verts, &music_str, music_rect[0] + 12.0, music_rect[1] + 10.0, 1.3, [0.7, 1.0, 0.9, 1.0]);

        // Map Section
        append_text_quads(verts, "SELECT BATTLE MAP", 400.0, 155.0, 1.5, [0.85, 0.85, 0.9, 1.0]);
        for (i, m) in state.maps.iter().enumerate() {
            let rect = [400.0, 180.0 + (i as f32) * 56.0, 420.0, 46.0];
            let is_sel = state.selected_map_idx == i;
            let bg = if is_sel {
                [0.15, 0.35, 0.45, 0.9]
            } else {
                [0.10, 0.12, 0.16, 0.8]
            };
            append_rect(verts, rect, bg);
            append_rect_outline(verts, rect, 1.5, [0.25, 0.6, 0.7, 0.7]);
            append_text_quads(verts, &m.name, rect[0] + 14.0, rect[1] + 8.0, 1.6, [0.95, 0.95, 1.0, 1.0]);
            let short_desc: String = if m.description.len() > 54 {
                format!("{}...", &m.description[..51])
            } else {
                m.description.clone()
            };
            append_text_quads(verts, &short_desc, rect[0] + 14.0, rect[1] + 28.0, 1.1, [0.6, 0.7, 0.75, 0.8]);
        }

        // Direct IP Connect box (Join LAN)
        if state.selected_mode == GameMode::JoinLan {
            append_text_quads(verts, "CONNECT TO LAN IP ADDRESS", 400.0, 355.0, 1.4, [0.85, 0.85, 0.9, 1.0]);
            let ip_rect = [400.0, 380.0, 420.0, 36.0];
            append_rect(verts, ip_rect, [0.08, 0.10, 0.14, 0.9]);
            let border_col = if state.ip_input_focused {
                [0.2, 0.85, 1.0, 1.0]
            } else {
                [0.3, 0.4, 0.5, 0.7]
            };
            append_rect_outline(verts, ip_rect, 1.5, border_col);
            append_text_quads(verts, &state.lan_ip_input, ip_rect[0] + 12.0, ip_rect[1] + 10.0, 1.7, [0.9, 0.95, 1.0, 1.0]);
        }

        // Action Button: START / CONNECT
        let action_rect = [400.0, 480.0, 280.0, 52.0];
        append_rect(verts, action_rect, [0.15, 0.55, 0.25, 0.95]);
        append_rect_outline(verts, action_rect, 2.0, [0.3, 0.9, 0.4, 1.0]);

        let btn_label = if state.selected_mode == GameMode::JoinLan {
            "CONNECT TO GAME"
        } else {
            "START BATTLE"
        };
        append_text_quads(verts, btn_label, action_rect[0] + 32.0, action_rect[1] + 16.0, 2.0, [1.0, 1.0, 1.0, 1.0]);

        // Two Tuxedo Cats (Black & White) Showcase on the Main Load Screen
        let showcase_x = (viewport_size.x - 410.0).max(835.0);
        let showcase_rect = [showcase_x, 180.0, 380.0, 352.0];
        append_text_quads(
            verts,
            "TUXEDO CAT COMMANDERS",
            showcase_x,
            155.0,
            1.5,
            [0.85, 0.85, 0.9, 1.0],
        );
        append_rect(verts, showcase_rect, [0.06, 0.09, 0.14, 0.92]);
        append_rect_outline(verts, showcase_rect, 2.0, [0.25, 0.65, 0.90, 0.85]);

        // Pedestal / floor platform inside showcase
        append_rect(
            verts,
            [showcase_x + 20.0, 442.0, 340.0, 8.0],
            [0.18, 0.28, 0.40, 0.9],
        );

        // Tuxedo Cat #1 (Left): Raphael - Black & White Tuxedo Cat with Emerald Green eyes & Red Bow Tie
        append_tuxedo_cat(
            verts,
            showcase_x + 105.0,
            345.0,
            1.0,
            true,
            [0.20, 0.95, 0.40, 1.0],
            [0.95, 0.18, 0.20, 1.0],
        );
        append_text_quads(
            verts,
            "RAPHAEL",
            showcase_x + 58.0,
            465.0,
            1.5,
            [1.0, 0.35, 0.35, 1.0],
        );
        append_text_quads(
            verts,
            "TUXEDO COMMANDER",
            showcase_x + 36.0,
            488.0,
            1.0,
            [0.75, 0.82, 0.90, 0.9],
        );

        // Center "VS" badge between the two Tuxedo Cats
        append_text_quads(
            verts,
            "VS",
            showcase_x + 176.0,
            340.0,
            1.8,
            [1.0, 0.85, 0.30, 1.0],
        );

        // Tuxedo Cat #2 (Right): Michaelangelo - Black & White Tuxedo Cat with Golden Amber eyes & Orange Bow Tie
        append_tuxedo_cat(
            verts,
            showcase_x + 275.0,
            345.0,
            1.0,
            false,
            [1.0, 0.80, 0.15, 1.0],
            [1.0, 0.55, 0.08, 1.0],
        );
        append_text_quads(
            verts,
            "MICHAELANGELO",
            showcase_x + 206.0,
            465.0,
            1.4,
            [1.0, 0.65, 0.20, 1.0],
        );
        append_text_quads(
            verts,
            "TUXEDO COMMANDER",
            showcase_x + 210.0,
            488.0,
            1.0,
            [0.75, 0.82, 0.90, 0.9],
        );
    }

    fn render_in_game(
        &self,
        state: &UiState,
        viewport_size: Vec2,
        map_size: Vec2,
        instances: &[RenderInstance],
        camera: &IsometricCamera,
        verts: &mut Vec<Vertex2D>,
    ) {
        // 1. Top Bar
        append_rect(verts, [0.0, 0.0, viewport_size.x, 46.0], [0.06, 0.08, 0.12, 0.92]);
        append_rect(verts, [0.0, 45.0, viewport_size.x, 1.5], [0.2, 0.35, 0.45, 0.8]);

        // Menu button
        let menu_rect = [15.0, 8.0, 80.0, 30.0];
        append_rect(verts, menu_rect, [0.12, 0.16, 0.22, 0.85]);
        append_rect_outline(verts, menu_rect, 1.0, [0.3, 0.45, 0.6, 0.7]);
        append_text_quads(verts, "MENU", menu_rect[0] + 18.0, menu_rect[1] + 8.0, 1.6, [0.9, 0.95, 1.0, 1.0]);

        // Pause/Resume button
        let pause_rect = [105.0, 8.0, 110.0, 30.0];
        let pause_bg = if state.paused {
            [0.55, 0.35, 0.10, 0.9]
        } else {
            [0.12, 0.16, 0.22, 0.85]
        };
        append_rect(verts, pause_rect, pause_bg);
        append_rect_outline(verts, pause_rect, 1.0, [0.8, 0.6, 0.2, 0.8]);
        let pause_text = if state.paused { "RESUME [P]" } else { "PAUSE [P]" };
        append_text_quads(verts, pause_text, pause_rect[0] + 10.0, pause_rect[1] + 8.0, 1.5, [1.0, 0.9, 0.5, 1.0]);

        // FSR Mode Button in Top Bar
        let fsr_rect = [225.0, 8.0, 115.0, 30.0];
        append_rect(verts, fsr_rect, [0.18, 0.14, 0.24, 0.85]);
        append_rect_outline(verts, fsr_rect, 1.0, [0.6, 0.3, 0.9, 0.8]);
        let fsr_short = match state.fsr_mode {
            FsrMode::Off => "FSR OFF [F]",
            FsrMode::UltraQuality => "FSR U-Q [F]",
            FsrMode::Quality => "FSR QLT [F]",
            FsrMode::Balanced => "FSR BAL [F]",
            FsrMode::Performance => "FSR PRF [F]",
        };
        append_text_quads(verts, fsr_short, fsr_rect[0] + 8.0, fsr_rect[1] + 8.0, 1.4, [0.85, 0.7, 1.0, 1.0]);

        // Resource counter & Supply
        let ore_text = format!("MINES: {:>4}", state.resources);
        append_text_quads(verts, &ore_text, 360.0, 14.0, 1.7, [0.2, 0.95, 1.0, 1.0]);

        let supply_text = format!("SUPPLY: {:>2}/{:<2}", state.supply_used, state.supply_cap);
        append_text_quads(verts, &supply_text, 520.0, 14.0, 1.7, [0.95, 0.8, 0.2, 1.0]);

        // Map & Mode Label in top center
        let mode_label = format!("{} | {}", state.current_map().name, state.selected_mode.label());
        append_text_quads(verts, &mode_label, 675.0, 16.0, 1.15, [0.65, 0.75, 0.85, 0.9]);

        // PROMINENT TOP-RIGHT FPS & TIME COUNTER
        let total_secs = state.elapsed_seconds as u64;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        let stats_text = format!("FPS: {:<3}   TIME: {:02}:{:02}", state.fps, mins, secs);

        let tr_box_w = 260.0;
        let tr_box_x = viewport_size.x - tr_box_w - 15.0;
        let tr_box = [tr_box_x, 7.0, tr_box_w, 32.0];
        append_rect(verts, tr_box, [0.08, 0.11, 0.16, 0.9]);
        append_rect_outline(verts, tr_box, 1.5, [0.3, 0.7, 0.9, 0.8]);
        append_text_quads(verts, &stats_text, tr_box[0] + 16.0, tr_box[1] + 9.0, 1.7, [0.25, 0.95, 0.45, 1.0]);

        // 2. Interactive Minimap (Bottom-left)
        let minimap_size = 190.0;
        let minimap_rect = [
            20.0,
            viewport_size.y - minimap_size - 20.0,
            minimap_size,
            minimap_size,
        ];
        append_rect(verts, minimap_rect, [0.05, 0.07, 0.10, 0.95]);
        append_rect_outline(verts, minimap_rect, 2.0, [0.25, 0.5, 0.7, 0.85]);

        for inst in instances {
            let mx = minimap_rect[0] + (inst.position.x / map_size.x.max(1.0)) * minimap_size;
            let my = minimap_rect[1] + (inst.position.z / map_size.y.max(1.0)) * minimap_size;

            let dot_col = if inst.mesh_archetype == "resource_node" {
                [0.15, 0.9, 1.0, 1.0]
            } else if inst.mesh_archetype == "obstacle" {
                [0.62, 0.52, 0.38, 0.95]
            } else if inst.color[0] > 0.6 && inst.color[2] < 0.4 {
                [0.95, 0.2, 0.2, 1.0]
            } else {
                [0.2, 0.6, 1.0, 1.0]
            };

            let dot_size = if inst.mesh_archetype == "obstacle" && inst.scale.x >= 4.5 {
                6.5
            } else if inst.mesh_archetype == "obstacle" {
                4.2
            } else if inst.mesh_archetype.contains("center")
                || inst.mesh_archetype.contains("barracks")
                || inst.mesh_archetype.contains("turret")
            {
                5.0
            } else {
                3.0
            };

            append_rect(
                verts,
                [mx - dot_size * 0.5, my - dot_size * 0.5, dot_size, dot_size],
                dot_col,
            );
        }

        // Camera Frustum footprint on minimap
        let cam_target_x = minimap_rect[0] + (camera.target.x / map_size.x.max(1.0)) * minimap_size;
        let cam_target_y = minimap_rect[1] + (camera.target.z / map_size.y.max(1.0)) * minimap_size;
        let cam_box_w = (camera.distance / 120.0) * minimap_size * 0.7;
        let cam_box_h = (camera.distance / 120.0) * minimap_size * 0.5;
        append_rect_outline(
            verts,
            [
                cam_target_x - cam_box_w * 0.5,
                cam_target_y - cam_box_h * 0.5,
                cam_box_w,
                cam_box_h,
            ],
            1.5,
            [1.0, 1.0, 0.3, 0.8],
        );

        // 3. Selection Details Panel (Bottom-center)
        let sel_w = 340.0;
        let sel_h = 170.0;
        let sel_rect = [230.0, viewport_size.y - sel_h - 20.0, sel_w, sel_h];
        append_rect(verts, sel_rect, [0.06, 0.08, 0.12, 0.92]);
        append_rect_outline(verts, sel_rect, 1.5, [0.2, 0.35, 0.45, 0.8]);

        if let Some(unit) = &state.selected_unit {
            append_text_quads(verts, &unit.name, sel_rect[0] + 16.0, sel_rect[1] + 14.0, 2.0, [1.0, 0.95, 0.6, 1.0]);

            let hp_pct = (unit.hp / unit.max_hp.max(1.0)).clamp(0.0, 1.0);
            let bar_rect = [sel_rect[0] + 16.0, sel_rect[1] + 45.0, 220.0, 16.0];
            append_rect(verts, bar_rect, [0.2, 0.2, 0.25, 0.9]);
            append_rect(
                verts,
                [bar_rect[0], bar_rect[1], bar_rect[2] * hp_pct, bar_rect[3]],
                [0.2, 0.85, 0.3, 1.0],
            );
            append_rect_outline(verts, bar_rect, 1.0, [0.4, 0.5, 0.6, 0.8]);

            let hp_label = format!("{:.0}/{:.0} HP", unit.hp, unit.max_hp);
            append_text_quads(verts, &hp_label, bar_rect[0] + 8.0, bar_rect[1] + 3.0, 1.2, [1.0, 1.0, 1.0, 1.0]);

            let stats_line = format!("ARMOR: {:.0}   ATTACK: {:.0}", unit.armor, unit.damage);
            append_text_quads(verts, &stats_line, sel_rect[0] + 16.0, sel_rect[1] + 74.0, 1.5, [0.75, 0.85, 0.95, 1.0]);

            let order_line = format!("ORDER: {}", unit.order_desc);
            append_text_quads(verts, &order_line, sel_rect[0] + 16.0, sel_rect[1] + 98.0, 1.4, [0.5, 0.8, 1.0, 0.9]);

            if let Some(progress) = unit.production_progress {
                let p_bar = [sel_rect[0] + 16.0, sel_rect[1] + 126.0, 220.0, 12.0];
                append_rect(verts, p_bar, [0.15, 0.15, 0.2, 0.9]);
                append_rect(
                    verts,
                    [p_bar[0], p_bar[1], p_bar[2] * progress.clamp(0.0, 1.0), p_bar[3]],
                    [0.2, 0.7, 1.0, 1.0],
                );
                append_rect_outline(verts, p_bar, 1.0, [0.3, 0.5, 0.7, 0.8]);
                append_text_quads(verts, "TRAINING / BUILDING...", p_bar[0] + 4.0, p_bar[1] + 2.0, 1.0, [1.0, 1.0, 1.0, 0.9]);
            }
        } else {
            append_text_quads(verts, "NO UNIT SELECTED", sel_rect[0] + 16.0, sel_rect[1] + 20.0, 1.6, [0.5, 0.6, 0.7, 0.8]);
            append_text_quads(verts, "Left-click or drag-box to select Cat units", sel_rect[0] + 16.0, sel_rect[1] + 50.0, 1.2, [0.4, 0.5, 0.6, 0.7]);
        }

        // 4. Command Card Panel (Bottom-right)
        let cmd_w = 340.0;
        let cmd_h = 170.0;
        let cmd_rect = [viewport_size.x - cmd_w - 20.0, viewport_size.y - cmd_h - 20.0, cmd_w, cmd_h];
        append_rect(verts, cmd_rect, [0.06, 0.08, 0.12, 0.92]);
        append_rect_outline(verts, cmd_rect, 1.5, [0.2, 0.35, 0.45, 0.8]);

        if let Some(unit) = &state.selected_unit {
            if unit.is_command_center {
                append_text_quads(verts, "COMMAND CENTER ORDERS", cmd_rect[0] + 14.0, cmd_rect[1] + 10.0, 1.3, [0.7, 0.8, 0.9, 0.9]);
                let btn = [cmd_rect[0] + 10.0, cmd_rect[1] + 32.0, 320.0, 36.0];
                append_rect(verts, btn, [0.12, 0.30, 0.45, 0.9]);
                append_rect_outline(verts, btn, 1.5, [0.2, 0.7, 1.0, 0.8]);
                append_text_quads(verts, "TRAIN WORKER CAT [W] (50)", btn[0] + 18.0, btn[1] + 10.0, 1.6, [1.0, 1.0, 1.0, 1.0]);
            } else if unit.is_worker {
                append_text_quads(verts, "WORKER CAT CONSTRUCTION", cmd_rect[0] + 14.0, cmd_rect[1] + 10.0, 1.3, [0.7, 0.8, 0.9, 0.9]);

                let b_mine = [cmd_rect[0] + 10.0, cmd_rect[1] + 30.0, 150.0, 34.0];
                let b_house = [cmd_rect[0] + 170.0, cmd_rect[1] + 30.0, 150.0, 34.0];
                let b_barracks = [cmd_rect[0] + 10.0, cmd_rect[1] + 70.0, 150.0, 34.0];
                let b_repair = [cmd_rect[0] + 170.0, cmd_rect[1] + 70.0, 150.0, 34.0];
                let b_turret = [cmd_rect[0] + 10.0, cmd_rect[1] + 110.0, 150.0, 34.0];
                let b_stop = [cmd_rect[0] + 170.0, cmd_rect[1] + 110.0, 150.0, 34.0];

                for (b, text) in [
                    (b_mine, "MINE [M] (75)"),
                    (b_house, "HOUSE [H] (100)"),
                    (b_barracks, "BARRACKS [B] (175)"),
                    (b_repair, "REPAIR [R]"),
                    (b_turret, "TURRET [T] (1000)"),
                    (b_stop, "STOP [X]"),
                ] {
                    append_rect(verts, b, [0.12, 0.25, 0.35, 0.85]);
                    append_rect_outline(verts, b, 1.2, [0.25, 0.6, 0.8, 0.75]);
                    append_text_quads(verts, text, b[0] + 10.0, b[1] + 9.0, 1.3, [0.95, 0.95, 1.0, 1.0]);
                }
            } else if unit.is_barracks {
                append_text_quads(verts, "BARRACKS PRODUCTION", cmd_rect[0] + 14.0, cmd_rect[1] + 10.0, 1.3, [0.7, 0.8, 0.9, 0.9]);
                let b_sol = [cmd_rect[0] + 10.0, cmd_rect[1] + 32.0, 320.0, 36.0];
                let b_snp = [cmd_rect[0] + 10.0, cmd_rect[1] + 74.0, 320.0, 36.0];

                append_rect(verts, b_sol, [0.15, 0.35, 0.25, 0.9]);
                append_rect_outline(verts, b_sol, 1.5, [0.3, 0.8, 0.5, 0.8]);
                append_text_quads(verts, "TRAIN SOLDIER CAT [S] (100)", b_sol[0] + 16.0, b_sol[1] + 10.0, 1.5, [1.0, 1.0, 1.0, 1.0]);

                append_rect(verts, b_snp, [0.15, 0.25, 0.40, 0.9]);
                append_rect_outline(verts, b_snp, 1.5, [0.3, 0.6, 0.9, 0.8]);
                append_text_quads(verts, "TRAIN SNIPER CAT [N] (200)", b_snp[0] + 16.0, b_snp[1] + 10.0, 1.5, [1.0, 1.0, 1.0, 1.0]);
            } else if unit.is_combat {
                append_text_quads(verts, "COMBAT ORDERS", cmd_rect[0] + 14.0, cmd_rect[1] + 10.0, 1.3, [0.7, 0.8, 0.9, 0.9]);
                let b_atk = [cmd_rect[0] + 10.0, cmd_rect[1] + 32.0, 320.0, 36.0];
                let b_stp = [cmd_rect[0] + 10.0, cmd_rect[1] + 74.0, 320.0, 36.0];

                let atk_bg = if state.attack_move_mode {
                    [0.6, 0.2, 0.2, 0.95]
                } else {
                    [0.3, 0.15, 0.15, 0.85]
                };
                append_rect(verts, b_atk, atk_bg);
                append_rect_outline(verts, b_atk, 1.5, [0.9, 0.3, 0.3, 0.8]);
                append_text_quads(verts, "ATTACK-MOVE [A]", b_atk[0] + 16.0, b_atk[1] + 10.0, 1.6, [1.0, 1.0, 1.0, 1.0]);

                append_rect(verts, b_stp, [0.15, 0.18, 0.22, 0.85]);
                append_rect_outline(verts, b_stp, 1.5, [0.4, 0.5, 0.6, 0.8]);
                append_text_quads(verts, "STOP [X]", b_stp[0] + 16.0, b_stp[1] + 10.0, 1.6, [1.0, 1.0, 1.0, 1.0]);
            }
        } else {
            append_text_quads(verts, "NO COMMANDS", cmd_rect[0] + 16.0, cmd_rect[1] + 20.0, 1.6, [0.5, 0.6, 0.7, 0.8]);
        }

        // 5. Box-selection overlay
        if let (Some(start), Some(curr)) = (state.box_drag_start, state.box_drag_current) {
            let min_x = start.x.min(curr.x);
            let min_y = start.y.min(curr.y);
            let w = (start.x - curr.x).abs();
            let h = (start.y - curr.y).abs();
            append_rect(verts, [min_x, min_y, w, h], [0.2, 0.9, 0.4, 0.15]);
            append_rect_outline(verts, [min_x, min_y, w, h], 1.5, [0.3, 1.0, 0.5, 0.85]);
        }

        // 5b. Floating Cat "MEOW!" / "MROW!" Speech Callouts above units
        for callout in &state.meow_callouts {
            if let Some(screen_pos) = camera.world_to_screen(callout.world_pos, viewport_size) {
                let bubble_w = (callout.text.len() as f32) * 10.0 + 16.0;
                let bubble_h = 24.0;
                let bx = screen_pos.x - bubble_w * 0.5;
                let by = screen_pos.y - 36.0;
                append_rect(verts, [bx, by, bubble_w, bubble_h], [0.10, 0.14, 0.20, 0.9]);
                append_rect_outline(verts, [bx, by, bubble_w, bubble_h], 1.5, [1.0, 0.85, 0.3, 0.95]);
                append_text_quads(
                    verts,
                    &callout.text,
                    bx + 8.0,
                    by + 6.0,
                    1.5,
                    [1.0, 0.95, 0.4, 1.0],
                );
            }
        }

        // 6. Placement mode cursor hint
        if let Some(b_type) = &state.placement_mode {
            let label = format!("PLACING: {} (CLICK TO BUILD, ESC TO CANCEL)", b_type.to_uppercase());
            let banner = [viewport_size.x * 0.5 - 260.0, 60.0, 520.0, 36.0];
            append_rect(verts, banner, [0.1, 0.45, 0.2, 0.9]);
            append_rect_outline(verts, banner, 1.5, [0.3, 1.0, 0.5, 0.9]);
            append_text_quads(verts, &label, banner[0] + 16.0, banner[1] + 10.0, 1.5, [1.0, 1.0, 1.0, 1.0]);
        }

        // 7. PROMINENT GAME PAUSED BANNER
        if state.paused {
            let p_box_w = 640.0;
            let p_box_h = 70.0;
            let p_box = [
                viewport_size.x * 0.5 - p_box_w * 0.5,
                viewport_size.y * 0.5 - p_box_h * 0.5 - 30.0,
                p_box_w,
                p_box_h,
            ];
            append_rect(verts, p_box, [0.75, 0.45, 0.08, 0.95]);
            append_rect_outline(verts, p_box, 3.0, [1.0, 0.9, 0.3, 1.0]);
            append_text_quads(
                verts,
                "GAME PAUSED (PRESS P TO RESUME)",
                p_box[0] + 40.0,
                p_box[1] + 24.0,
                2.5,
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        // 8. Victory / Defeat Center Banner
        if let Some(won) = state.game_over {
            let end_w = 700.0;
            let end_h = 160.0;
            let end_rect = [
                viewport_size.x * 0.5 - end_w * 0.5,
                viewport_size.y * 0.5 - end_h * 0.5,
                end_w,
                end_h,
            ];

            let (banner_bg, border_col, title_text) = if won {
                (
                    [0.10, 0.45, 0.20, 0.95],
                    [0.2, 1.0, 0.4, 1.0],
                    "VICTORY - ALL ENEMY BUILDINGS DESTROYED!",
                )
            } else {
                (
                    [0.55, 0.12, 0.12, 0.95],
                    [1.0, 0.3, 0.3, 1.0],
                    "DEFEAT - YOUR COMMAND CENTER WAS DESTROYED!",
                )
            };

            append_rect(verts, end_rect, banner_bg);
            append_rect_outline(verts, end_rect, 3.0, border_col);
            append_text_quads(verts, title_text, end_rect[0] + 30.0, end_rect[1] + 35.0, 2.2, [1.0, 1.0, 1.0, 1.0]);

            let ret_btn = [
                viewport_size.x * 0.5 - 120.0,
                end_rect[1] + 90.0,
                240.0,
                42.0,
            ];
            append_rect(verts, ret_btn, [0.15, 0.20, 0.30, 0.9]);
            append_rect_outline(verts, ret_btn, 1.5, [0.4, 0.6, 0.8, 1.0]);
            append_text_quads(verts, "RETURN TO MENU", ret_btn[0] + 32.0, ret_btn[1] + 12.0, 1.8, [1.0, 1.0, 1.0, 1.0]);
        }
    }
}

fn is_inside(pos: Vec2, rect: [f32; 4]) -> bool {
    pos.x >= rect[0]
        && pos.x <= rect[0] + rect[2]
        && pos.y >= rect[1]
        && pos.y <= rect[1] + rect[3]
}

/// Draws a full-body Black & White Tuxedo Cat portrait centered around `(cx, cy)`.
/// Includes jet-black fur, pointy cat ears with pink inner ears, snowy-white tuxedo
/// chest bib, white muzzle blaze, four white socks/mittens, white-tipped curved tail,
/// glowing feline eyes with vertical slit pupils, pink nose, white whiskers, and a bow tie.
fn append_tuxedo_cat(
    verts: &mut Vec<Vertex2D>,
    cx: f32,
    cy: f32,
    scale: f32,
    facing_right: bool,
    eye_color: [f32; 4],
    bowtie_color: [f32; 4],
) {
    let s = scale;
    let dir = if facing_right { 1.0 } else { -1.0 };

    let black_fur = [0.07, 0.08, 0.10, 1.0];
    let charcoal_rim = [0.22, 0.25, 0.32, 1.0];
    let white_fur = [0.96, 0.97, 0.99, 1.0];
    let ear_pink = [0.95, 0.65, 0.74, 1.0];
    let nose_pink = [0.98, 0.56, 0.68, 1.0];
    let pupil_dark = [0.04, 0.04, 0.06, 1.0];

    // 1. Curved Black Tail with Snowy-White Tip (extends behind the cat)
    let tail_x = cx - dir * 42.0 * s;
    let hip_min_x = cx.min(tail_x);
    append_rect(
        verts,
        [hip_min_x, cy + 56.0 * s, 42.0 * s, 12.0 * s],
        black_fur,
    );
    append_rect(
        verts,
        [tail_x - 6.0 * s, cy - 18.0 * s, 12.0 * s, 84.0 * s],
        black_fur,
    );
    // White tuxedo tail tip
    append_rect(
        verts,
        [tail_x - 7.0 * s, cy - 34.0 * s, 14.0 * s, 18.0 * s],
        white_fur,
    );

    // 2. Seated Black Torso & Haunches (with subtle silver-blue rim outline)
    let body_rect = [cx - 34.0 * s, cy - 8.0 * s, 68.0 * s, 92.0 * s];
    append_rect(verts, body_rect, black_fur);
    append_rect_outline(verts, body_rect, 1.5 * s, charcoal_rim);

    // Rear haunches
    append_rect(
        verts,
        [cx - 40.0 * s, cy + 44.0 * s, 80.0 * s, 40.0 * s],
        black_fur,
    );

    // 3. Snowy-White Tuxedo Chest Bib (formal shirt front + V-shaped tuxedo lapel)
    append_rect(
        verts,
        [cx - 20.0 * s, cy + 2.0 * s, 40.0 * s, 54.0 * s],
        white_fur,
    );
    append_triangle(
        verts,
        [cx - 22.0 * s, cy + 2.0 * s],
        [cx + 22.0 * s, cy + 2.0 * s],
        [cx, cy + 72.0 * s],
        white_fur,
    );

    // 4. Black Front Legs & Four Snowy-White Tuxedo Socks / Mittens (Paws)
    // Left & right front legs
    append_rect(
        verts,
        [cx - 26.0 * s, cy + 52.0 * s, 18.0 * s, 36.0 * s],
        black_fur,
    );
    append_rect(
        verts,
        [cx + 8.0 * s, cy + 52.0 * s, 18.0 * s, 36.0 * s],
        black_fur,
    );
    // White front mittens
    append_rect(
        verts,
        [cx - 28.0 * s, cy + 80.0 * s, 22.0 * s, 16.0 * s],
        white_fur,
    );
    append_rect(
        verts,
        [cx + 6.0 * s, cy + 80.0 * s, 22.0 * s, 16.0 * s],
        white_fur,
    );
    // White hind paws peeking out at base
    append_rect(
        verts,
        [cx - 44.0 * s, cy + 82.0 * s, 16.0 * s, 14.0 * s],
        white_fur,
    );
    append_rect(
        verts,
        [cx + 28.0 * s, cy + 82.0 * s, 16.0 * s, 14.0 * s],
        white_fur,
    );

    // 5. Formal Bow Tie at the Tuxedo Collar
    let bow_y = cy - 4.0 * s;
    append_triangle(
        verts,
        [cx - 18.0 * s, bow_y - 9.0 * s],
        [cx - 18.0 * s, bow_y + 9.0 * s],
        [cx, bow_y],
        bowtie_color,
    );
    append_triangle(
        verts,
        [cx + 18.0 * s, bow_y - 9.0 * s],
        [cx + 18.0 * s, bow_y + 9.0 * s],
        [cx, bow_y],
        bowtie_color,
    );
    append_rect(
        verts,
        [cx - 4.0 * s, bow_y - 4.0 * s, 8.0 * s, 8.0 * s],
        white_fur,
    );

    // 6. Cat Head (Jet-Black with Cheek Tufts)
    let head_rect = [cx - 32.0 * s, cy - 64.0 * s, 64.0 * s, 56.0 * s];
    append_rect(verts, head_rect, black_fur);
    append_rect_outline(verts, head_rect, 1.5 * s, charcoal_rim);

    // 7. Pointy Triangular Cat Ears (Black outer + Pink inner ear triangles)
    // Left Ear
    append_triangle(
        verts,
        [cx - 32.0 * s, cy - 62.0 * s],
        [cx - 8.0 * s, cy - 62.0 * s],
        [cx - 26.0 * s, cy - 98.0 * s],
        black_fur,
    );
    append_triangle(
        verts,
        [cx - 28.0 * s, cy - 64.0 * s],
        [cx - 13.0 * s, cy - 64.0 * s],
        [cx - 24.0 * s, cy - 89.0 * s],
        ear_pink,
    );
    // Right Ear
    append_triangle(
        verts,
        [cx + 8.0 * s, cy - 62.0 * s],
        [cx + 32.0 * s, cy - 62.0 * s],
        [cx + 26.0 * s, cy - 98.0 * s],
        black_fur,
    );
    append_triangle(
        verts,
        [cx + 13.0 * s, cy - 64.0 * s],
        [cx + 28.0 * s, cy - 64.0 * s],
        [cx + 24.0 * s, cy - 89.0 * s],
        ear_pink,
    );

    // 8. Classic Tuxedo White Facial Blaze & Muzzle (Inverted V on Nose/Cheeks + White Chin)
    append_triangle(
        verts,
        [cx, cy - 46.0 * s],
        [cx - 20.0 * s, cy - 10.0 * s],
        [cx + 20.0 * s, cy - 10.0 * s],
        white_fur,
    );
    append_rect(
        verts,
        [cx - 16.0 * s, cy - 22.0 * s, 32.0 * s, 14.0 * s],
        white_fur,
    );

    // 9. Luminous Feline Eyes + Vertical Slit Pupils + Specular Highlight
    for &eye_dx in &[-18.0 * s, 6.0 * s] {
        let ex = cx + eye_dx;
        let ey = cy - 46.0 * s;
        append_rect(verts, [ex, ey, 12.0 * s, 10.0 * s], eye_color);
        // Vertical slit pupil
        append_rect(
            verts,
            [ex + 4.5 * s, ey + 1.0 * s, 3.0 * s, 8.0 * s],
            pupil_dark,
        );
        // Specular eye shine
        append_rect(
            verts,
            [ex + 2.0 * s, ey + 1.5 * s, 2.5 * s, 2.5 * s],
            white_fur,
        );
    }

    // 10. Pink Triangular Nose
    append_triangle(
        verts,
        [cx - 5.0 * s, cy - 26.0 * s],
        [cx + 5.0 * s, cy - 26.0 * s],
        [cx, cy - 20.0 * s],
        nose_pink,
    );

    // 11. Six Crisp White Whiskers (3 on each side)
    for (i, &wy) in [-24.0 * s, -20.0 * s, -16.0 * s].iter().enumerate() {
        let spread = (i as f32 - 1.0) * 1.5 * s;
        // Left whiskers
        append_rect(
            verts,
            [cx - 44.0 * s, cy + wy + spread, 24.0 * s, 1.5 * s],
            white_fur,
        );
        // Right whiskers
        append_rect(
            verts,
            [cx + 20.0 * s, cy + wy + spread, 24.0 * s, 1.5 * s],
            white_fur,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_menu_renders_two_tuxedo_cats() {
        let state = UiState::new();
        let hud = HudRenderer;
        let cam = IsometricCamera::new(glam::Vec3::ZERO);
        let verts = hud.render(
            &state,
            Vec2::new(1280.0, 720.0),
            Vec2::new(96.0, 96.0),
            &[],
            &cam,
        );

        // Verify the main load screen produces geometry including black fur and white tuxedo markings
        assert!(verts.len() > 500);
        let has_black_fur = verts
            .iter()
            .any(|v| (v.color[0] - 0.07).abs() < 0.01 && (v.color[1] - 0.08).abs() < 0.01);
        let has_white_tuxedo = verts
            .iter()
            .any(|v| (v.color[0] - 0.96).abs() < 0.01 && (v.color[1] - 0.97).abs() < 0.01);
        assert!(has_black_fur, "Main menu should include black fur vertices for the two tuxedo cats");
        assert!(
            has_white_tuxedo,
            "Main menu should include white tuxedo markings for the two tuxedo cats"
        );
    }

    #[test]
    fn test_victory_return_to_menu_button() {
        let mut state = UiState::new();
        state.current_screen = ScreenMode::InGame;
        state.game_over = Some(true);

        let cam = IsometricCamera::new(glam::Vec3::ZERO);
        // Simulate clicking the center of the RETURN TO MENU button at 1920x1080 scaled resolution
        // On 1280x720 reference canvas, ret_btn is [520.0, 370.0, 240.0, 42.0] (center = 640.0, 391.0)
        let mut input = InputState::new(1920, 1080);
        input.mouse_pos = Vec2::new(640.0 * (1920.0 / 1280.0), 391.0 * (1080.0 / 720.0));
        input.left_pressed = true;

        let actions = state.handle_input(
            &input,
            &cam,
            Vec2::new(1920.0, 1080.0),
            Vec2::new(96.0, 96.0),
        );

        assert_eq!(state.current_screen, ScreenMode::MainMenu);
        assert!(actions.iter().any(|a| matches!(a, UiAction::ReturnToMenu)));
    }
}
