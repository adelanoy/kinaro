use crate::ui::editor::steps::StepDataEditor::{Delay, Rest};
use crate::ui::editor::steps::delay_editor::DelayEditor;
use crate::ui::editor::steps::rest_editor::RestEditor;
use gpui_kit::{App, AppContext, Entity};
use ki_workspace::TestStep;
use ki_workspace::test::test_step::StepData;

pub mod delay_editor;
pub mod rest_editor;

#[derive(Clone, Debug)]
pub enum StepDataEditor {
  Delay(Entity<DelayEditor>),
  Rest(Entity<RestEditor>),
}

impl StepDataEditor {
  pub fn new(value: Entity<TestStep>, cx: &mut App) -> Self {
    match value.read(cx).data() {
      StepData::RestStep(_) => Rest(cx.new(|_| RestEditor::new(value))),
      StepData::Delay(_) => Delay(cx.new(|_| DelayEditor::new(value))),
    }
  }
}