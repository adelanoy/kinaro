#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod views;

use crate::views::WorkspaceView;
use gpui::*;
use gpui_component::Root;
use kassets::Assets;
use log::info;
use settings::GlobalSettings;
use std::path::PathBuf;

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
    kassets::init(cx);
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
    options.titlebar = Some(TitlebarOptions {
        title: None,
        appears_transparent: true,
        traffic_light_position: Some(point(px(9.0), px(9.0))),
    });
    options
}
