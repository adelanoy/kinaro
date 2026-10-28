use crate::step_editor::rest_editor::RestEditorPanel;
use gpui_kit::component::Icon;
use gpui_kit::{App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Window, div};
use ki_app_shared::panel::PanelDescriptor;
use ki_assets::icon::IconAsset;
use ki_workspace::test::step_rest::RestData;

pub struct RestBodyEditor {
  _rest_data: Entity<RestData>,
}

impl PanelDescriptor<RestEditorPanel, RestData> for RestBodyEditor {
  fn name() -> &'static str {
    "Body"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::Braces
  }

  fn panel_type() -> RestEditorPanel {
    RestEditorPanel::Body
  }

  fn new(entity: Entity<RestData>, _window: &mut Window, cx: &mut App) -> Entity<Self> {
    cx.new(|_| RestBodyEditor { _rest_data: entity })
  }
}

impl Render for RestBodyEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div().child("Body editor goes here")
  }
}
