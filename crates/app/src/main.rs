#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod log;
pub(crate) mod ui;

use crate::app::{build_window_options, init_app};
use crate::ui::WorkspaceView;
use gpui_kit::AppContext;
use gpui_kit::component::Root;
use ki_assets::Assets;

fn main() {
  let config_dir = dirs::config_local_dir().unwrap().join("Kinaro");
  gpui_kit::platform::application().with_assets(Assets).run(move |cx| {
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
