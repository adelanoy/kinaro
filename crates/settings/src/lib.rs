use crate::window_settings::WindowSettings;
use gpui::{App, Global};
use std::path::PathBuf;

pub mod error;
pub mod window_settings;

pub fn init(config_dir: &PathBuf, cx: &mut App) {
    let settings = GlobalSettings::init(config_dir);
    cx.set_global(settings);
}

#[derive(Debug)]
pub struct GlobalSettings {
    pub window_settings: WindowSettings,
}

impl GlobalSettings {
    fn init(config_dir: &PathBuf) -> Self {
        let window_settings = WindowSettings::init(config_dir);

        Self {
            window_settings,
        }
    }
    
}

impl Global for GlobalSettings {}
