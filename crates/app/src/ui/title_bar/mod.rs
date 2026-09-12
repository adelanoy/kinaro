pub mod project_selector;

use crate::ui::title_bar::project_selector::ProjectSelector;
use ki_workspace::Workspace;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable, Theme, ThemeMode, TitleBar};
use gpui_kit::*;
use ki_settings::app_state::AppState;

pub struct AppTitleBar {
  project_selector: Entity<ProjectSelector>,
}

impl AppTitleBar {
  pub fn new(workspace: Entity<Workspace>, cx: &mut Context<Self>) -> Self {
    Self {
      project_selector: cx.new(|cx| ProjectSelector::new(workspace, cx)),
    }
  }
}

impl Render for AppTitleBar {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let is_dark_mode = cx.theme().mode == ThemeMode::Dark;

    TitleBar::new()
      .h_10()
      .justify_between()
      .child(
        div()
          .flex()
          .items_center()
          .child(self.project_selector.clone()),
      )
      .child(
        div()
          .flex()
          .items_center()
          .px_2()
          .gap_2()
          .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .child(Icon::new(IconName::Moon))
          .child(
            Switch::new("theme-mode-selector")
              .checked(is_dark_mode)
              .small()
              .on_click(|checked, _, cx| {
                let mode = if *checked {
                  ThemeMode::Dark
                } else {
                  ThemeMode::Light
                };
                Theme::change(mode, None, cx);
                cx.refresh_windows();
                AppState::update(cx, |app_state, _cx| {
                  app_state.theme = mode;
                  true
                })
              }),
          ),
      )
  }
}
