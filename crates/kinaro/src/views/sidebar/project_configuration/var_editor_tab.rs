use crate::views::sidebar::project_configuration::ProjectConfigurationTab;
use crate::workspace::Project;
use gpui::{App, AppContext, Context, Entity, IntoElement, Render, Window};
use gpui_component::Icon;
use ki_assets::icon::IconAsset;

pub(super) struct VariableEditor {}

impl ProjectConfigurationTab for VariableEditor {
    fn name() -> &'static str {
        "Variables"
    }

    fn icon() -> impl Into<Icon> {
        IconAsset::Variable
    }

    fn new(
        _active_project: Entity<Project>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Entity<impl Render> {
        cx.new(|_| Self {})
    }
}

impl Render for VariableEditor {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Variable Editor"
    }
}
