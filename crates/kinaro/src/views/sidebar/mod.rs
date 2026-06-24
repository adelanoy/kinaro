mod project_configurator_panel;
mod project_selector;
mod project_configuration;

use crate::views::sidebar::project_configurator_panel::ProjectConfigurator;
use crate::views::sidebar::project_selector::ProjectSelector;
use crate::workspace::Workspace;
use gpui::{AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window};
use gpui_component::separator::Separator;
use gpui_component::v_flex;
use crate::views::sidebar::project_configuration::ProjectConfigurationTabs;

pub struct ProjectSidebar {
    project_selector: Entity<ProjectSelector>,
    project_configurator: Entity<ProjectConfigurator>,
    project_config_tabs: Entity<ProjectConfigurationTabs>,
}

impl ProjectSidebar {
    pub fn new(workspace: Entity<Workspace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project_selector = cx.new(|cx| ProjectSelector::new(workspace.clone(), cx));
        let project_configurator =
            cx.new(|cx| ProjectConfigurator::new(workspace.clone(), window, cx));
        let project_config_tabs =
            cx.new(|cx| ProjectConfigurationTabs::new(workspace, window, cx));

        Self {
            project_selector,
            project_configurator,
            project_config_tabs
        }
    }
}

impl Render for ProjectSidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .pb_1()
            .gap_y_2()
            .child(self.project_selector.clone())
            .child(Separator::horizontal())
            .child(self.project_configurator.clone())
            .child(Separator::horizontal())
            .child(self.project_config_tabs.clone())
    }
}
