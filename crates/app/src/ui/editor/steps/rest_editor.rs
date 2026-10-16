use gpui_kit::base::v_flex;
use gpui_kit::component::label::Label;
use gpui_kit::{Context, EventEmitter, IntoElement, ParentElement, Render, Styled, Window};
use ki_workspace::test::step_rest::RestData;

#[allow(unused)]
pub enum RestEditorEvent {
  ValueChanged,
}

pub struct RestEditor {
  _rest_data: RestData,
}

impl RestEditor {
  pub fn new(rest_data: RestData, _window: &mut Window, _cx: &mut Context<RestEditor>) -> Self {
    Self { _rest_data: rest_data }
  }
}

impl EventEmitter<RestEditorEvent> for RestEditor {}

impl Render for RestEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<RestEditor>) -> impl IntoElement {
    v_flex().size_full().child(Label::new("Rest").text_lg())
  }
}
