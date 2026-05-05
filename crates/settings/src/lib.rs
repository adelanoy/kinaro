use crate::app_state::AppState;
use gpui::{App, Global, Task};
use std::path::PathBuf;
use std::time::Duration;

pub mod app_state;
pub mod error;

pub fn init(config_dir: PathBuf, cx: &mut App) {
    let settings = GlobalSettings::init(config_dir);
    cx.set_global(settings);
}

#[derive(Debug)]
pub struct GlobalSettings {
    pub config_dir: PathBuf,
    app_state: AppState,
    app_state_save_task_queued: Option<Task<()>>,
}

impl GlobalSettings {
    fn init(config_dir: PathBuf) -> Self {
        let app_state = AppState::init(&config_dir);

        Self {
            app_state_save_task_queued: None,
            config_dir,
            app_state,
        }
    }

    /// Update the app state and save the file.
    /// Throttle the saving of the file every 500ms
    pub fn update_app_state(&mut self, cx: &mut App, update: impl FnOnce(&mut AppState, &mut App)) {
        update(&mut self.app_state, cx);
        if self.app_state_save_task_queued.is_some() {
            return;
        }

        self.app_state_save_task_queued = Some(cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            cx.update_global(|settings: &mut GlobalSettings, _cx| {
                settings.app_state.save(&settings.config_dir);
                settings.app_state_save_task_queued.take();
            })
        }));
    }
}

impl Global for GlobalSettings {}
