use crate::views::sidebar::project_configuration::profile_editor_tab::ProfileEditor;
use crate::views::sidebar::project_configuration::project_tree_tab::ProjectTreeTab;
use crate::views::sidebar::project_configuration::var_editor_tab::VariableEditor;
use crate::workspace::WorkspaceProject;
use gpui::*;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{Icon, Sizable, v_flex};

mod profile_editor_tab;
mod project_tree_tab;
mod var_editor_tab;

pub struct ProjectConfigurationTabs {
    selected_tab_index: usize,
    config_tabs: Vec<Entity<ProjectConfigurationTabContainer>>,
}

impl ProjectConfigurationTabs {
    pub(super) fn new(
        project: Entity<WorkspaceProject>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            selected_tab_index: 0,
            config_tabs: Self::build_tab_containers(project, window, cx),
        }
    }

    fn build_tab_containers(
        active_project: Entity<WorkspaceProject>,
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<Entity<ProjectConfigurationTabContainer>> {
        vec![
            ProjectConfigurationTabContainer::container::<VariableEditor>(
                active_project.clone(),
                window,
                cx,
            ),
            ProjectConfigurationTabContainer::container::<ProfileEditor>(
                active_project.clone(),
                window,
                cx,
            ),
            ProjectConfigurationTabContainer::container::<ProjectTreeTab>(
                active_project.clone(),
                window,
                cx,
            ),
        ]
    }
}

struct ProjectConfigurationTabContainer {
    title: SharedString,
    icon: Icon,
    view: AnyView,
}

impl ProjectConfigurationTabContainer {
    fn container<T: ProjectConfigurationTab>(
        active_project: Entity<WorkspaceProject>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let title = T::name();
        let icon = T::icon();
        let view = T::new(active_project, window, cx);

        cx.new(|_| Self {
            title: SharedString::from(title),
            icon: icon.into(),
            view: view.into(),
        })
    }
}

trait ProjectConfigurationTab: Render {
    fn name() -> &'static str;

    fn icon() -> impl Into<Icon>;

    fn new(
        active_project: Entity<WorkspaceProject>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<impl Render>;
}

impl Render for ProjectConfigurationTabs {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_tab = self
            .config_tabs
            .get(self.selected_tab_index)
            .unwrap()
            .read(cx);
        v_flex()
            .size_full()
            .child(
                TabBar::new("project_conf_tabs")
                    .w_full()
                    .small()
                    .selected_index(self.selected_tab_index)
                    .on_click(cx.listener(|this, index, _, _| this.selected_tab_index = *index))
                    .segmented()
                    .children(self.config_tabs.iter().map(|tab| {
                        let tab = tab.read(cx);
                        Tab::new().prefix(tab.icon.clone()).label(tab.title.clone())
                    })),
            )
            .child(current_tab.view.clone())
    }
}
