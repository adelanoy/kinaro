mod profile_editor_tab;
mod profile_selector;
mod project_tree_tab;
mod var_editor_tab;

use crate::ui::side_bar::profile_editor_tab::ProfileEditor;
use crate::ui::side_bar::profile_selector::ProfileSelector;
use crate::ui::side_bar::project_tree_tab::ProjectTree;
use crate::ui::side_bar::var_editor_tab::VariableEditor;
use gpui_kit::base::v_flex;
use gpui_kit::component::Sizable;
use gpui_kit::component::tab::TabBar;
use gpui_kit::*;
use gpui_kit::component::separator::Separator;
use ki_app_shared::panel::{Panel, PanelContainer};
use ki_settings::app_state::AppState;
use ki_utils::SidebarPanel;
use ki_workspace::Project;

pub struct ProjectSidebar {
  profile_selector: Entity<ProfileSelector>,
  panel_container: PanelContainer<SidebarPanel>,
}

impl ProjectSidebar {
  pub(super) fn new(project: Entity<Project>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let vars = project.read(cx).variables.clone();
    Self {
      profile_selector: cx.new(|cx| ProfileSelector::new(vars, window, cx)),
      panel_container: Self::build_project_panels(project, window, cx),
    }
  }

  fn build_project_panels(active_project: Entity<Project>, window: &mut Window, cx: &mut App) -> PanelContainer<SidebarPanel> {
    PanelContainer::new(
      vec![
        Panel::build::<Project, ProjectTree>(active_project.clone(), window, cx),
        Panel::build::<Project, VariableEditor>(active_project.clone(), window, cx),
        Panel::build::<Project, ProfileEditor>(active_project.clone(), window, cx),
      ],
      AppState::read(cx, |app_state| app_state.sidebar.selected_panel),
    )
  }

  fn select_panel(&mut self, ix: usize, cx: &mut Context<Self>) {
    self.panel_container.set_selected_ix(ix);
    let panel_type = self.panel_container.selected_panel();
    AppState::update(cx, |state, _| {
      state.sidebar.selected_panel = panel_type;
      true
    });
    cx.notify();
  }
}

impl Render for ProjectSidebar {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    v_flex()
      .size_full()
      .gap_y_1()
      .pt_1()
      .child(div().px_1().child(self.profile_selector.clone()))
      .child(Separator::horizontal())
      .child(
        TabBar::new("project-config-tabs")
          .segmented()
          .small()
          .selected_index(self.panel_container.selected_ix())
          .on_click(cx.listener(|this, ix, _, cx| this.select_panel(*ix, cx)))
          .children(self.panel_container.tabs()),
      )
      .child(self.panel_container.view())
  }
}
