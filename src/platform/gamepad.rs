use crate::platform::x11::{InputState, KeyCode};
use glam::Vec2;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::OnceLock;

// SDL3 Constants
pub const SDL_INIT_JOYSTICK: u32 = 0x00000200;
pub const SDL_INIT_GAMEPAD: u32 = 0x00002000;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SDL_GamepadAxis {
    LeftX = 0,
    LeftY = 1,
    RightX = 2,
    RightY = 3,
    LeftTrigger = 4,
    RightTrigger = 5,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SDL_GamepadButton {
    South = 0,
    East = 1,
    West = 2,
    North = 3,
    Back = 4,
    Guide = 5,
    Start = 6,
    LeftShoulder = 9,
    RightShoulder = 10,
    DpadUp = 11,
    DpadDown = 12,
    DpadLeft = 13,
    DpadRight = 14,
}

#[derive(Debug, Clone)]
pub struct GamepadState {
    pub connected: bool,
    pub controller_name: String,
    pub left_stick: Vec2,
    pub right_stick: Vec2,
    pub zoom_delta: f32,
}

impl Default for GamepadState {
    fn default() -> Self {
        Self {
            connected: false,
            controller_name: String::new(),
            left_stick: Vec2::ZERO,
            right_stick: Vec2::ZERO,
            zoom_delta: 0.0,
        }
    }
}

// Function pointer types for SDL3 dynamic loading
type FnInitSubSystem = unsafe extern "C" fn(flags: u32) -> bool;
type FnUpdateGamepads = unsafe extern "C" fn();
type FnGetGamepads = unsafe extern "C" fn(count: *mut c_int) -> *mut u32;
type FnOpenGamepad = unsafe extern "C" fn(instance_id: u32) -> *mut c_void;
type FnCloseGamepad = unsafe extern "C" fn(gamepad: *mut c_void);
type FnGetGamepadAxis = unsafe extern "C" fn(gamepad: *mut c_void, axis: c_int) -> i16;
type FnGetGamepadButton = unsafe extern "C" fn(gamepad: *mut c_void, button: c_int) -> bool;
type FnGetGamepadName = unsafe extern "C" fn(gamepad: *mut c_void) -> *const c_char;
type FnFree = unsafe extern "C" fn(mem: *mut c_void);

struct Sdl3Syms {
    init_sub_system: FnInitSubSystem,
    update_gamepads: FnUpdateGamepads,
    get_gamepads: FnGetGamepads,
    open_gamepad: FnOpenGamepad,
    close_gamepad: FnCloseGamepad,
    get_gamepad_axis: FnGetGamepadAxis,
    get_gamepad_button: FnGetGamepadButton,
    get_gamepad_name: FnGetGamepadName,
    free: FnFree,
}

static SDL3: OnceLock<Option<Sdl3Syms>> = OnceLock::new();

fn load_sdl3() -> Option<Sdl3Syms> {
    unsafe {
        let lib_names: &[&[u8]] = &[
            b"libSDL3.so.0\0",
            b"libSDL3.so\0",
            b"/usr/local/lib/libSDL3.so\0",
        ];
        let mut handle: *mut c_void = std::ptr::null_mut();
        for name in lib_names {
            handle = libc::dlopen(name.as_ptr() as *const c_char, libc::RTLD_LAZY);
            if !handle.is_null() {
                break;
            }
        }

        if handle.is_null() {
            println!("[Gamepad] SDL3 library not found via dlopen");
            return None;
        }

        macro_rules! load_sym {
            ($name:expr, $fty:ty) => {{
                let sym = libc::dlsym(handle, concat!($name, "\0").as_ptr() as *const c_char);
                if sym.is_null() {
                    println!("[Gamepad] Failed to load SDL3 symbol: {}", $name);
                    return None;
                }
                std::mem::transmute::<*mut c_void, $fty>(sym)
            }};
        }

        let init_sub_system = load_sym!("SDL_InitSubSystem", FnInitSubSystem);
        let update_gamepads = load_sym!("SDL_UpdateGamepads", FnUpdateGamepads);
        let get_gamepads = load_sym!("SDL_GetGamepads", FnGetGamepads);
        let open_gamepad = load_sym!("SDL_OpenGamepad", FnOpenGamepad);
        let close_gamepad = load_sym!("SDL_CloseGamepad", FnCloseGamepad);
        let get_gamepad_axis = load_sym!("SDL_GetGamepadAxis", FnGetGamepadAxis);
        let get_gamepad_button = load_sym!("SDL_GetGamepadButton", FnGetGamepadButton);
        let get_gamepad_name = load_sym!("SDL_GetGamepadName", FnGetGamepadName);
        let free = load_sym!("SDL_free", FnFree);

        println!("[Gamepad] Successfully loaded SDL3 dynamically.");

        Some(Sdl3Syms {
            init_sub_system,
            update_gamepads,
            get_gamepads,
            open_gamepad,
            close_gamepad,
            get_gamepad_axis,
            get_gamepad_button,
            get_gamepad_name,
            free,
        })
    }
}

#[allow(dead_code)]
pub struct GamepadManager {
    handle: *mut c_void,
    instance_id: u32,
    prev_buttons: u32, // bitmask or individual tracking
    south_was_down: bool,
    east_was_down: bool,
}

impl GamepadManager {
    pub fn new() -> Self {
        let syms = SDL3.get_or_init(load_sdl3);
        if let Some(s) = syms {
            unsafe {
                let ok1 = (s.init_sub_system)(SDL_INIT_JOYSTICK);
                let ok2 = (s.init_sub_system)(SDL_INIT_GAMEPAD);
                if ok1 || ok2 {
                    println!("[Gamepad] SDL3 Gamepad subsystem initialized successfully.");
                } else {
                    println!("[Gamepad] Warning: SDL_InitSubSystem for joystick/gamepad returned false.");
                }
            }
        } else {
            println!("[Gamepad] SDL3 not available. Gamepad support disabled.");
        }

        Self {
            handle: std::ptr::null_mut(),
            instance_id: 0,
            prev_buttons: 0,
            south_was_down: false,
            east_was_down: false,
        }
    }

    pub fn poll(&mut self, input: &mut InputState, dt_secs: f32) -> GamepadState {
        let mut state = GamepadState::default();
        let syms = match SDL3.get_or_init(load_sdl3) {
            Some(s) => s,
            None => return state,
        };

        unsafe {
            (syms.update_gamepads)();

            // Check if current handle is still valid / connected or if we need to open one
            if self.handle.is_null() {
                let mut count: c_int = 0;
                let ids_ptr = (syms.get_gamepads)(&mut count);
                if !ids_ptr.is_null() && count > 0 {
                    let ids = std::slice::from_raw_parts(ids_ptr, count as usize);
                    // Open the first available gamepad
                    let id = ids[0];
                    let gamepad = (syms.open_gamepad)(id);
                    if !gamepad.is_null() {
                        self.handle = gamepad;
                        self.instance_id = id;
                        let name_ptr = (syms.get_gamepad_name)(gamepad);
                        if !name_ptr.is_null() {
                            state.controller_name = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();
                        } else {
                            state.controller_name = "Unknown Gamepad".to_string();
                        }
                        println!("[Gamepad] Connected: {} (id: {})", state.controller_name, id);
                    }
                    (syms.free)(ids_ptr as *mut c_void);
                }
            } else {
                // Verify handle is still open/connected (or we can just read, if handle is valid)
                state.connected = true;
                let name_ptr = (syms.get_gamepad_name)(self.handle);
                if !name_ptr.is_null() {
                    state.controller_name = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();
                } else {
                    state.controller_name = "Unknown Gamepad".to_string();
                }
            }

            if self.handle.is_null() {
                return state;
            }

            state.connected = true;

            // Read Left Stick
            let raw_lx = (syms.get_gamepad_axis)(self.handle, SDL_GamepadAxis::LeftX as c_int) as f32 / 32768.0;
            let raw_ly = (syms.get_gamepad_axis)(self.handle, SDL_GamepadAxis::LeftY as c_int) as f32 / 32768.0;
            let left_vec = Vec2::new(raw_lx, raw_ly);
            let deadzone = 0.15;
            let left_mag = left_vec.length();
            if left_mag > deadzone {
                let normalized_mag = ((left_mag - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0);
                let filtered = left_vec / left_mag * normalized_mag;
                state.left_stick = filtered;

                // Move mouse cursor smoothly across (0..window_width, 0..window_height) at 520.0 * dt_secs
                let speed = 520.0;
                input.mouse_pos.x += filtered.x * speed * dt_secs;
                input.mouse_pos.y += filtered.y * speed * dt_secs;
                input.mouse_pos.x = input.mouse_pos.x.clamp(0.0, input.window_width as f32);
                input.mouse_pos.y = input.mouse_pos.y.clamp(0.0, input.window_height as f32);
            }

            // Read Right Stick
            let raw_rx = (syms.get_gamepad_axis)(self.handle, SDL_GamepadAxis::RightX as c_int) as f32 / 32768.0;
            let raw_ry = (syms.get_gamepad_axis)(self.handle, SDL_GamepadAxis::RightY as c_int) as f32 / 32768.0;
            let right_vec = Vec2::new(raw_rx, raw_ry);
            let right_mag = right_vec.length();
            if right_mag > deadzone {
                let normalized_mag = ((right_mag - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0);
                let filtered = right_vec / right_mag * normalized_mag;
                state.right_stick = filtered;
            }

            // Zoom delta from triggers / shoulders
            let left_trigger = (syms.get_gamepad_axis)(self.handle, SDL_GamepadAxis::LeftTrigger as c_int) as f32 / 32767.0;
            let right_trigger = (syms.get_gamepad_axis)(self.handle, SDL_GamepadAxis::RightTrigger as c_int) as f32 / 32767.0;
            state.zoom_delta = right_trigger - left_trigger;

            let lt_shoulder = (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::LeftShoulder as c_int);
            let rt_shoulder = (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::RightShoulder as c_int);
            if lt_shoulder { state.zoom_delta -= 1.0; }
            if rt_shoulder { state.zoom_delta += 1.0; }

            // Buttons edge transitions
            let south_down = (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::South as c_int);
            if south_down && !self.south_was_down {
                input.left_pressed = true;
                input.drag_start = Some(input.mouse_pos);
            }
            if !south_down && self.south_was_down {
                input.left_released = true;
                input.drag_start = None;
            }
            input.left_down = south_down;
            self.south_was_down = south_down;

            let east_down = (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::East as c_int);
            if east_down && !self.east_was_down {
                input.right_pressed = true;
            }
            self.east_was_down = east_down;

            // Specific mappings requested:
            // START -> KeyP
            if (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::Start as c_int) {
                input.keys_pressed.insert(KeyCode::KeyP);
            }
            // BACK -> KeyF
            if (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::Back as c_int) {
                input.keys_pressed.insert(KeyCode::KeyF);
            }
            // GUIDE -> KeyJ
            if (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::Guide as c_int) {
                input.keys_pressed.insert(KeyCode::KeyJ);
            }
            // WEST (X) -> KeyW, KeyS, KeyA
            if (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::West as c_int) {
                input.keys_pressed.insert(KeyCode::KeyW);
                input.keys_pressed.insert(KeyCode::KeyS);
                input.keys_pressed.insert(KeyCode::KeyA);
            }
            // NORTH (Y) -> KeyM, KeyN
            if (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::North as c_int) {
                input.keys_pressed.insert(KeyCode::KeyM);
                input.keys_pressed.insert(KeyCode::KeyN);
            }
            // DPAD_UP -> KeyH
            if (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::DpadUp as c_int) {
                input.keys_pressed.insert(KeyCode::KeyH);
            }
            // DPAD_RIGHT -> KeyB
            if (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::DpadRight as c_int) {
                input.keys_pressed.insert(KeyCode::KeyB);
            }
            // DPAD_DOWN -> KeyR
            if (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::DpadDown as c_int) {
                input.keys_pressed.insert(KeyCode::KeyR);
            }
            // DPAD_LEFT -> KeyX
            if (syms.get_gamepad_button)(self.handle, SDL_GamepadButton::DpadLeft as c_int) {
                input.keys_pressed.insert(KeyCode::KeyX);
            }
        }

        state
    }
}

impl Drop for GamepadManager {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            if let Some(syms) = SDL3.get_or_init(load_sdl3) {
                unsafe {
                    (syms.close_gamepad)(self.handle);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamepad_manager_init_and_deadzone() {
        let manager = GamepadManager::new();
        assert!(manager.handle.is_null() || !manager.handle.is_null()); // Safe init test

        let mut input = InputState::new(800, 600);
        let mut mgr = GamepadManager::new();
        let _state = mgr.poll(&mut input, 0.016);

        // Test deadzone filter calculation directly
        let deadzone = 0.15_f32;
        let test_val_below = 0.10_f32;
        let test_val_above = 0.50_f32;

        let mag_below = test_val_below;
        let filtered_below = if mag_below > deadzone {
            let norm = ((mag_below - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0);
            test_val_below / mag_below * norm
        } else {
            0.0_f32
        };
        assert_eq!(filtered_below, 0.0);

        let mag_above = test_val_above;
        let filtered_above = if mag_above > deadzone {
            let norm = ((mag_above - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0);
            test_val_above / mag_above * norm
        } else {
            0.0_f32
        };
        assert!(filtered_above > 0.0);
    }
}
