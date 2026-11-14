pub mod audio;
pub mod net;
pub mod platform;
pub mod renderer;
pub mod scripting;
pub mod sim;
pub mod types;
pub mod ui;

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use glam::{Vec2, Vec3};

use crate::audio::{CatAudioEngine, MeowType, MusicTrack};
use crate::net::{LockstepSession, MdnsService};
use crate::platform::x11::KeyCode;
use crate::platform::{AppWindow, GamepadManager};
use crate::renderer::{FsrMode, IsometricCamera, VulkanRenderer};
use crate::scripting::LuaScriptingEngine;
use crate::sim::{BlueprintRegistry, SimWorld};
use crate::types::{
    AiDifficulty, EntityId, GameCommand, UnitOrderState, TEAM_PLAYER_1, TEAM_PLAYER_2,
};
use crate::ui::hud::{
    GameMode, HudRenderer, LanLobbyInfo as UiLanLobbyInfo, MapOption, MeowCallout, ScreenMode,
    SelectedUnitInfo, UiAction, UiState,
};

const SIM_TICK_HZ: u64 = 20;
const SIM_TICK_DURATION: Duration = Duration::from_millis(1000 / SIM_TICK_HZ);
const DEFAULT_LAN_PORT: u16 = 28765;

struct CliOptions {
    headless: bool,
    headless_ticks: u64,
    start_immediately: bool,
    map_id: String,
    difficulty: AiDifficulty,
    host_port: Option<u16>,
    connect_addr: Option<String>,
    fsr_mode: FsrMode,
    music_track: MusicTrack,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            headless: false,
            headless_ticks: 200,
            start_immediately: false,
            map_id: "midnight_valley".to_string(),
            difficulty: AiDifficulty::Medium,
            host_port: None,
            connect_addr: None,
            fsr_mode: FsrMode::Off,
            music_track: MusicTrack::Off,
        }
    }
}

fn parse_cli() -> CliOptions {
    let mut opts = CliOptions::default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--headless" => {
                opts.headless = true;
                opts.start_immediately = true;
            }
            "--ticks" if i + 1 < args.len() => {
                i += 1;
                opts.headless_ticks = args[i].parse().unwrap_or(200);
            }
            "--map" if i + 1 < args.len() => {
                i += 1;
                opts.map_id = args[i].clone();
                opts.start_immediately = true;
            }
            "--ai" if i + 1 < args.len() => {
                i += 1;
                opts.difficulty = match args[i].to_lowercase().as_str() {
                    "easy" | "1" => AiDifficulty::Easy,
                    "hard" | "3" => AiDifficulty::Hard,
                    _ => AiDifficulty::Medium,
                };
                opts.start_immediately = true;
            }
            "--host" => {
                let port = if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    i += 1;
                    args[i].parse().unwrap_or(DEFAULT_LAN_PORT)
                } else {
                    DEFAULT_LAN_PORT
                };
                opts.host_port = Some(port);
                opts.start_immediately = true;
            }
            "--connect" if i + 1 < args.len() => {
                i += 1;
                opts.connect_addr = Some(args[i].clone());
                opts.start_immediately = true;
            }
            "--fsr" if i + 1 < args.len() => {
                i += 1;
                opts.fsr_mode = match args[i].to_lowercase().as_str() {
                    "ultra" | "ultraquality" => FsrMode::UltraQuality,
                    "quality" => FsrMode::Quality,
                    "balanced" => FsrMode::Balanced,
                    "performance" | "perf" => FsrMode::Performance,
                    _ => FsrMode::Off,
                };
            }
            "--music" if i + 1 < args.len() => {
                i += 1;
                opts.music_track = match args[i].to_lowercase().as_str() {
                    "off" | "none" => MusicTrack::Off,
                    "blues" | "stlouis" => MusicTrack::StLouisBlues,
                    "jazz" | "cooljazz" => MusicTrack::MidnightCoolJazz,
                    "shuffle" | "boogie" => MusicTrack::BoogieBluesShuffle,
                    custom => MusicTrack::CustomFile(custom.to_string()),
                };
            }
            "--help" | "-h" => {
                println!(
                    "Midnight Battle Zone - Vulkan 3D Cat RTS\n\
                     Usage: midnight-battle-zone [OPTIONS]\n\
                     Options:\n\
                       --map <midnight_valley|twin_iron_ridge|shattered_basin>\n\
                       --ai <easy|medium|hard>\n\
                       --host [port]              Host a 2-player LAN game (default port 28765)\n\
                       --connect <ip:port>        Join a 2-player LAN game\n\
                       --fsr <off|ultra|quality|balanced|performance>\n\
                       --music <off|blues|jazz|shuffle|path.wav>\n\
                       --headless [--ticks N]     Run deterministic headless simulation"
                );
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    opts
}

fn trigger_meow_for_command(
    cmd: &GameCommand,
    world: &SimWorld,
    audio: &mut CatAudioEngine,
    callouts: &mut Vec<MeowCallout>,
) {
    let unit_ids: &[EntityId] = match cmd {
        GameCommand::MoveUnits { unit_ids, .. }
        | GameCommand::AttackTarget { unit_ids, .. }
        | GameCommand::StopUnits { unit_ids, .. } => unit_ids,
        GameCommand::ConstructBuilding { worker_ids, .. }
        | GameCommand::RepairBuilding { worker_ids, .. }
        | GameCommand::HarvestMine { worker_ids, .. } => worker_ids,
        GameCommand::TrainUnit { .. } => &[],
    };

    let mut played_worker_meow = false;
    let mut played_soldier_meow = false;

    for &uid in unit_ids {
        if let Some(u) = world.units.get(&uid) {
            if u.unit_type == "worker" && !played_worker_meow {
                played_worker_meow = true;
                audio.play_meow(MeowType::WorkerMeow);
                callouts.push(MeowCallout {
                    world_pos: Vec3::new(u.pos.x, 2.2, u.pos.y),
                    text: "MEOW!".to_string(),
                    ttl: 1.2,
                });
            } else if u.unit_type == "soldier" && !played_soldier_meow {
                played_soldier_meow = true;
                audio.play_meow(MeowType::SoldierMeow);
                callouts.push(MeowCallout {
                    world_pos: Vec3::new(u.pos.x, 2.4, u.pos.y),
                    text: "MROW!".to_string(),
                    ttl: 1.2,
                });
            }
        }
    }
}

fn update_selected_unit_ui(ui_state: &mut UiState, world: &SimWorld) {
    ui_state
        .selected_unit_ids
        .retain(|id| world.units.contains_key(id) || world.buildings.contains_key(id));

    let Some(&first_id) = ui_state.selected_unit_ids.first() else {
        ui_state.selected_unit = None;
        return;
    };

    if let Some(u) = world.units.get(&first_id) {
        let bp = world.registry.get_unit(&u.unit_type);
        let name = bp
            .map(|b| b.display_name.clone())
            .unwrap_or_else(|| u.unit_type.clone());
        let armor = bp.map(|b| b.armor).unwrap_or(0.0);
        let damage = bp.map(|b| b.attack_damage).unwrap_or(0.0);
        let is_worker = bp.map(|b| b.can_build || b.can_harvest).unwrap_or(false);
        let order_desc = match u.order {
            UnitOrderState::Idle => "Standing By".to_string(),
            UnitOrderState::Moving => "Moving".to_string(),
            UnitOrderState::AttackMoving => "Attack-Moving".to_string(),
            UnitOrderState::Attacking(t) => format!("Attacking #{}", t),
            UnitOrderState::Constructing(b) => format!("Building #{}", b),
            UnitOrderState::Repairing(b) => format!("Repairing #{}", b),
            UnitOrderState::Harvesting { returning_to_base, .. } => {
                if returning_to_base {
                    "Returning Ore".to_string()
                } else {
                    "Mining Ore".to_string()
                }
            }
        };

        ui_state.selected_unit = Some(SelectedUnitInfo {
            id: u.id,
            name,
            archetype: u.unit_type.clone(),
            hp: u.hp,
            max_hp: u.max_hp,
            armor,
            damage,
            order_desc,
            production_progress: None,
            is_building: false,
            is_worker,
            is_combat: !is_worker,
            is_barracks: false,
            is_command_center: false,
        });
    } else if let Some(b) = world.buildings.get(&first_id) {
        let bp = world.registry.get_building(&b.building_type);
        let name = bp
            .map(|bb| bb.display_name.clone())
            .unwrap_or_else(|| b.building_type.clone());
        let armor = bp.map(|bb| bb.armor).unwrap_or(2.0);
        let progress = if !b.completed {
            Some(b.build_progress_ticks as f32 / b.build_total_ticks.max(1) as f32)
        } else if let Some(first_q) = b.production_queue.first() {
            let total_t = world
                .registry
                .get_unit(first_q)
                .map(|u| u.build_time_ticks)
                .unwrap_or(100);
            Some(b.production_progress_ticks as f32 / total_t.max(1) as f32)
        } else {
            None
        };
        let order_desc = if !b.completed {
            "Under Construction".to_string()
        } else if !b.production_queue.is_empty() {
            format!("Training {} ({} queued)", b.production_queue[0], b.production_queue.len())
        } else {
            "Operational".to_string()
        };

        ui_state.selected_unit = Some(SelectedUnitInfo {
            id: b.id,
            name,
            archetype: b.building_type.clone(),
            hp: b.hp,
            max_hp: b.max_hp,
            armor,
            damage: 0.0,
            order_desc,
            production_progress: progress,
            is_building: true,
            is_worker: false,
            is_combat: false,
            is_barracks: b.building_type == "barracks",
            is_command_center: b.building_type == "command_center",
        });
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = parse_cli();

    // 1. Initialize Lua 5.4 Scripting Engine & Blueprint Registry
    let mut registry = BlueprintRegistry::new();
    let mut lua_engine = LuaScriptingEngine::new().expect("Failed to initialize Lua 5.4 engine");
    if let Err(e) = lua_engine.load_scripts(&mut registry) {
        log::warn!("Lua script load warning (using fallbacks): {}", e);
    }

    let selected_map = lua_engine
        .get_map(&cli.map_id)
        .or_else(|| lua_engine.maps().values().next())
        .cloned()
        .expect("At least one map definition must be available");

    // Headless simulation mode for CI / automated validation
    if cli.headless {
        log::info!(
            "Running headless simulation on map '{}' ({}x{}) vs {:?} AI for {} ticks...",
            selected_map.name,
            selected_map.width,
            selected_map.height,
            cli.difficulty,
            cli.headless_ticks
        );
        let mut world = SimWorld::new(selected_map, registry);
        for _ in 0..cli.headless_ticks {
            if let Ok(ai_cmds) = lua_engine.compute_ai_commands(&world, TEAM_PLAYER_1, AiDifficulty::Hard) {
                for cmd in ai_cmds {
                    world.apply_command(cmd);
                }
            }
            if let Ok(ai_cmds) = lua_engine.compute_ai_commands(&world, TEAM_PLAYER_2, cli.difficulty) {
                for cmd in ai_cmds {
                    world.apply_command(cmd);
                }
            }
            let calc_dmg = |base: f32, armor: f32, ap: f32| lua_engine.calculate_damage(base, armor, ap);
            world.step_tick(Some(&calc_dmg));
            if world.winner.is_some() {
                break;
            }
        }
        println!(
            "Headless run complete at tick {}: Team0(res={}, units={}, bldgs={}) vs Team1(res={}, units={}, bldgs={}), checksum=0x{:08X}",
            world.tick,
            world.team_resources[0],
            world.units.values().filter(|u| u.team == 0).count(),
            world.buildings.values().filter(|b| b.team == 0).count(),
            world.team_resources[1],
            world.units.values().filter(|u| u.team == 1).count(),
            world.buildings.values().filter(|b| b.team == 1).count(),
            world.compute_checksum()
        );
        return;
    }

    // 2. Initialize Window (X11 or Headless fallback), SDL3 Gamepad, & Vulkan 3D Renderer
    let mut window = AppWindow::open("Midnight Battle Zone - Vulkan Cat RTS", 1280, 720);
    let mut gamepad_mgr = GamepadManager::new();
    let (init_w, init_h) = window.dimensions();
    let mut renderer = VulkanRenderer::new(window.x11_handle(), init_w, init_h)
        .expect("Failed to initialize Vulkan Renderer");
    renderer.set_fsr_mode(cli.fsr_mode);

    // 3. Initialize Audio Engine (Cat Meows + Optional Public Domain Jazz/Blues Music)
    let mut audio = CatAudioEngine::new(true);
    if cli.music_track != MusicTrack::Off {
        audio.set_music_track(cli.music_track.clone());
    }

    // 4. Initialize UI, Camera, World, and LAN Discovery
    let mut ui_state = UiState::new();
    ui_state.fsr_mode = cli.fsr_mode;
    ui_state.selected_difficulty = cli.difficulty;
    ui_state.music_label = audio.current_track().label().to_string();

    if !lua_engine.maps().is_empty() {
        let mut map_opts = Vec::new();
        for map_id in lua_engine.map_order() {
            if let Some(m) = lua_engine.get_map(map_id) {
                let mines_per_side = m.resource_nodes.len() / 2;
                map_opts.push(MapOption {
                    id: m.id.clone(),
                    name: format!("{} [{} Mines/Side]", m.name, mines_per_side),
                    description: m.description.clone(),
                    width: m.width,
                    height: m.height,
                });
            }
        }
        if !map_opts.is_empty() {
            ui_state.maps = map_opts;
        }
    }

    let mut world = SimWorld::new(selected_map.clone(), registry.clone());
    let mut camera = IsometricCamera::new(Vec3::new(
        world.map.team_spawns[0][0],
        0.0,
        world.map.team_spawns[0][1],
    ));
    let hud_renderer = HudRenderer;

    let mut mdns = MdnsService::new(
        false,
        "CatCommander".to_string(),
        selected_map.id.clone(),
        DEFAULT_LAN_PORT,
    )
    .ok();
    let mut lockstep: Option<LockstepSession> = None;
    let mut pending_local_commands: Vec<GameCommand> = Vec::new();

    if let Some(port) = cli.host_port {
        ui_state.selected_mode = GameMode::HostLan;
        ui_state.current_screen = ScreenMode::InGame;
        mdns = MdnsService::new(true, "CatHost".to_string(), selected_map.id.clone(), port).ok();
        if let Ok(session) = LockstepSession::host(port, selected_map.id.clone()) {
            log::info!("Hosting LAN game on UDP port {} (mDNS active)", port);
            lockstep = Some(session);
        }
    } else if let Some(addr_str) = &cli.connect_addr {
        ui_state.selected_mode = GameMode::JoinLan;
        ui_state.current_screen = ScreenMode::InGame;
        if let Ok(sock_addr) = addr_str.parse::<SocketAddr>() {
            if let Ok(session) = LockstepSession::connect(sock_addr, "CatClient".to_string()) {
                log::info!("Connecting to LAN host at {}", sock_addr);
                ui_state.local_team = TEAM_PLAYER_2;
                camera.target = Vec3::new(
                    world.map.team_spawns[1][0],
                    0.0,
                    world.map.team_spawns[1][1],
                );
                lockstep = Some(session);
            }
        }
    } else if cli.start_immediately {
        ui_state.current_screen = ScreenMode::InGame;
    }

    let start_instant = Instant::now();
    let mut last_frame_instant = Instant::now();
    let mut sim_accumulator = Duration::ZERO;
    let mut fps_timer = Instant::now();
    let mut frame_counter = 0u32;

    while !window.should_close() {
        let now = Instant::now();
        let dt = now.duration_since(last_frame_instant);
        last_frame_instant = now;
        let dt_secs = dt.as_secs_f32().min(0.1);

        frame_counter += 1;
        if now.duration_since(fps_timer) >= Duration::from_secs(1) {
            ui_state.fps = frame_counter;
            frame_counter = 0;
            fps_timer = now;
        }

        // Poll mDNS LAN discovery
        let now_ms = now.duration_since(start_instant).as_millis() as u64;
        if let Some(mdns_svc) = &mut mdns {
            let discovered = mdns_svc.poll(now_ms);
            if !discovered.is_empty() {
                ui_state.discovered_lobbies = discovered
                    .into_iter()
                    .map(|d| UiLanLobbyInfo {
                        host_name: d.host_name,
                        map_name: d.map_id,
                        ip_addr: d.host_addr.to_string(),
                        ping_ms: 4,
                    })
                    .collect();
            }
        }

        // Decay floating "MEOW!" / "MROW!" callout bubbles
        for callout in &mut ui_state.meow_callouts {
            callout.ttl -= dt_secs;
            callout.world_pos.y += dt_secs * 1.2;
        }
        ui_state.meow_callouts.retain(|c| c.ttl > 0.0);

        let _ = window.poll_events();
        let gp_state = gamepad_mgr.poll(window.input_mut(), dt_secs);
        let input = window.input().clone();
        let viewport_size = Vec2::new(input.window_width as f32, input.window_height as f32);
        let map_size = Vec2::new(world.map.width, world.map.height);

        // Camera panning & zooming when InGame (Keyboard + SDL3 Gamepad Right Stick & Triggers)
        if ui_state.current_screen == ScreenMode::InGame {
            let mut pan = gp_state.right_stick;
            if input.is_key_down(KeyCode::Up) {
                pan.y -= 1.0;
            }
            if input.is_key_down(KeyCode::Down) {
                pan.y += 1.0;
            }
            if input.is_key_down(KeyCode::Left) {
                pan.x -= 1.0;
            }
            if input.is_key_down(KeyCode::Right) {
                pan.x += 1.0;
            }
            if pan != Vec2::ZERO {
                camera.pan(
                    pan.x * dt_secs * 28.0,
                    pan.y * dt_secs * 28.0,
                    map_size.x,
                    map_size.y,
                );
            }
            let total_scroll = input.scroll_delta + gp_state.zoom_delta * dt_secs * 8.0;
            if total_scroll.abs() > 0.01 {
                camera.zoom(-total_scroll * 3.5);
            }
        }

        // Handle UI actions
        let ui_actions = ui_state.handle_input(&input, &camera, viewport_size, map_size);
        for action in ui_actions {
            match action {
                UiAction::StartGame {
                    mode,
                    difficulty,
                    map_id,
                } => {
                    ui_state.selected_mode = mode;
                    ui_state.selected_difficulty = difficulty;
                    let chosen_map = lua_engine
                        .get_map(&map_id)
                        .cloned()
                        .unwrap_or_else(|| selected_map.clone());
                    world = SimWorld::new(chosen_map.clone(), registry.clone());
                    ui_state.local_team = TEAM_PLAYER_1;
                    ui_state.selected_unit_ids.clear();
                    ui_state.selected_unit = None;
                    camera.target = Vec3::new(
                        world.map.team_spawns[0][0],
                        0.0,
                        world.map.team_spawns[0][1],
                    );

                    if mode == GameMode::HostLan {
                        mdns = MdnsService::new(
                            true,
                            "CatHost".to_string(),
                            chosen_map.id.clone(),
                            DEFAULT_LAN_PORT,
                        )
                        .ok();
                        if let Ok(session) = LockstepSession::host(DEFAULT_LAN_PORT, chosen_map.id) {
                            lockstep = Some(session);
                        }
                    } else {
                        lockstep = None;
                    }
                }
                UiAction::ConnectLan { ip } => {
                    let target_ip = ui_state
                        .discovered_lobbies
                        .first()
                        .map(|l| l.ip_addr.clone())
                        .filter(|_| ip.is_empty())
                        .unwrap_or(ip);
                    if let Ok(addr) = target_ip.parse::<SocketAddr>() {
                        if let Ok(session) = LockstepSession::connect(addr, "CatPeer".to_string()) {
                            ui_state.local_team = TEAM_PLAYER_2;
                            camera.target = Vec3::new(
                                world.map.team_spawns[1][0],
                                0.0,
                                world.map.team_spawns[1][1],
                            );
                            lockstep = Some(session);
                        }
                    }
                }
                UiAction::IssueCommand(cmd) => {
                    trigger_meow_for_command(&cmd, &world, &mut audio, &mut ui_state.meow_callouts);
                    pending_local_commands.push(cmd);
                }
                UiAction::SetPlacementMode(mode) => {
                    ui_state.placement_mode = mode;
                }
                UiAction::MoveCameraToGround(pos) => {
                    camera.target = Vec3::new(
                        pos.x.clamp(0.0, map_size.x),
                        0.0,
                        pos.y.clamp(0.0, map_size.y),
                    );
                }
                UiAction::TogglePause => {
                    world.paused = ui_state.paused;
                    if let Some(session) = &mut lockstep {
                        session.send_pause_state(world.paused);
                    }
                }
                UiAction::ToggleMenu | UiAction::ReturnToMenu => {
                    ui_state.current_screen = ScreenMode::MainMenu;
                }
                UiAction::SetFsrMode(mode) => {
                    renderer.set_fsr_mode(mode);
                }
                UiAction::CycleMusic => {
                    let track = audio.cycle_music_track();
                    ui_state.music_label = track.label().to_string();
                }
            }
        }

        // World selection & right-click tactical orders when cursor is in the 3D battlefield viewport
        let in_hud_panel = input.mouse_pos.y < 48.0
            || (input.mouse_pos.y > viewport_size.y - 210.0
                && (input.mouse_pos.x < 220.0
                    || (input.mouse_pos.x >= 225.0 && input.mouse_pos.x <= 580.0)
                    || input.mouse_pos.x >= viewport_size.x - 365.0));

        if ui_state.current_screen == ScreenMode::InGame && !in_hud_panel {
            // Track drag box
            if input.left_down {
                if let Some(start) = input.drag_start {
                    if start.distance(input.mouse_pos) > 8.0 {
                        ui_state.box_drag_start = Some(start);
                        ui_state.box_drag_current = Some(input.mouse_pos);
                    }
                }
            }

            if input.left_released {
                if let (Some(box_start), Some(box_end)) =
                    (ui_state.box_drag_start.take(), ui_state.box_drag_current.take())
                {
                    let min_x = box_start.x.min(box_end.x);
                    let max_x = box_start.x.max(box_end.x);
                    let min_y = box_start.y.min(box_end.y);
                    let max_y = box_start.y.max(box_end.y);

                    let mut boxed = Vec::new();
                    for u in world.units.values() {
                        if u.team == ui_state.local_team {
                            if let Some(sp) = camera.world_to_screen(
                                Vec3::new(u.pos.x, 0.5, u.pos.y),
                                viewport_size,
                            ) {
                                if sp.x >= min_x && sp.x <= max_x && sp.y >= min_y && sp.y <= max_y
                                {
                                    boxed.push(u.id);
                                }
                            }
                        }
                    }
                    if !boxed.is_empty() {
                        ui_state.selected_unit_ids = boxed;
                    }
                } else if ui_state.placement_mode.as_deref() == Some("repair") {
                    if let Some(ground) = camera.screen_to_ground(input.mouse_pos, viewport_size) {
                        if let Some(b) = world
                            .buildings
                            .values()
                            .filter(|b| b.team == ui_state.local_team && b.hp < b.max_hp)
                            .min_by(|a, b| {
                                a.pos
                                    .distance_squared(ground)
                                    .partial_cmp(&b.pos.distance_squared(ground))
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                        {
                            let cmd = GameCommand::RepairBuilding {
                                team: ui_state.local_team,
                                worker_ids: ui_state.selected_unit_ids.clone(),
                                building_id: b.id,
                            };
                            trigger_meow_for_command(
                                &cmd,
                                &world,
                                &mut audio,
                                &mut ui_state.meow_callouts,
                            );
                            pending_local_commands.push(cmd);
                        }
                    }
                    ui_state.placement_mode = None;
                } else if ui_state.placement_mode.is_none() {
                    if let Some(ground) = camera.screen_to_ground(input.mouse_pos, viewport_size) {
                        if ui_state.attack_move_mode && !ui_state.selected_unit_ids.is_empty() {
                            let cmd = GameCommand::MoveUnits {
                                team: ui_state.local_team,
                                unit_ids: ui_state.selected_unit_ids.clone(),
                                target: [ground.x, ground.y],
                                attack_move: true,
                            };
                            trigger_meow_for_command(
                                &cmd,
                                &world,
                                &mut audio,
                                &mut ui_state.meow_callouts,
                            );
                            pending_local_commands.push(cmd);
                            ui_state.attack_move_mode = false;
                        } else {
                            // Select closest friendly unit or building near click
                            let mut picked = None;
                            let mut best_dist = 3.2_f32;
                            for u in world.units.values() {
                                if u.team == ui_state.local_team {
                                    let d = u.pos.distance(ground);
                                    if d < best_dist {
                                        best_dist = d;
                                        picked = Some(u.id);
                                    }
                                }
                            }
                            if picked.is_none() {
                                for b in world.buildings.values() {
                                    if b.team == ui_state.local_team {
                                        let d = b.pos.distance(ground);
                                        if d < 4.5 {
                                            picked = Some(b.id);
                                            break;
                                        }
                                    }
                                }
                            }
                            if let Some(id) = picked {
                                ui_state.selected_unit_ids = vec![id];
                            } else {
                                ui_state.selected_unit_ids.clear();
                            }
                        }
                    }
                }
            }

            // Right-click context command (Attack, Harvest Mine/Node, Repair Damaged Building, or Move)
            if input.right_pressed && !ui_state.selected_unit_ids.is_empty() {
                if let Some(ground) = camera.screen_to_ground(input.mouse_pos, viewport_size) {
                    let has_worker = ui_state.selected_unit_ids.iter().any(|id| {
                        world
                            .units
                            .get(id)
                            .map(|u| u.unit_type == "worker")
                            .unwrap_or(false)
                    });

                    // 1. Check visible enemy unit or building
                    let enemy_target = world
                        .units
                        .values()
                        .filter(|u| {
                            u.team != ui_state.local_team
                                && world.fog.is_pos_visible(ui_state.local_team, u.pos)
                                && u.pos.distance(ground) < 3.0
                        })
                        .map(|u| u.id)
                        .next()
                        .or_else(|| {
                            world
                                .buildings
                                .values()
                                .filter(|b| {
                                    b.team != ui_state.local_team
                                        && world.fog.is_pos_visible(ui_state.local_team, b.pos)
                                        && b.pos.distance(ground) < 4.5
                                })
                                .map(|b| b.id)
                                .next()
                        });

                    // 2. Check resource node or friendly mine (for Worker Cats)
                    let harvest_target = if has_worker {
                        world
                            .buildings
                            .values()
                            .find(|b| {
                                b.team == ui_state.local_team
                                    && b.building_type == "mine"
                                    && b.pos.distance(ground) < 4.0
                            })
                            .map(|b| b.id)
                            .or_else(|| {
                                world
                                    .resource_nodes
                                    .values()
                                    .find(|n| n.pos.distance(ground) < 4.0)
                                    .map(|n| n.id)
                            })
                    } else {
                        None
                    };

                    // 3. Check damaged friendly building (for Worker Cats)
                    let repair_target = if has_worker {
                        world
                            .buildings
                            .values()
                            .find(|b| {
                                b.team == ui_state.local_team
                                    && b.hp < b.max_hp
                                    && b.pos.distance(ground) < 4.2
                            })
                            .map(|b| b.id)
                    } else {
                        None
                    };

                    let cmd = if let Some(target_id) = enemy_target {
                        GameCommand::AttackTarget {
                            team: ui_state.local_team,
                            unit_ids: ui_state.selected_unit_ids.clone(),
                            target_id,
                        }
                    } else if let Some(mine_or_node_id) = harvest_target {
                        GameCommand::HarvestMine {
                            team: ui_state.local_team,
                            worker_ids: ui_state.selected_unit_ids.clone(),
                            target_id: mine_or_node_id,
                        }
                    } else if let Some(building_id) = repair_target {
                        GameCommand::RepairBuilding {
                            team: ui_state.local_team,
                            worker_ids: ui_state.selected_unit_ids.clone(),
                            building_id,
                        }
                    } else {
                        GameCommand::MoveUnits {
                            team: ui_state.local_team,
                            unit_ids: ui_state.selected_unit_ids.clone(),
                            target: [ground.x, ground.y],
                            attack_move: ui_state.attack_move_mode,
                        }
                    };

                    ui_state.attack_move_mode = false;
                    trigger_meow_for_command(&cmd, &world, &mut audio, &mut ui_state.meow_callouts);
                    pending_local_commands.push(cmd);
                }
            }
        }

        // Poll LAN lockstep session if active
        if let Some(session) = &mut lockstep {
            session.poll_network();
            if let Some(remote_paused) = session.take_pause_update() {
                world.paused = remote_paused;
                ui_state.paused = remote_paused;
            }
        }

        // Step deterministic 20 Hz simulation
        if ui_state.current_screen == ScreenMode::InGame {
            sim_accumulator += dt;
            while sim_accumulator >= SIM_TICK_DURATION {
                sim_accumulator -= SIM_TICK_DURATION;

                if world.paused {
                    break;
                }

                if let Some(session) = &mut lockstep {
                    if session.is_connected() {
                        let next_tick = world.tick + 1;
                        let local_cmds = std::mem::take(&mut pending_local_commands);
                        session.submit_local_tick(next_tick, local_cmds, world.compute_checksum());
                        if let Some((combined_cmds, _chk)) = session.take_ready_tick(next_tick) {
                            for cmd in combined_cmds {
                                world.apply_command(cmd);
                            }
                            let calc_dmg = |base: f32, armor: f32, ap: f32| {
                                lua_engine.calculate_damage(base, armor, ap)
                            };
                            world.step_tick(Some(&calc_dmg));
                        }
                    }
                } else {
                    for cmd in pending_local_commands.drain(..) {
                        world.apply_command(cmd);
                    }
                    if let Ok(ai_cmds) = lua_engine.compute_ai_commands(
                        &world,
                        TEAM_PLAYER_2,
                        ui_state.selected_difficulty,
                    ) {
                        for cmd in ai_cmds {
                            world.apply_command(cmd);
                        }
                    }
                    let calc_dmg =
                        |base: f32, armor: f32, ap: f32| lua_engine.calculate_damage(base, armor, ap);
                    world.step_tick(Some(&calc_dmg));
                }
            }
        }

        // Synchronize UI state with simulation world
        let team_idx = ui_state.local_team as usize;
        ui_state.resources = world.team_resources[team_idx.min(1)];
        ui_state.supply_used = world.team_supply_used[team_idx.min(1)];
        ui_state.supply_cap = world.team_supply_cap[team_idx.min(1)];
        ui_state.game_ticks = world.tick;
        ui_state.elapsed_seconds = world.tick as f32 / SIM_TICK_HZ as f32;
        ui_state.paused = world.paused;
        ui_state.game_over = world.winner.map(|w| w == ui_state.local_team);
        update_selected_unit_ui(&mut ui_state, &world);

        // Build 3D render instances (filtered by local team's Fog of War) and 2D HUD vertices
        let instances =
            world.build_render_instances(ui_state.local_team, &ui_state.selected_unit_ids);
        let fog_rgba = world.fog.rgba_texture_data(ui_state.local_team);
        let fog_dims = (
            world.fog.width_cells() as u32,
            world.fog.height_cells() as u32,
        );
        let ui_verts =
            hud_renderer.render(&ui_state, viewport_size, map_size, &instances, &camera);

        if let Err(e) = renderer.render_frame(
            &camera,
            map_size,
            &fog_rgba,
            fog_dims,
            &instances,
            &ui_verts,
        ) {
            log::debug!("Render frame info: {}", e);
        }

        std::thread::sleep(Duration::from_millis(4));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_lua_cat_units_and_extensibility() {
        let mut registry = BlueprintRegistry::new();
        let mut lua = LuaScriptingEngine::new().expect("Lua engine init");
        lua.load_scripts(&mut registry).expect("Load scripts");

        let worker = registry.get_unit("worker").expect("Worker Cat must exist");
        let soldier = registry.get_unit("soldier").expect("Soldier Cat must exist");
        let sniper = registry.get_unit("sniper").expect("Sniper Cat must exist");

        assert_eq!(worker.display_name, "Worker Cat");
        assert_eq!(soldier.display_name, "Soldier Cat");
        assert_eq!(sniper.display_name, "Sniper Cat");

        // Snipers must have less armor and stronger weapons than Soldiers
        assert!(sniper.armor < soldier.armor, "Sniper Cat must have less armor than Soldier Cat");
        assert!(
            sniper.attack_damage > soldier.attack_damage,
            "Sniper Cat must have stronger weapon damage than Soldier Cat"
        );
        assert!(
            sniper.attack_range > soldier.attack_range,
            "Sniper Cat must have longer range than Soldier Cat"
        );

        // Verify extensibility: register a new custom cat unit dynamically
        let custom_lua = r#"
            return {
                {
                    id = "panther_tank_cat",
                    display_name = "Panther Tank Cat",
                    mesh_archetype = "soldier",
                    scale = 1.4,
                    cost = 250,
                    supply_cost = 3,
                    build_time_ticks = 160,
                    max_hp = 320.0,
                    armor = 8.0,
                    armor_piercing = 6.0,
                    move_speed = 3.2,
                    vision_range = 14.0,
                    attack_damage = 55.0,
                    attack_range = 9.0,
                    attack_cooldown_ticks = 28,
                    projectile_speed = 30.0,
                    can_harvest = false,
                    can_build = false,
                    can_repair = false,
                    producer_building = "barracks",
                    hotkey = "T"
                }
            }
        "#;
        let tab: mlua::Table = lua.lua.load(custom_lua).eval().unwrap();
        for pair in tab.pairs::<mlua::Value, mlua::Table>() {
            let (_, t) = pair.unwrap();
            let id: String = t.get("id").unwrap();
            let display_name: String = t.get("display_name").unwrap();
            registry.register_unit(crate::types::UnitBlueprint {
                id,
                display_name,
                mesh_archetype: t.get("mesh_archetype").unwrap(),
                scale: t.get("scale").unwrap(),
                cost: t.get("cost").unwrap(),
                supply_cost: t.get("supply_cost").unwrap(),
                build_time_ticks: t.get("build_time_ticks").unwrap(),
                max_hp: t.get("max_hp").unwrap(),
                armor: t.get("armor").unwrap(),
                armor_piercing: t.get("armor_piercing").unwrap(),
                move_speed: t.get("move_speed").unwrap(),
                vision_range: t.get("vision_range").unwrap(),
                attack_damage: t.get("attack_damage").unwrap(),
                attack_range: t.get("attack_range").unwrap(),
                attack_cooldown_ticks: t.get("attack_cooldown_ticks").unwrap(),
                projectile_speed: t.get("projectile_speed").unwrap(),
                can_harvest: t.get("can_harvest").unwrap(),
                can_build: t.get("can_build").unwrap(),
                can_repair: t.get("can_repair").unwrap(),
                producer_building: t.get("producer_building").unwrap(),
                hotkey: t.get("hotkey").unwrap(),
            });
        }
        assert!(registry.get_unit("panther_tank_cat").is_some());
    }

    #[test]
    fn test_all_maps_have_3_to_6_mines_per_side() {
        let mut registry = BlueprintRegistry::new();
        let mut lua = LuaScriptingEngine::new().expect("Lua init");
        lua.load_scripts(&mut registry).expect("Load scripts");

        for map_id in ["midnight_valley", "twin_iron_ridge", "shattered_basin"] {
            let map = lua
                .get_map(map_id)
                .unwrap_or_else(|| panic!("Map {} must exist", map_id));
            let side0_nodes = map.resource_nodes.iter().filter(|n| n.side_hint == 0).count();
            let side1_nodes = map.resource_nodes.iter().filter(|n| n.side_hint == 1).count();
            assert!(
                (3..=6).contains(&side0_nodes),
                "Map {} side 0 has {} nodes (expected 3..=6)",
                map_id,
                side0_nodes
            );
            assert!(
                (3..=6).contains(&side1_nodes),
                "Map {} side 1 has {} nodes (expected 3..=6)",
                map_id,
                side1_nodes
            );
            assert_eq!(side0_nodes, side1_nodes, "Map {} must have symmetric mine counts", map_id);
        }
    }

    #[test]
    fn test_economy_construction_repair_fog_and_victory() {
        let mut registry = BlueprintRegistry::new();
        let mut lua = LuaScriptingEngine::new().unwrap();
        lua.load_scripts(&mut registry).unwrap();

        let map = lua.get_map("midnight_valley").unwrap().clone();
        let mut world = SimWorld::new(map, registry);

        // 1. Fog of war: Team 0 cannot see Team 1's base at start
        let team1_spawn = Vec2::new(world.map.team_spawns[1][0], world.map.team_spawns[1][1]);
        assert!(
            !world.fog.is_pos_visible(0, team1_spawn),
            "Enemy base must be hidden by Fog of War initially"
        );

        // 2. Worker Cat building repair consumes resources and restores HP
        let cc_id = world
            .buildings
            .values()
            .find(|b| b.team == 0 && b.building_type == "command_center")
            .unwrap()
            .id;
        world.buildings.get_mut(&cc_id).unwrap().hp = 500.0;
        let initial_res = world.team_resources[0];

        let worker_ids: Vec<EntityId> = world
            .units
            .values()
            .filter(|u| u.team == 0 && u.unit_type == "worker")
            .map(|u| u.id)
            .collect();

        world.apply_command(GameCommand::RepairBuilding {
            team: 0,
            worker_ids,
            building_id: cc_id,
        });

        for _ in 0..25 {
            world.step_tick(None);
        }

        let repaired_hp = world.buildings.get(&cc_id).unwrap().hp;
        assert!(repaired_hp > 500.0, "Worker Cats must repair damaged building HP");
        assert!(
            world.team_resources[0] < initial_res,
            "Repairing a building must consume mined resources"
        );

        // 3. Pause freezes simulation for both players
        let tick_before_pause = world.tick;
        world.paused = true;
        world.step_tick(None);
        assert_eq!(world.tick, tick_before_pause, "Paused world must not advance ticks");
        world.paused = false;

        // 4. Destroying all buildings of Team 1 awards Victory to Team 0
        let team1_buildings: Vec<EntityId> = world
            .buildings
            .values()
            .filter(|b| b.team == 1)
            .map(|b| b.id)
            .collect();
        for bid in team1_buildings {
            world.buildings.get_mut(&bid).unwrap().hp = 0.0;
        }
        world.step_tick(None);
        assert_eq!(world.winner, Some(0), "Team 0 must win when all Team 1 buildings are destroyed");
    }

    #[test]
    fn test_ai_three_difficulty_levels() {
        let mut registry = BlueprintRegistry::new();
        let mut lua = LuaScriptingEngine::new().unwrap();
        lua.load_scripts(&mut registry).unwrap();
        let map = lua.get_map("twin_iron_ridge").unwrap().clone();

        for diff in [AiDifficulty::Easy, AiDifficulty::Medium, AiDifficulty::Hard] {
            let mut world = SimWorld::new(map.clone(), registry.clone());
            let mut total_ai_cmds = 0;
            for _ in 0..95 {
                let cmds = lua
                    .compute_ai_commands(&world, TEAM_PLAYER_2, diff)
                    .expect("AI script should execute cleanly");
                total_ai_cmds += cmds.len();
                for c in cmds {
                    world.apply_command(c);
                }
                world.step_tick(None);
            }
            assert!(
                total_ai_cmds > 0,
                "AI difficulty {:?} should issue economy/build/train commands",
                diff
            );
        }
    }
}

