mod project_configurator_panel;
mod project_selector;

use crate::views::sidebar::project_configurator_panel::ProjectConfigurator;
use crate::views::sidebar::project_selector::ProjectSelector;
use crate::workspace::Workspace;
use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled
    , Window,
};
use gpui_component::separator::Separator;
use gpui_component::v_flex;

pub(crate) struct ProjectSidebar {
    project_selector: Entity<ProjectSelector>,
    project_configurator: Entity<ProjectConfigurator>,
}

impl ProjectSidebar {
    pub(crate) fn new(
        workspace: Entity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let project_selector =
            cx.new(|cx| ProjectSelector::new(workspace.clone(), cx));
        let project_configurator = cx.new(|cx| ProjectConfigurator::new(workspace.clone(), window, cx));

        Self {
            project_selector,
            project_configurator,
        }
    }
}

impl Render for ProjectSidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_2()
            .gap_y_2()
            .child(self.project_selector.clone())
            .child(Separator::horizontal())
            .child(self.project_configurator.clone())
            .child(Separator::horizontal())
    }
}