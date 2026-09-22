mod profile_editor_tab;
mod project_configurator_panel;
mod project_tree_tab;
mod var_editor_tab;

use crate::ui::side_bar::profile_editor_tab::ProfileEditor;
use crate::ui::side_bar::project_tree_tab::ProjectTree;
use crate::ui::side_bar::var_editor_tab::VariableEditor;
use gpui_kit::component::{ActiveTheme, Colorize, Icon};
use gpui_kit::*;
use ki_utils::shared::SidebarPanel;
use ki_workspace::Project;

pub struct ProjectSidebar {
  pub selected_panel: SidebarPanel,
  panels: Vec<Entity<ProjectPanelContainer>>,
}

impl ProjectSidebar {
  pub(super) fn new(project: Entity<Project>, selected_panel: SidebarPanel, window: &mut Window, cx: &mut Context<Self>) -> Self {
    Self {
      selected_panel,
      panels: Self::build_project_panels(project, window, cx),
    }
  }

  fn build_project_panels(
    active_project: Entity<Project>,
    window: &mut Window,
    cx: &mut App,
  ) -> Vec<Entity<ProjectPanelContainer>> {
    vec![
      ProjectPanelContainer::container::<ProjectTree>(active_project.clone(), window, cx),
      ProjectPanelContainer::container::<VariableEditor>(active_project.clone(), window, cx),
      ProjectPanelContainer::container::<ProfileEditor>(active_project.clone(), window, cx),
    ]
  }

  pub fn panel_descriptors(&self, cx: &App) -> Vec<ProjectPanelDescriptor> {
    self.panels.iter().map(|panel| panel.read(cx).descriptor.clone()).collect()
  }
}

#[derive(Clone)]
pub struct ProjectPanelDescriptor {
  pub title: SharedString,
  pub icon: Icon,
  pub panel_type: SidebarPanel,
}

struct ProjectPanelContainer {
  descriptor: ProjectPanelDescriptor,
  view: AnyView,
}

impl ProjectPanelContainer {
  fn container<T: ProjectConfigurationPanel>(active_project: Entity<Project>, window: &mut Window, cx: &mut App) -> Entity<Self> {
    let title = T::name();
    let icon = T::icon();
    let panel_type = T::panel_type();
    let view = T::new(active_project, window, cx);

    cx.new(|_| Self {
      descriptor: ProjectPanelDescriptor {
        title: SharedString::from(title),
        icon: icon.into(),
        panel_type,
      },
      view: view.into(),
    })
  }
}

trait ProjectConfigurationPanel: Render {
  fn name() -> &'static str;

  fn icon() -> impl Into<Icon>;

  fn panel_type() -> SidebarPanel;

  fn new(active_project: Entity<Project>, window: &mut Window, cx: &mut App) -> Entity<impl Render>;
}

impl Render for ProjectSidebar {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let panel = self
      .panels
      .iter()
      .find(|panel| panel.read(cx).descriptor.panel_type == self.selected_panel)
      .unwrap();
    div().size_full().p_2().child(
      div()
        .size_full()
        .rounded_xl()
        .bg(cx.theme().background.darken(0.3))
        .child(panel.read(cx).view.clone()),
    )
  }
}
