mod profile_editor_tab;
mod project_tree_tab;
mod var_editor_tab;

use crate::ui::side_bar::profile_editor_tab::ProfileEditor;
use crate::ui::side_bar::project_tree_tab::ProjectTree;
use crate::ui::side_bar::var_editor_tab::VariableEditor;
use gpui_kit::base::v_flex;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::*;
use ki_settings::app_state::AppState;
use ki_utils::SidebarPanel;
use ki_workspace::Project;

pub struct ProjectSidebar {
  selected_panel: SidebarPanel,
  panels: Vec<Entity<ProjectPanelContainer>>,
}

impl ProjectSidebar {
  pub(super) fn new(project: Entity<Project>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let selected_panel = AppState::read(cx, |app_state| app_state.sidebar.selected_panel);
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

  fn select_panel(&mut self, ix: usize, cx: &mut Context<Self>) {
    let panel_type = self.panels[ix].read(cx).descriptor.panel_type;
    self.selected_panel = panel_type;
    AppState::update(cx, |state, _| {
      state.sidebar.selected_panel = panel_type;
      true
    });
    cx.notify();
  }
}

#[derive(Clone)]
struct ProjectPanelDescriptor {
  title: SharedString,
  icon: Icon,
  panel_type: SidebarPanel,
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
    let (ix, panel) = self
      .panels
      .iter()
      .enumerate()
      .find(|(_, panel)| panel.read(cx).descriptor.panel_type == self.selected_panel)
      .unwrap();

    v_flex()
      .size_full()
      .gap_y_1()
      .child(
        TabBar::new("project-config-tabs")
          .segmented()
          .small()
          .selected_index(ix)
          .on_click(cx.listener(|this, ix, _, cx| this.select_panel(*ix, cx)))
          .children(self.panels.iter().map(|panel| {
            Tab::new()
              .px_2()
              .prefix(panel.read(cx).descriptor.icon.clone())
              .child(panel.read(cx).descriptor.title.clone())
          })),
      )
      .child(panel.read(cx).view.clone())
  }
}
