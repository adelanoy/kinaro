use crate::actions;
use gpui_kit::component::WindowExt;
use gpui_kit::component::notification::Notification;
use gpui_kit::{App, ClipboardItem, Entity, Size, TitlebarOptions, Window, WindowDecorations, WindowOptions, point, px};
use ki_settings::app_state::AppState;
use ki_utils::{CASE, STEP, SUITE, TestPath};
use ki_workspace::test::TestsContainer;
use log::{info, warn};
use std::path::PathBuf;

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

/// Copies the node at `path` in `tests` to the clipboard, serialized as JSON, with metadata telling
/// its kind ([`SUITE`], [`CASE`] or [`STEP`]) so it can be pasted back. Nothing
/// is copied, and a warning is logged, if no node is at `path` or it can't be serialized.
pub fn copy_test(path: TestPath, tests: &Entity<TestsContainer>, cx: &mut App) {
  match path {
    TestPath::Suite(_) => {
      let Some(suite) = tests.read_with(cx, |tests, _| {
        tests.suite_from_path(&path, cx).map(|suite| suite.read(cx).get_file(cx))
      }) else {
        warn!("copy_test: no suite from {}", path);
        return;
      };
      let Ok(value) = serde_json::to_string(&suite) else {
        warn!("Error while serializing suite: {}", path);
        return;
      };
      cx.write_to_clipboard(ClipboardItem::new_string_with_metadata(value, SUITE.to_string()));
    }
    TestPath::Case(_, _) => {
      let Some(case) = tests.read_with(cx, |tests, _| {
        tests.case_from_path(&path, cx).map(|case| case.read(cx).get_file(cx))
      }) else {
        warn!("copy_test: no case from {}", path);
        return;
      };
      let Ok(value) = serde_json::to_string(&case) else {
        warn!("Error while serializing case: {}", path);
        return;
      };
      cx.write_to_clipboard(ClipboardItem::new_string_with_metadata(value, CASE.to_string()));
    }
    TestPath::SuiteStep(_, _) | TestPath::Step(_, _, _) => {
      let Some(step) = tests.read_with(cx, |tests, _| {
        tests.step_from_path(&path, cx).map(|step| step.read(cx).get_file())
      }) else {
        warn!("copy_test: no step from {}", path);
        return;
      };
      let Ok(value) = serde_json::to_string(&step) else {
        warn!("Error while serializing step: {}", path);
        return;
      };
      cx.write_to_clipboard(ClipboardItem::new_string_with_metadata(value, STEP.to_string()));
    }
  }
}
