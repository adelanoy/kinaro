#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use assets::Assets;
use gpui::*;
use gpui_component::{
    Root, StyledExt,
    button::{Button, ButtonVariants},
};
use log::info;
use settings::GlobalSettings;
use std::path::PathBuf;
use std::time::Duration;

pub struct HelloWorld {
    bounds_save_task_queued: Option<Task<()>>,
}

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

impl HelloWorld {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let view = Self {
            bounds_save_task_queued: None,
        };
        view.register_window_observers(window, cx);
        view
    }

    pub(crate) fn register_window_observers(&self, window: &mut Window, cx: &mut Context<Self>) {
        // Observe the window movement and throttle the saving of its geometry every 100ms. Heavily inspired by Zed's Workspace impl
        cx.observe_window_bounds(window, move |this, window, cx| {
            if this.bounds_save_task_queued.is_some() {
                return;
            }
            this.bounds_save_task_queued = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                this.update_in(cx, |this, window, cx| {
                    cx.update_global(|settings: &mut GlobalSettings, cx| {
                        settings.window_settings.save(window, cx)
                    });
                    this.bounds_save_task_queued.take();
                })
                .ok();
            }));
            cx.notify();
        })
        .detach();
    }
}

fn main() {
    let config_dir = dirs::config_local_dir().unwrap().join("Kinaro_gpui");
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            init_app(config_dir, cx);
            let options = build_window_options(cx);
            cx.spawn(async move |cx| {
                cx.open_window(options, |window, cx| {
                    let view = cx.new(|cx| HelloWorld::new(window, cx));
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

    settings::init(&config_dir, cx);

    info!("Initializing component lib");
    gpui_component::init(cx);
}

fn build_window_options(cx: &mut App) -> WindowOptions {
    let (display, bounds) = cx.read_global(|settings: &GlobalSettings, _cx| {
        (
            settings.window_settings.display().and_then(|id| {
                cx.displays()
                    .into_iter()
                    .find(|display| display.uuid().ok() == Some(id))
                    .map(|display| display.id())
            }),
            settings.window_settings.window_bounds(),
        )
    });
    let mut options = WindowOptions::default();
    options.window_bounds = bounds;
    options.display_id = display;
    options
}
