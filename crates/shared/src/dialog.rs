use gpui_kit::base::input::InputState;
use gpui_kit::base::{markdown, v_flex, v_resizable};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::dialog::{DialogAction, DialogClose, DialogFooter};
use gpui_kit::component::input::{Input, Textarea, TextareaState};
use gpui_kit::component::{ActiveTheme, WindowExt};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{App, AppContext, Entity, IntoElement, ParentElement, SharedString, Styled, Window, div};
use std::rc::Rc;

/// Callback run with the field's value when the dialog is confirmed.
type OkFunction = dyn Fn(SharedString, &mut App);

/// Editor used by the dialog to edit the field.
#[derive(Clone)]
enum InputMode {
  /// Single-line text input.
  Input(Entity<InputState>),
  /// Multi-line markdown editor, shown with a live preview.
  Markdown(Entity<TextareaState>),
}

impl InputMode {
  /// Gives the keyboard focus to the editor.
  fn focus(&self, window: &mut Window, cx: &mut App) {
    match self {
      InputMode::Input(state) => state.update(cx, |state, cx| state.focus(window, cx)),
      InputMode::Markdown(state) => state.update(cx, |state, cx| state.focus(window, cx)),
    }
  }
  /// Returns the current content of the editor.
  fn value(&self, cx: &App) -> SharedString {
    match self {
      InputMode::Input(state) => state.read(cx).value(),
      InputMode::Markdown(state) => state.read(cx).value(),
    }
  }
}

/// Generic dialog to modify the value of a single field
pub struct EditFieldDialog {
  initial_value: SharedString,
  title: Option<SharedString>,
  desc: Option<SharedString>,
  on_ok: Rc<OkFunction>,
}

impl EditFieldDialog {
  /// Creates a new dialog with the given initial value and what to execute when the user presses Ok
  pub fn new<F>(initial_value: SharedString, on_ok: F) -> Self
  where
    F: Fn(SharedString, &mut App) + 'static,
  {
    Self {
      initial_value,
      title: None,
      desc: None,
      on_ok: Rc::new(on_ok),
    }
  }

  /// Sets the dialog's title. Defaults to "Edit".
  pub fn title(mut self, title: impl Into<SharedString>) -> Self {
    self.title = Some(title.into());
    self
  }

  /// Sets the text shown above the editor. Defaults to "Enter the new value:".
  pub fn description(mut self, desc: impl Into<SharedString>) -> Self {
    self.desc = Some(desc.into());
    self
  }

  /// Presents a single field dialog
  pub fn show(self, window: &mut Window, cx: &mut App) {
    let input = InputMode::Input(cx.new(|cx| {
      let mut state = InputState::new(window, cx);
      state.set_value(self.initial_value.clone(), window, cx);
      state
    }));
    self.show_impl(input, window, cx);
  }

  /// Presents a dialog with a markdown editor and a real-time viewer
  pub fn show_markdown(self, window: &mut Window, cx: &mut App) {
    let input = InputMode::Markdown(cx.new(|cx| {
      let mut state = TextareaState::new(window, cx);
      state.set_value(self.initial_value.clone(), window, cx);
      state
    }));
    self.show_impl(input, window, cx);
  }

  /// Opens the dialog around `input`. On confirmation, the dialog's callback is called with the content of `input`.
  fn show_impl(self, input: InputMode, window: &mut Window, cx: &mut App) {
    let title = self.title.unwrap_or("Edit".into());
    let desc = self.desc.unwrap_or("Enter the new value:".into());
    let ok_func = self.on_ok;

    window.open_dialog(cx, move |dialog, window, cx| {
      input.clone().focus(window, cx);
      dialog
        .when(matches!(&input, InputMode::Markdown(_)), |this| {
          let size = window.viewport_size().map(|size| size * 0.8);
          this.w(size.width).h(size.height).size_full()
        })
        .title(title.clone())
        .child(v_flex().gap_3().child(desc.clone()).map(|this| {
          match &input {
            InputMode::Input(state) => this.child(Input::new(state)),
            InputMode::Markdown(state) => this.child(
              v_resizable("dialog-markdown-resizable")
                .child(
                  div()
                    .p_1()
                    .w_full()
                    .border_color(cx.theme().border)
                    .border_1()
                    .rounded_md()
                    .child(Textarea::new(state).h_full())
                    .into_any_element(),
                )
                .child(
                  div()
                    .p_1()
                    .size_full()
                    .border_color(cx.theme().border)
                    .border_1()
                    .rounded_md()
                    .child(markdown(state.read(cx).value()))
                    .into_any_element(),
                ),
            ),
          }
        }))
        .footer(
          DialogFooter::new()
            .child(DialogClose::new().child(Button::new("cancel").label("Cancel").outline()))
            .child(DialogAction::new().child(Button::new("confirm").primary().label("Confirm"))),
        )
        .on_ok({
          let input = input.clone();
          let ok_func = ok_func.clone();
          move |_, _, cx| {
            ok_func(input.value(cx), cx);
            true
          }
        })
    });
  }
}
