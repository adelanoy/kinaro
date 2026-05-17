use crate::error::SettingsError;
use crate::GlobalSettings;
use gpui::{point, px, size, App, AppContext, BorrowAppContext, Bounds, Window, WindowBounds};
use gpui_component::ThemeMode;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

const WINDOW_FILE: &str = "state.json";

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
enum WindowBoundsContent {
    Windowed {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    Maximized {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    Fullscreen {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
}

impl From<WindowBounds> for WindowBoundsContent {
    fn from(value: WindowBounds) -> Self {
        match value {
            WindowBounds::Windowed(bounds) => {
                let origin = bounds.origin;
                let size = bounds.size;
                WindowBoundsContent::Windowed {
                    x: f32::from(origin.x).round() as i32,
                    y: f32::from(origin.y).round() as i32,
                    width: f32::from(size.width).round() as i32,
                    height: f32::from(size.height).round() as i32,
                }
            }
            WindowBounds::Maximized(bounds) => {
                let origin = bounds.origin;
                let size = bounds.size;
                WindowBoundsContent::Maximized {
                    x: f32::from(origin.x).round() as i32,
                    y: f32::from(origin.y).round() as i32,
                    width: f32::from(size.width).round() as i32,
                    height: f32::from(size.height).round() as i32,
                }
            }
            WindowBounds::Fullscreen(bounds) => {
                let origin = bounds.origin;
                let size = bounds.size;
                WindowBoundsContent::Fullscreen {
                    x: f32::from(origin.x).round() as i32,
                    y: f32::from(origin.y).round() as i32,
                    width: f32::from(size.width).round() as i32,
                    height: f32::from(size.height).round() as i32,
                }
            }
        }
    }
}

impl From<&WindowBoundsContent> for WindowBounds {
    fn from(value: &WindowBoundsContent) -> Self {
        match value {
            WindowBoundsContent::Windowed {
                x,
                y,
                width,
                height,
            } => WindowBounds::Windowed(Bounds {
                origin: point(px(*x as f32), px(*y as f32)),
                size: size(px(*width as f32), px(*height as f32)),
            }),
            WindowBoundsContent::Maximized {
                x,
                y,
                width,
                height,
            } => WindowBounds::Maximized(Bounds {
                origin: point(px(*x as f32), px(*y as f32)),
                size: size(px(*width as f32), px(*height as f32)),
            }),
            WindowBoundsContent::Fullscreen {
                x,
                y,
                width,
                height,
            } => WindowBounds::Fullscreen(Bounds {
                origin: point(px(*x as f32), px(*y as f32)),
                size: size(px(*width as f32), px(*height as f32)),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SidebarState {
    pub collapsed: bool,
    pub width: f32,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            collapsed: false,
            width: 250.0,
        }
    }
}

/// Contains the application state, such as:
/// * window bounds
/// * active theme
/// * sidebar state
/// * etc...
#[derive(Serialize, Deserialize, Debug)]
pub struct AppState {
    pub display: Option<Uuid>,
    bounds: Option<WindowBoundsContent>,
    pub theme: ThemeMode,
    pub sidebar: SidebarState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            display: None,
            bounds: None,
            theme: ThemeMode::Dark,
            sidebar: Default::default(),
        }
    }
}

impl AppState {
    pub(crate) fn init(config_dir: &PathBuf) -> Self {
        let file_path = config_dir.join(WINDOW_FILE);
        Self::load_or_default(&file_path)
    }

    pub fn bounds(&self) -> Option<WindowBounds> {
        if let Some(bounds) = &self.bounds {
            let bounds = WindowBounds::from(bounds);
            Some(bounds)
        } else {
            None
        }
    }

    /// Update the app state with the given closure
    ///
    /// The bool returned by the closure instructs if the state file should be saved to disk
    pub fn update(cx: &mut App, update: impl FnOnce(&mut AppState, &mut App) -> bool) {
        cx.update_global(|settings: &mut GlobalSettings, cx| settings.update_app_state(cx, update));
    }

    pub fn read<R>(cx: &App, read_func: impl FnOnce(&AppState) -> R) -> R {
        cx.read_global(|settings: &GlobalSettings, _cx| read_func(&settings.app_state))
    }

    pub fn update_bounds(&mut self, window: &mut Window, cx: &mut App) -> bool {
        let Some(display) = window.display(cx) else {
            return false;
        };
        let Ok(display_uuid) = display.uuid() else {
            return false;
        };
        let window_bounds = window.inner_window_bounds();
        let display = Some(display_uuid);
        let bounds = Some(WindowBoundsContent::from(window_bounds));
        if self.display != self.display || self.bounds != bounds {
            self.display = display;
            self.bounds = bounds;
            return true;
        }
        false
    }

    fn load_or_default(file_path: &PathBuf) -> AppState {
        if file_path.exists() {
            debug!(
                "Opening window settings file: {}",
                file_path.to_string_lossy()
            );
            let settings = fs::read(&file_path)
                .map_err(|e| SettingsError::Io(e))
                .and_then(|file| {
                    serde_json::from_slice::<AppState>(&file)
                        .map_err(|err| SettingsError::ReadJson(err.to_string()))
                });
            if let Err(err) = &settings {
                error!(
                    "Failed to load window settings: {}. Reverting to default",
                    err
                );
                Default::default()
            }
            settings.unwrap()
        } else {
            Default::default()
        }
    }

    pub(crate) fn save(&self, config_dir: &PathBuf) {
        let file_path = config_dir.join(WINDOW_FILE);
        debug!("Saving app state to {}", file_path.to_string_lossy());

        if let Err(err) = fs::File::create(file_path)
            .map_err(|err| SettingsError::from(err))
            .and_then(|file| {
                serde_json::to_writer_pretty(file, &self)
                    .map_err(|err| SettingsError::WriteJson(err.to_string()))
            })
        {
            error!("Error saving window settings file: {}", err);
        }
    }
}
