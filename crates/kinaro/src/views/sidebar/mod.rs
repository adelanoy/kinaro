mod project_configuration;
mod project_configurator_panel;

use crate::views::sidebar::project_configurator_panel::ProjectConfigurator;
use crate::workspace::{Workspace, WorkspaceEvent};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::v_flex;

pub struct ProjectSidebar {
    project_configurator: Option<Entity<ProjectConfigurator>>,
    _workspace_sub: Subscription,
}

impl ProjectSidebar {
    pub fn new(workspace: Entity<Workspace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _workspace_sub = cx.subscribe_in(
            &workspace,
            window,
            |this, workspace, event, window, cx| match event {
                WorkspaceEvent::ActiveProjectChanged(_) => match workspace.read(cx).active_project() {
                    None => this.project_configurator = None,
                    Some(project) => {
                        this.project_configurator =
                            Some(cx.new(|cx| ProjectConfigurator::new(project, window, cx)))
                    }
                },
                _ => {}
            },
        );
        let project_configurator = match workspace.read(cx).active_project() {
            None => None,
            Some(project) => Some(cx.new(|cx| ProjectConfigurator::new(project, window, cx))),
        };

        Self {
            project_configurator,
            _workspace_sub,
        }
    }
}

impl Render for ProjectSidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .pb_1()
            .gap_y_2()
            .when_some(
                self.project_configurator.clone(),
                |div, project_configurator| div.child(project_configurator),
            )
            .when_none(&self.project_configurator, |this| {
                this.child(
                    v_flex()
                        .h_full()
                        .items_center()
                        .justify_center()
                        .child("Select or create a project"),
                )
            })
    }
}
