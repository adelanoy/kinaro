use gpui_kit::{App, AppContext, Entity};
use ki_workspace::test::StepDataProvider;
use ki_workspace::test::test_step::StepData;
use crate::ui::editor::steps::delay_editor::DelayEditor;
use crate::ui::editor::steps::rest_editor::RestEditor;
use crate::ui::editor::steps::StepDataEditor::{Delay, Rest};

pub mod delay_editor;
pub mod rest_editor;

#[derive(Clone, Debug)]
pub enum StepDataEditor<S: StepDataProvider> {
  Delay(Entity<DelayEditor<S>>),
  Rest(Entity<RestEditor<S>>),
}

impl<S: StepDataProvider> StepDataEditor<S> {
  pub fn new(value: Entity<S>, cx: &mut App) -> Self {
    match value.read(cx).data().unwrap() {
      StepData::RestStep(_) => Rest(cx.new(|_| RestEditor::new(value))),
      StepData::Delay(_) => Delay(cx.new(|_| DelayEditor::new(value))),
    }
  }
}