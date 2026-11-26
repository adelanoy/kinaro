use gpui_kit::base::input::InputState;
use gpui_kit::component::input::InputEvent;
use gpui_kit::{AppContext, Context, Entity, EventEmitter, SharedString, Subscription, Window};
use ki_utils::ExprValue;
use log::warn;
use std::str::FromStr;

/// Events emitted by an [`ExprValueInputState`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExprValueInputEvent {
  /// Enter was pressed in the input; the typed text has been committed
  PressEnter,
  /// The input lost focus; the typed text has been committed
  Blur,
  /// The input switched between literal and expression mode
  Switched,
}

/// Custom InputState that handles [`ExprValue`]. It adapts its internal [`InputState`] on the value of the passed ExprValue.
///
/// It keeps both type of values internally so that switching from literal (and vice versa) to expression does not lose the
/// other value. The text typed in the input is committed to the value when Enter is pressed, when the input loses focus, and
/// when switching mode.
pub struct ExprValueInputState<T: ToString + FromStr + Clone> {
  value: T,
  expr_value: SharedString,
  expr_flag: bool,
  input_state: Entity<InputState>,
  _sub: Subscription,
}

impl<T: ToString + FromStr + Clone + 'static> ExprValueInputState<T> {
  /// Commits the text of the [`InputState`] to the value of the current mode.
  ///
  /// The text is trimmed. In literal mode, text that does not parse as `T` is discarded and the input shows the last valid
  /// value again.
  fn commit_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let text = self.input_state.read(cx).value();
    let text = text.trim();
    if self.expr_flag {
      self.expr_value = SharedString::new(text);
    } else if let Ok(value) = text.parse::<T>() {
      self.value = value;
    } else {
      warn!("Could not parse value input: {}", text);
      self.write_input(window, cx);
    }
  }

  /// Returns whether the input currently edits an expression (as opposed to a literal).
  pub fn is_expr(&self) -> bool {
    self.expr_flag
  }

  /// Creates an input showing `value`.
  ///
  /// `default_literal` and `default_expr` are the values shown when first switching to the mode `value` is not in.
  pub fn new(
    value: ExprValue<T>,
    default_literal: T,
    default_expr: impl ToString,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    let input_state = cx.new(|cx| InputState::new(window, cx));
    let _sub = cx.subscribe_in(&input_state, window, |this, _, e: &InputEvent, window, cx| match e {
      InputEvent::PressEnter { .. } => {
        this.commit_input(window, cx);
        cx.emit(ExprValueInputEvent::PressEnter);
      }
      InputEvent::Blur => {
        this.commit_input(window, cx);
        cx.emit(ExprValueInputEvent::Blur);
      }
      _ => {}
    });
    let mut this = match value {
      ExprValue::Literal(value) => Self {
        value,
        expr_value: default_expr.to_string().into(),
        expr_flag: false,
        input_state,
        _sub,
      },
      ExprValue::Expr(expr_value) => Self {
        value: default_literal,
        expr_value,
        expr_flag: true,
        input_state,
        _sub,
      },
    };
    this.write_input(window, cx);
    this
  }

  /// Replaces the value with `value`, switching to its mode. The value of the other mode is kept.
  pub fn set_value(&mut self, value: ExprValue<T>, window: &mut Window, cx: &mut Context<Self>) {
    match value {
      ExprValue::Literal(value) => {
        self.value = value;
        self.expr_flag = false;
      }
      ExprValue::Expr(expr_value) => {
        self.expr_value = expr_value;
        self.expr_flag = true;
      }
    }
    self.write_input(window, cx);
    cx.notify();
  }

  /// Returns the inner [`InputState`], to be rendered by an input element.
  pub fn state(&self) -> &Entity<InputState> {
    &self.input_state
  }

  /// Switches to expression mode when `to_expr` is true, to literal mode otherwise, after committing the typed text.
  /// Does nothing when already in that mode.
  ///
  /// # Events
  /// Emits [`ExprValueInputEvent::Switched`] when the mode changes.
  pub fn switch(&mut self, to_expr: bool, window: &mut Window, cx: &mut Context<Self>) {
    if self.expr_flag == to_expr {
      return;
    }
    self.commit_input(window, cx);
    self.expr_flag = to_expr;
    self.write_input(window, cx);
    cx.emit(ExprValueInputEvent::Switched);
    cx.notify();
  }

  /// Returns the committed value of the current mode.
  pub fn value(&self) -> ExprValue<T> {
    if self.expr_flag {
      ExprValue::Expr(self.expr_value.clone())
    } else {
      ExprValue::Literal(self.value.clone())
    }
  }

  /// Writes the value of the current mode in the [`InputState`].
  fn write_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let text: SharedString = if self.expr_flag {
      self.expr_value.clone()
    } else {
      self.value.to_string().into()
    };
    self.input_state.update(cx, |state, cx| state.set_value(text, window, cx));
  }
}

impl<T: ToString + FromStr + Clone + 'static> EventEmitter<ExprValueInputEvent> for ExprValueInputState<T> {}

#[cfg(test)]
mod tests {
  use super::*;
  use gpui_kit::{TestAppContext, VisualTestContext};
  use std::cell::RefCell;
  use std::rc::Rc;

  type Events = Rc<RefCell<Vec<ExprValueInputEvent>>>;

  /// Builds an [`ExprValueInputState<u32>`] from `value` (defaults: `1000` / `"{{default}}"`), and records the events it emits.
  fn input(value: ExprValue<u32>, cx: &mut TestAppContext) -> (Entity<ExprValueInputState<u32>>, Events, &mut VisualTestContext) {
    cx.update(gpui_kit::init);
    let cx = cx.add_empty_window();
    let state = cx.update(|window, cx| cx.new(|cx| ExprValueInputState::new(value, 1000, "{{default}}", window, cx)));
    let events: Events = Default::default();
    let recorded = events.clone();
    cx.update(|_, cx| {
      cx.subscribe(&state, move |_, e: &ExprValueInputEvent, _| recorded.borrow_mut().push(*e))
        .detach()
    });
    (state, events, cx)
  }

  /// Replaces the text of the inner [`InputState`] with `text`, as if typed by the user.
  fn type_text(state: &Entity<ExprValueInputState<u32>>, text: &str, cx: &mut VisualTestContext) {
    let input = state.read_with(cx, |state, _| state.state().clone());
    let text = text.to_string();
    input.update_in(cx, |input, window, cx| input.set_value(text, window, cx));
  }

  /// Makes the inner [`InputState`] emit `event`.
  fn emit_input(state: &Entity<ExprValueInputState<u32>>, event: InputEvent, cx: &mut VisualTestContext) {
    let input = state.read_with(cx, |state, _| state.state().clone());
    input.update(cx, |_, cx| cx.emit(event));
  }

  fn text_of(state: &Entity<ExprValueInputState<u32>>, cx: &mut VisualTestContext) -> String {
    cx.read(|cx| state.read(cx).state().read(cx).value().to_string())
  }

  fn value_of(state: &Entity<ExprValueInputState<u32>>, cx: &mut VisualTestContext) -> ExprValue<u32> {
    state.read_with(cx, |state, _| state.value())
  }

  fn switch(state: &Entity<ExprValueInputState<u32>>, to_expr: bool, cx: &mut VisualTestContext) {
    state.update_in(cx, |state, window, cx| state.switch(to_expr, window, cx));
  }

  #[gpui_kit::test]
  fn new_from_literal(cx: &mut TestAppContext) {
    let (state, events, cx) = input(42.into(), cx);
    assert!(!state.read_with(cx, |state, _| state.is_expr()));
    assert_eq!(value_of(&state, cx), ExprValue::Literal(42));
    assert_eq!(text_of(&state, cx), "42");
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn new_from_expr(cx: &mut TestAppContext) {
    let (state, _, cx) = input(ExprValue::Expr("{{var.delay}}".into()), cx);
    assert!(state.read_with(cx, |state, _| state.is_expr()));
    assert_eq!(value_of(&state, cx), ExprValue::Expr("{{var.delay}}".into()));
    assert_eq!(text_of(&state, cx), "{{var.delay}}");
  }

  #[gpui_kit::test]
  fn blur_commits_typed_literal(cx: &mut TestAppContext) {
    let (state, events, cx) = input(42.into(), cx);
    type_text(&state, "7", cx);
    // Typing alone does not commit
    assert_eq!(value_of(&state, cx), ExprValue::Literal(42));
    emit_input(&state, InputEvent::Blur, cx);
    assert_eq!(value_of(&state, cx), ExprValue::Literal(7));
    assert_eq!(*events.borrow(), vec![ExprValueInputEvent::Blur]);
  }

  #[gpui_kit::test]
  fn press_enter_commits_typed_expr(cx: &mut TestAppContext) {
    let (state, events, cx) = input(ExprValue::Expr("{{var.a}}".into()), cx);
    type_text(&state, "{{var.b}}", cx);
    emit_input(
      &state,
      InputEvent::PressEnter {
        secondary: false,
        shift: false,
      },
      cx,
    );
    assert_eq!(value_of(&state, cx), ExprValue::Expr("{{var.b}}".into()));
    assert_eq!(*events.borrow(), vec![ExprValueInputEvent::PressEnter]);
  }

  #[gpui_kit::test]
  fn other_input_events_are_ignored(cx: &mut TestAppContext) {
    let (state, events, cx) = input(42.into(), cx);
    type_text(&state, "7", cx);
    emit_input(&state, InputEvent::Focus, cx);
    emit_input(&state, InputEvent::Change, cx);
    assert_eq!(value_of(&state, cx), ExprValue::Literal(42));
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn invalid_literal_keeps_previous_value(cx: &mut TestAppContext) {
    let (state, events, cx) = input(42.into(), cx);
    type_text(&state, "abc", cx);
    emit_input(&state, InputEvent::Blur, cx);
    assert_eq!(value_of(&state, cx), ExprValue::Literal(42));
    assert_eq!(text_of(&state, cx), "42");
    assert_eq!(*events.borrow(), vec![ExprValueInputEvent::Blur]);
  }

  #[gpui_kit::test]
  fn switch_to_expr_starts_from_default_expr(cx: &mut TestAppContext) {
    let (state, events, cx) = input(42.into(), cx);
    switch(&state, true, cx);
    assert!(state.read_with(cx, |state, _| state.is_expr()));
    assert_eq!(value_of(&state, cx), ExprValue::Expr("{{default}}".into()));
    assert_eq!(text_of(&state, cx), "{{default}}");
    assert_eq!(*events.borrow(), vec![ExprValueInputEvent::Switched]);
  }

  #[gpui_kit::test]
  fn switch_to_literal_starts_from_default_literal(cx: &mut TestAppContext) {
    let (state, events, cx) = input(ExprValue::Expr("{{var.a}}".into()), cx);
    switch(&state, false, cx);
    assert!(!state.read_with(cx, |state, _| state.is_expr()));
    assert_eq!(value_of(&state, cx), ExprValue::Literal(1000));
    assert_eq!(text_of(&state, cx), "1000");
    assert_eq!(*events.borrow(), vec![ExprValueInputEvent::Switched]);
  }

  #[gpui_kit::test]
  fn switch_to_current_mode_does_nothing(cx: &mut TestAppContext) {
    let (state, events, cx) = input(42.into(), cx);
    type_text(&state, "7", cx);
    switch(&state, false, cx);
    assert_eq!(value_of(&state, cx), ExprValue::Literal(42));
    assert_eq!(text_of(&state, cx), "7");
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn switch_back_and_forth_keeps_both_values(cx: &mut TestAppContext) {
    let (state, _, cx) = input(42.into(), cx);
    // Uncommitted text is committed by the switch
    type_text(&state, "7", cx);
    switch(&state, true, cx);
    type_text(&state, "{{var.b}}", cx);
    switch(&state, false, cx);
    assert_eq!(value_of(&state, cx), ExprValue::Literal(7));
    assert_eq!(text_of(&state, cx), "7");
    switch(&state, true, cx);
    assert_eq!(value_of(&state, cx), ExprValue::Expr("{{var.b}}".into()));
    assert_eq!(text_of(&state, cx), "{{var.b}}");
  }

  #[gpui_kit::test]
  fn set_value_changes_mode_and_text(cx: &mut TestAppContext) {
    let (state, events, cx) = input(42.into(), cx);
    state.update_in(cx, |state, window, cx| {
      state.set_value(ExprValue::Expr("{{var.a}}".into()), window, cx)
    });
    assert!(state.read_with(cx, |state, _| state.is_expr()));
    assert_eq!(value_of(&state, cx), ExprValue::Expr("{{var.a}}".into()));
    assert_eq!(text_of(&state, cx), "{{var.a}}");
    state.update_in(cx, |state, window, cx| state.set_value(ExprValue::Literal(5), window, cx));
    assert!(!state.read_with(cx, |state, _| state.is_expr()));
    assert_eq!(text_of(&state, cx), "5");
    // The expression set before is kept for the next switch
    switch(&state, true, cx);
    assert_eq!(value_of(&state, cx), ExprValue::Expr("{{var.a}}".into()));
    assert_eq!(*events.borrow(), vec![ExprValueInputEvent::Switched]);
  }

  #[gpui_kit::test]
  fn committed_text_is_trimmed(cx: &mut TestAppContext) {
    let (state, _, cx) = input(42.into(), cx);
    type_text(&state, " 7 ", cx);
    emit_input(&state, InputEvent::Blur, cx);
    assert_eq!(value_of(&state, cx), ExprValue::Literal(7));
    switch(&state, true, cx);
    type_text(&state, " {{var.a}} ", cx);
    emit_input(&state, InputEvent::Blur, cx);
    assert_eq!(value_of(&state, cx), ExprValue::Expr("{{var.a}}".into()));
  }

  #[gpui_kit::test]
  fn set_value_notifies_observers(cx: &mut TestAppContext) {
    let (state, _, cx) = input(42.into(), cx);
    let notified = Rc::new(RefCell::new(0));
    let count = notified.clone();
    cx.update(|_, cx| cx.observe(&state, move |_, _| *count.borrow_mut() += 1).detach());
    state.update_in(cx, |state, window, cx| {
      state.set_value(ExprValue::Expr("{{var.a}}".into()), window, cx)
    });
    assert_eq!(*notified.borrow(), 1);
  }
}
