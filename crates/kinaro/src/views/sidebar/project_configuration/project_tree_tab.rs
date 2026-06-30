use crate::views::sidebar::project_configuration::ProjectConfigurationTab;
use crate::workspace::Project;
use gpui::{App, AppContext, Context, Entity, IntoElement, Render, Window};
use gpui_component::Icon;
use ki_assets::icon::IconAsset;

pub(super) struct ProjectTreeTab {}

impl ProjectConfigurationTab for ProjectTreeTab {
    fn name() -> &'static str {
        "Project Tree"
    }

    fn icon() -> impl Into<Icon> {
        IconAsset::Tree
    }

    fn new(
        _active_project: Entity<Project>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Entity<impl Render> {
        cx.new(|_| Self {})
    }
}

impl Render for ProjectTreeTab {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Project Tree"
    }
}
