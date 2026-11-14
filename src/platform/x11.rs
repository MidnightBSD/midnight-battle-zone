use glam::Vec2;
use std::collections::HashSet;
use std::ffi::CString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Space,
    Return,
    Escape,
    Backspace,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Shift,
    Control,
    Alt,
    Unknown(u32),
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub mouse_pos: Vec2,
    pub left_down: bool,
    pub left_pressed: bool,
    pub left_released: bool,
    pub right_pressed: bool,
    pub drag_start: Option<Vec2>,
    pub scroll_delta: f32,
    pub keys_down: HashSet<KeyCode>,
    pub keys_pressed: HashSet<KeyCode>,
    pub window_width: u32,
    pub window_height: u32,
    pub should_close: bool,
}

impl InputState {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            mouse_pos: Vec2::ZERO,
            left_down: false,
            left_pressed: false,
            left_released: false,
            right_pressed: false,
            drag_start: None,
            scroll_delta: 0.0,
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            window_width: width,
            window_height: height,
            should_close: false,
        }
    }

    pub fn begin_frame(&mut self) {
        self.left_pressed = false;
        self.left_released = false;
        self.right_pressed = false;
        self.scroll_delta = 0.0;
        self.keys_pressed.clear();
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }
}

// X11 C Types and Constants
pub const KEY_PRESS: libc::c_int = 2;
pub const KEY_RELEASE: libc::c_int = 3;
pub const BUTTON_PRESS: libc::c_int = 4;
pub const BUTTON_RELEASE: libc::c_int = 5;
pub const MOTION_NOTIFY: libc::c_int = 6;
pub const DESTROY_NOTIFY: libc::c_int = 17;
pub const CONFIGURE_NOTIFY: libc::c_int = 22;
pub const CLIENT_MESSAGE: libc::c_int = 33;

pub const KEY_PRESS_MASK: libc::c_long = 1 << 0;
pub const KEY_RELEASE_MASK: libc::c_long = 1 << 1;
pub const BUTTON_PRESS_MASK: libc::c_long = 1 << 2;
pub const BUTTON_RELEASE_MASK: libc::c_long = 1 << 3;
pub const POINTER_MOTION_MASK: libc::c_long = 1 << 6;
pub const EXPOSURE_MASK: libc::c_long = 1 << 15;
pub const STRUCTURE_NOTIFY_MASK: libc::c_long = 1 << 17;

pub const XK_BACKSPACE: libc::c_ulong = 0xff08;
pub const XK_TAB: libc::c_ulong = 0xff09;
pub const XK_RETURN: libc::c_ulong = 0xff0d;
pub const XK_ESCAPE: libc::c_ulong = 0xff1b;
pub const XK_SPACE: libc::c_ulong = 0x0020;
pub const XK_LEFT: libc::c_ulong = 0xff51;
pub const XK_UP: libc::c_ulong = 0xff52;
pub const XK_RIGHT: libc::c_ulong = 0xff53;
pub const XK_DOWN: libc::c_ulong = 0xff54;
pub const XK_SHIFT_L: libc::c_ulong = 0xffe1;
pub const XK_SHIFT_R: libc::c_ulong = 0xffe2;
pub const XK_CONTROL_L: libc::c_ulong = 0xffe3;
pub const XK_CONTROL_R: libc::c_ulong = 0xffe4;
pub const XK_ALT_L: libc::c_ulong = 0xffe9;
pub const XK_ALT_R: libc::c_ulong = 0xffea;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XKeyEvent {
    pub type_: libc::c_int,
    pub serial: libc::c_ulong,
    pub send_event: libc::c_int,
    pub display: *mut libc::c_void,
    pub window: libc::c_ulong,
    pub root: libc::c_ulong,
    pub subwindow: libc::c_ulong,
    pub time: libc::c_ulong,
    pub x: libc::c_int,
    pub y: libc::c_int,
    pub x_root: libc::c_int,
    pub y_root: libc::c_int,
    pub state: libc::c_uint,
    pub keycode: libc::c_uint,
    pub same_screen: libc::c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XButtonEvent {
    pub type_: libc::c_int,
    pub serial: libc::c_ulong,
    pub send_event: libc::c_int,
    pub display: *mut libc::c_void,
    pub window: libc::c_ulong,
    pub root: libc::c_ulong,
    pub subwindow: libc::c_ulong,
    pub time: libc::c_ulong,
    pub x: libc::c_int,
    pub y: libc::c_int,
    pub x_root: libc::c_int,
    pub y_root: libc::c_int,
    pub state: libc::c_uint,
    pub button: libc::c_uint,
    pub same_screen: libc::c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XMotionEvent {
    pub type_: libc::c_int,
    pub serial: libc::c_ulong,
    pub send_event: libc::c_int,
    pub display: *mut libc::c_void,
    pub window: libc::c_ulong,
    pub root: libc::c_ulong,
    pub subwindow: libc::c_ulong,
    pub time: libc::c_ulong,
    pub x: libc::c_int,
    pub y: libc::c_int,
    pub x_root: libc::c_int,
    pub y_root: libc::c_int,
    pub state: libc::c_uint,
    pub is_hint: libc::c_char,
    pub same_screen: libc::c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XConfigureEvent {
    pub type_: libc::c_int,
    pub serial: libc::c_ulong,
    pub send_event: libc::c_int,
    pub display: *mut libc::c_void,
    pub event: libc::c_ulong,
    pub window: libc::c_ulong,
    pub x: libc::c_int,
    pub y: libc::c_int,
    pub width: libc::c_int,
    pub height: libc::c_int,
    pub border_width: libc::c_int,
    pub above: libc::c_ulong,
    pub override_redirect: libc::c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union ClientMessageData {
    pub b: [libc::c_char; 20],
    pub s: [libc::c_short; 10],
    pub l: [libc::c_long; 5],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XClientMessageEvent {
    pub type_: libc::c_int,
    pub serial: libc::c_ulong,
    pub send_event: libc::c_int,
    pub display: *mut libc::c_void,
    pub window: libc::c_ulong,
    pub message_type: libc::c_ulong,
    pub format: libc::c_int,
    pub data: ClientMessageData,
}

#[repr(C)]
pub union XEvent {
    pub type_: libc::c_int,
    pub xkey: XKeyEvent,
    pub xbutton: XButtonEvent,
    pub xmotion: XMotionEvent,
    pub xconfigure: XConfigureEvent,
    pub xclient: XClientMessageEvent,
    pub pad: [libc::c_long; 24],
}

#[cfg(unix)]
#[link(name = "X11")]
extern "C" {
    pub fn XOpenDisplay(display_name: *const libc::c_char) -> *mut libc::c_void;
    pub fn XDefaultScreen(display: *mut libc::c_void) -> libc::c_int;
    pub fn XDisplayWidth(display: *mut libc::c_void, screen_number: libc::c_int) -> libc::c_int;
    pub fn XDisplayHeight(display: *mut libc::c_void, screen_number: libc::c_int) -> libc::c_int;
    pub fn XRootWindow(display: *mut libc::c_void, screen_number: libc::c_int) -> libc::c_ulong;
    pub fn XCreateSimpleWindow(
        display: *mut libc::c_void,
        parent: libc::c_ulong,
        x: libc::c_int,
        y: libc::c_int,
        width: libc::c_uint,
        height: libc::c_uint,
        border_width: libc::c_uint,
        border: libc::c_ulong,
        background: libc::c_ulong,
    ) -> libc::c_ulong;
    pub fn XSelectInput(
        display: *mut libc::c_void,
        w: libc::c_ulong,
        event_mask: libc::c_long,
    ) -> libc::c_int;
    pub fn XStoreName(
        display: *mut libc::c_void,
        w: libc::c_ulong,
        window_name: *const libc::c_char,
    ) -> libc::c_int;
    pub fn XMapWindow(display: *mut libc::c_void, w: libc::c_ulong) -> libc::c_int;
    pub fn XPending(display: *mut libc::c_void) -> libc::c_int;
    pub fn XNextEvent(display: *mut libc::c_void, event_return: *mut XEvent) -> libc::c_int;
    pub fn XLookupKeysym(key_event: *mut XKeyEvent, index: libc::c_int) -> libc::c_ulong;
    pub fn XInternAtom(
        display: *mut libc::c_void,
        atom_name: *const libc::c_char,
        only_if_exists: libc::c_int,
    ) -> libc::c_ulong;
    pub fn XSetWMProtocols(
        display: *mut libc::c_void,
        w: libc::c_ulong,
        protocols: *mut libc::c_ulong,
        count: libc::c_int,
    ) -> libc::c_int;
    pub fn XGetGeometry(
        display: *mut libc::c_void,
        d: libc::c_ulong,
        root_return: *mut libc::c_ulong,
        x_return: *mut libc::c_int,
        y_return: *mut libc::c_int,
        width_return: *mut libc::c_uint,
        height_return: *mut libc::c_uint,
        border_width_return: *mut libc::c_uint,
        depth_return: *mut libc::c_uint,
    ) -> libc::c_int;
    pub fn XCloseDisplay(display: *mut libc::c_void) -> libc::c_int;
}

pub fn x11_keysym_to_keycode(sym: libc::c_ulong) -> KeyCode {
    match sym {
        XK_BACKSPACE => KeyCode::Backspace,
        XK_TAB => KeyCode::Tab,
        XK_RETURN => KeyCode::Return,
        XK_ESCAPE => KeyCode::Escape,
        XK_SPACE => KeyCode::Space,
        XK_LEFT => KeyCode::Left,
        XK_UP => KeyCode::Up,
        XK_RIGHT => KeyCode::Right,
        XK_DOWN => KeyCode::Down,
        XK_SHIFT_L | XK_SHIFT_R => KeyCode::Shift,
        XK_CONTROL_L | XK_CONTROL_R => KeyCode::Control,
        XK_ALT_L | XK_ALT_R => KeyCode::Alt,
        0x0030 => KeyCode::Num0,
        0x0031 => KeyCode::Num1,
        0x0032 => KeyCode::Num2,
        0x0033 => KeyCode::Num3,
        0x0034 => KeyCode::Num4,
        0x0035 => KeyCode::Num5,
        0x0036 => KeyCode::Num6,
        0x0037 => KeyCode::Num7,
        0x0038 => KeyCode::Num8,
        0x0039 => KeyCode::Num9,
        0x0041 | 0x0061 => KeyCode::KeyA,
        0x0042 | 0x0062 => KeyCode::KeyB,
        0x0043 | 0x0063 => KeyCode::KeyC,
        0x0044 | 0x0064 => KeyCode::KeyD,
        0x0045 | 0x0065 => KeyCode::KeyE,
        0x0046 | 0x0066 => KeyCode::KeyF,
        0x0047 | 0x0067 => KeyCode::KeyG,
        0x0048 | 0x0068 => KeyCode::KeyH,
        0x0049 | 0x0069 => KeyCode::KeyI,
        0x004A | 0x006A => KeyCode::KeyJ,
        0x004B | 0x006B => KeyCode::KeyK,
        0x004C | 0x006C => KeyCode::KeyL,
        0x004D | 0x006D => KeyCode::KeyM,
        0x004E | 0x006E => KeyCode::KeyN,
        0x004F | 0x006F => KeyCode::KeyO,
        0x0050 | 0x0070 => KeyCode::KeyP,
        0x0051 | 0x0071 => KeyCode::KeyQ,
        0x0052 | 0x0072 => KeyCode::KeyR,
        0x0053 | 0x0073 => KeyCode::KeyS,
        0x0054 | 0x0074 => KeyCode::KeyT,
        0x0055 | 0x0075 => KeyCode::KeyU,
        0x0056 | 0x0076 => KeyCode::KeyV,
        0x0057 | 0x0077 => KeyCode::KeyW,
        0x0058 | 0x0078 => KeyCode::KeyX,
        0x0059 | 0x0079 => KeyCode::KeyY,
        0x005A | 0x007A => KeyCode::KeyZ,
        other => KeyCode::Unknown(other as u32),
    }
}

pub struct X11Window {
    pub display: *mut libc::c_void,
    pub window: libc::c_ulong,
    pub wm_delete_window: libc::c_ulong,
    pub input: InputState,
}

impl X11Window {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, String> {
        unsafe {
            let display = XOpenDisplay(std::ptr::null());
            if display.is_null() {
                return Err("Failed to open X11 display".to_string());
            }

            let screen = XDefaultScreen(display);
            let root = XRootWindow(display, screen);
            let screen_w = (XDisplayWidth(display, screen).max(800)) as u32;
            let screen_h = (XDisplayHeight(display, screen).max(600)) as u32;

            // Scale window automatically with display resolution (85% of screen size, or full screen if screen <= 1366x768)
            let (win_w, win_h) = if width == 0 || height == 0 {
                if screen_w <= 1366 && screen_h <= 768 {
                    (screen_w, screen_h)
                } else {
                    let scaled_w = ((screen_w as f32 * 0.85) as u32).max(1280).min(screen_w);
                    let scaled_h = ((screen_h as f32 * 0.85) as u32).max(720).min(screen_h);
                    (scaled_w, scaled_h)
                }
            } else {
                (width.min(screen_w), height.min(screen_h))
            };

            let pos_x = ((screen_w.saturating_sub(win_w)) / 2) as libc::c_int;
            let pos_y = ((screen_h.saturating_sub(win_h)) / 2) as libc::c_int;

            let window = XCreateSimpleWindow(
                display,
                root,
                pos_x,
                pos_y,
                win_w,
                win_h,
                0,
                0,
                0,
            );

            let c_title = CString::new(title).map_err(|e| e.to_string())?;
            XStoreName(display, window, c_title.as_ptr());

            let mask = KEY_PRESS_MASK
                | KEY_RELEASE_MASK
                | BUTTON_PRESS_MASK
                | BUTTON_RELEASE_MASK
                | POINTER_MOTION_MASK
                | STRUCTURE_NOTIFY_MASK
                | EXPOSURE_MASK;
            XSelectInput(display, window, mask);

            let wm_protocols = CString::new("WM_PROTOCOLS").unwrap();
            let wm_delete = CString::new("WM_DELETE_WINDOW").unwrap();
            let proto_atom = XInternAtom(display, wm_protocols.as_ptr(), 0);
            let delete_atom = XInternAtom(display, wm_delete.as_ptr(), 0);

            if proto_atom != 0 && delete_atom != 0 {
                let mut atoms = [delete_atom];
                XSetWMProtocols(display, window, atoms.as_mut_ptr(), 1);
            }

            XMapWindow(display, window);

            Ok(Self {
                display,
                window,
                wm_delete_window: delete_atom,
                input: InputState::new(win_w, win_h),
            })
        }
    }

    pub fn poll_events(&mut self) -> &InputState {
        self.input.begin_frame();

        unsafe {
            while XPending(self.display) > 0 {
                let mut event: XEvent = std::mem::zeroed();
                XNextEvent(self.display, &mut event);

                match event.type_ {
                    KEY_PRESS => {
                        let mut xkey = event.xkey;
                        let sym = XLookupKeysym(&mut xkey, 0);
                        let key = x11_keysym_to_keycode(sym);
                        if self.input.keys_down.insert(key) {
                            self.input.keys_pressed.insert(key);
                        }
                    }
                    KEY_RELEASE => {
                        let mut xkey = event.xkey;
                        let sym = XLookupKeysym(&mut xkey, 0);
                        let key = x11_keysym_to_keycode(sym);
                        self.input.keys_down.remove(&key);
                    }
                    BUTTON_PRESS => {
                        let xbutton = event.xbutton;
                        self.input.mouse_pos = Vec2::new(xbutton.x as f32, xbutton.y as f32);
                        match xbutton.button {
                            1 => {
                                self.input.left_down = true;
                                self.input.left_pressed = true;
                                self.input.drag_start = Some(self.input.mouse_pos);
                            }
                            3 => {
                                self.input.right_pressed = true;
                            }
                            4 => {
                                self.input.scroll_delta += 1.0;
                            }
                            5 => {
                                self.input.scroll_delta -= 1.0;
                            }
                            _ => {}
                        }
                    }
                    BUTTON_RELEASE => {
                        let xbutton = event.xbutton;
                        self.input.mouse_pos = Vec2::new(xbutton.x as f32, xbutton.y as f32);
                        if xbutton.button == 1 {
                            self.input.left_down = false;
                            self.input.left_released = true;
                            self.input.drag_start = None;
                        }
                    }
                    MOTION_NOTIFY => {
                        let xmotion = event.xmotion;
                        self.input.mouse_pos = Vec2::new(xmotion.x as f32, xmotion.y as f32);
                    }
                    CONFIGURE_NOTIFY => {
                        let xconf = event.xconfigure;
                        if xconf.width > 0 && xconf.height > 0 {
                            self.input.window_width = xconf.width as u32;
                            self.input.window_height = xconf.height as u32;
                        }
                    }
                    CLIENT_MESSAGE => {
                        let xclient = event.xclient;
                        if xclient.data.l[0] as libc::c_ulong == self.wm_delete_window {
                            self.input.should_close = true;
                        }
                    }
                    DESTROY_NOTIFY => {
                        self.input.should_close = true;
                    }
                    _ => {}
                }
            }
        }

        &self.input
    }
}

impl Drop for X11Window {
    fn drop(&mut self) {
        unsafe {
            if !self.display.is_null() {
                XCloseDisplay(self.display);
                self.display = std::ptr::null_mut();
            }
        }
    }
}

pub struct HeadlessWindow {
    pub input: InputState,
}

impl HeadlessWindow {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            input: InputState::new(width, height),
        }
    }

    pub fn poll_events(&mut self) -> &InputState {
        self.input.begin_frame();
        &self.input
    }

    pub fn simulate_mouse_move(&mut self, pos: Vec2) {
        self.input.mouse_pos = pos;
    }

    pub fn simulate_left_click(&mut self, pos: Vec2) {
        self.input.mouse_pos = pos;
        self.input.left_pressed = true;
        self.input.left_down = true;
    }

    pub fn simulate_right_click(&mut self, pos: Vec2) {
        self.input.mouse_pos = pos;
        self.input.right_pressed = true;
    }

    pub fn simulate_key_press(&mut self, key: KeyCode) {
        self.input.keys_down.insert(key);
        self.input.keys_pressed.insert(key);
    }
}
