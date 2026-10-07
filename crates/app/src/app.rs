use std::path::PathBuf;
use gpui_kit::{px, App, Size, WindowOptions, TitlebarOptions, point, WindowDecorations, ClipboardItem, Window};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::WindowExt;
use log::info;
use ki_settings::app_state::AppState;
use ki_utils::TestPath;
use crate::actions;

pub(super) fn init_app(config_dir: PathBuf, cx: &mut App) {
  if !config_dir.exists() {
    std::fs::create_dir_all(&config_dir).expect("config_dir created");
  }
  if let Err(err) = crate::log::init(&config_dir) {
    eprintln!("Failed to initialize logger: {}", err);
  }

  ki_settings::init(config_dir, cx);

  info!("Initializing component lib");
  gpui_kit::init(cx);
  ki_assets::init(cx);
  actions::init(cx);
}

pub(super) fn build_window_options(cx: &mut App) -> WindowOptions {
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

  WindowOptions {
    window_bounds: bounds,
    display_id: display,
    window_min_size: Some(Size::new(px(800.0), px(600.0))),
    titlebar: Some(TitlebarOptions {
      title: None,
      appears_transparent: true,
      traffic_light_position: Some(point(px(9.0), px(9.0))),
    }),
    window_decorations: if cfg!(target_os = "linux") {
      Some(WindowDecorations::Client)
    } else {
      Default::default()
    },
    ..Default::default()
  }
}

pub fn copy_id(path: TestPath, window: &mut Window, cx: &mut App) {
  cx.write_to_clipboard(ClipboardItem::new_string(path.id().to_string()));
  window.push_notification(Notification::info("Id copied!"), cx);
}

pub fn copy_path(path: TestPath, window: &mut Window, cx: &mut App) {
  cx.write_to_clipboard(ClipboardItem::new_string(path.to_string()));
  window.push_notification(Notification::info("Path copied!"), cx);
}