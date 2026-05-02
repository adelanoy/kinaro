use crate::error::{SettingsError};
use gpui::{point, px, size, App, Bounds,Window, WindowBounds};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

const WINDOW_FILE: &str = "window.json";

#[derive(Serialize, Deserialize, Debug)]
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
struct WindowSettingsContent {
    display: Uuid,
    bounds: WindowBoundsContent,
}

#[derive(Default, Debug)]
pub struct WindowSettings {
    file_path: PathBuf,
    settings_content: Option<WindowSettingsContent>,
}

impl WindowSettings {
    pub(crate) fn init(config_dir: &PathBuf) -> Self {
        let file_path = config_dir.join(WINDOW_FILE);
        let settings = Self::load_or_default(&file_path);

        Self {
            file_path,
            settings_content: settings,
        }
    }

    pub fn window_bounds(&self) -> Option<WindowBounds> {
        if let Some(settings) = &self.settings_content {
            let bounds = WindowBounds::from(&settings.bounds);
            Some(bounds)
        } else {
            None
        }
    }

    pub fn display(&self) -> Option<Uuid> {
        if let Some(settings) = &self.settings_content {
            Some(settings.display)
        } else {
            None
        }
    }

    fn load_or_default(file_path: &PathBuf) -> Option<WindowSettingsContent> {
        if file_path.exists() {
            debug!(
                "Opening window settings file: {}",
                file_path.to_string_lossy()
            );
            let settings = fs::read(&file_path)
                .map_err(|e| SettingsError::Io(e))
                .and_then(|file| {
                    serde_json::from_slice::<WindowSettingsContent>(&file)
                        .map_err(|err| SettingsError::ReadJson(err.to_string()))
                });
            if let Err(err) = &settings {
                error!(
                    "Failed to load window settings: {}. Reverting to default",
                    err
                );
                Default::default()
            }
            Some(settings.unwrap())
        } else {
            None
        }
    }

    pub fn save(&mut self, window: &mut Window, cx: &mut App) {
        debug!("Saving window settings file to {}", self.file_path.to_string_lossy());

        let Some(display) = window.display(cx) else {
            return;
        };
        let Ok(display_uuid) = display.uuid() else {
            return;
        };
        let window_bounds = window.inner_window_bounds();
        let settings_content = Some(WindowSettingsContent {
            display: display_uuid,
            bounds: WindowBoundsContent::from(window_bounds),
        });
        let file_path = self.file_path.clone();

        if let Err(err) = fs::File::create(file_path)
            .map_err(|err| SettingsError::from(err))
            .and_then(|file| {
                serde_json::to_writer_pretty(file, &settings_content)
                    .map_err(|err| SettingsError::WriteJson(err.to_string()))
            })
        {
            error!("Error saving window settings file: {}", err);
        }
    }
}
