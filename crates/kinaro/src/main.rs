#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub(crate) mod actions;
pub(crate) mod views;
pub(crate) mod workspace;

use crate::actions::EscAction;
use crate::views::WorkspaceView;
use gpui::*;
use gpui_component::Root;
use ki_assets::Assets;
use ki_settings::app_state::AppState;
use log::info;
use std::default::Default;
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
    if let Err(err) = ki_log::init(&config_dir) {
        eprintln!("Failed to initialize logger: {}", err);
    }

    ki_settings::init(config_dir, cx);

    info!("Initializing component lib");
    gpui_component::init(cx);
    ki_assets::init(cx);

    cx.bind_keys([KeyBinding::new("escape", EscAction, None)]);
}

fn build_window_options(cx: &mut App) -> WindowOptions {
    let (display, bounds) = AppState::read(cx, |app_state| {
        (
            app_state.display.and_then(|id| {
                cx.displays()
                    .into_iter()
                    .find(|display| display.uuid().ok() == Some(id))
                    .map(|display| display.id())
            }),
            app_state.bounds(),
        )
    });
    let mut options = WindowOptions {
        window_bounds: bounds,
        display_id: display,
        window_min_size: Some(Size::new(px(800.0), px(600.0))),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(point(px(9.0), px(9.0))),
        }),
        ..Default::default()
    };

    #[cfg(target_os = "linux")]
    {
        options.window_decorations = Some(WindowDecorations::Client)
    }
    options
}
