use gpui_kit::component::Icon;
use gpui_kit::{div, App, Context, Entity, IntoElement, Render, Window, AppContext, ParentElement};
use ki_app_shared::panel::PanelDescriptor;
use ki_assets::icon::IconAsset;
use ki_workspace::test::step_rest::RestData;
use crate::step_editor::rest_editor::RestEditorPanel;

pub struct RestHeaderEditor {
  _rest_data: Entity<RestData>,
}

impl PanelDescriptor<RestEditorPanel, RestData> for RestHeaderEditor {
  fn name() -> &'static str {
    "Headers"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::TableProperties
  }

  fn panel_type() -> RestEditorPanel {
    RestEditorPanel::Header
  }

  fn new(entity: Entity<RestData>, _window: &mut Window, cx: &mut App) -> Entity<Self> {
    cx.new(|_| RestHeaderEditor { _rest_data: entity })
  }
}

impl Render for RestHeaderEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div().child("Header editor goes here")
  }
}
