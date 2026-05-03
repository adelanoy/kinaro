#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod views;

use crate::views::WorkspaceView;
use assets::Assets;
use assets::theme::ThemeAsset;
use gpui::*;
use gpui_component::{Root, Theme, ThemeRegistry};
use log::{info, warn};
use settings::GlobalSettings;
use std::path::PathBuf;
use settings::app_state::AppState;

fn main() {
    let config_dir = dirs::config_local_dir().unwrap().join("Kinaro_gpui");
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            init_app(config_dir, cx);

            let options = build_window_options(cx);
            cx.spawn(async move |cx| {
                cx.open_window(options, |window, cx| {
                    let view = cx.new(|cx| WorkspaceView::new(window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                })
                    .expect("Failed to open window");
            })
                .detach();
        });
}

fn init_app(config_dir: PathBuf, cx: &mut App) {
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).expect("config_dir created");
    }
    if let Err(err) = klog::init(&config_dir) {
        eprintln!("Failed to initialize logger: {}", err);
    }

    settings::init(config_dir, cx);

    info!("Initializing component lib");
    gpui_component::init(cx);
    init_themes(cx);
}

fn init_themes(cx: &mut App) {
    let theme = AppState::read(cx,|app_state| { app_state.theme.clone()});
    if let Some(theme_asset) = ThemeAsset::Ayu.load_theme_asset(cx).ok().flatten() {
        if let Err(err) = ThemeRegistry::global_mut(cx)
            .load_themes_from_str(str::from_utf8(&*theme_asset).unwrap())
        {
            warn!("Error while loading themes: {}", err);
            return;
        }
        if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme).cloned() {
            Theme::global_mut(cx).apply_config(&theme);
        }
    }
}

fn build_window_options(cx: &mut App) -> WindowOptions {
    let (display, bounds) = cx.read_global(|settings: &GlobalSettings, _cx| {
        (
            settings.app_state.display.and_then(|id| {
                cx.displays()
                    .into_iter()
                    .find(|display| display.uuid().ok() == Some(id))
                    .map(|display| display.id())
            }),
            settings.app_state.bounds(),
        )
    });
    let mut options = WindowOptions::default();
    options.window_bounds = bounds;
    options.display_id = display;
    options
}
