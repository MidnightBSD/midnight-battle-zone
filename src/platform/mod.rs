pub mod gamepad;
pub mod x11;

pub use gamepad::{GamepadManager, GamepadState};
pub use x11::{HeadlessWindow, InputState, KeyCode, X11Window};

pub enum AppWindow {
    X11(X11Window),
    Headless(HeadlessWindow),
}

impl AppWindow {
    pub fn open(title: &str, width: u32, height: u32) -> Self {
        if std::env::var("DISPLAY").is_ok() {
            match X11Window::new(title, width, height) {
                Ok(win) => {
                    log::info!(
                        "Created X11 window ({}x{}) scaled to display resolution",
                        win.input.window_width,
                        win.input.window_height
                    );
                    return AppWindow::X11(win);
                }
                Err(err) => {
                    log::warn!("Failed to initialize X11 window: {}. Falling back to HeadlessWindow.", err);
                }
            }
        } else {
            log::info!("DISPLAY environment variable not set. Running in headless mode.");
        }

        AppWindow::Headless(HeadlessWindow::new(width.max(1280), height.max(720)))
    }

    pub fn headless(width: u32, height: u32) -> Self {
        AppWindow::Headless(HeadlessWindow::new(width, height))
    }

    pub fn poll_events(&mut self) -> &InputState {
        match self {
            AppWindow::X11(win) => win.poll_events(),
            AppWindow::Headless(win) => win.poll_events(),
        }
    }

    pub fn input(&self) -> &InputState {
        match self {
            AppWindow::X11(win) => &win.input,
            AppWindow::Headless(win) => &win.input,
        }
    }

    pub fn input_mut(&mut self) -> &mut InputState {
        match self {
            AppWindow::X11(win) => &mut win.input,
            AppWindow::Headless(win) => &mut win.input,
        }
    }

    pub fn x11_handle(&self) -> Option<(*mut libc::c_void, libc::c_ulong)> {
        match self {
            AppWindow::X11(win) => Some((win.display, win.window)),
            AppWindow::Headless(_) => None,
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        let input = self.input();
        (input.window_width, input.window_height)
    }

    pub fn should_close(&self) -> bool {
        self.input().should_close
    }
}
