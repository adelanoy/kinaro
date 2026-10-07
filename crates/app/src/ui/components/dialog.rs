use gpui_kit::base::input::InputState;
use gpui_kit::base::{markdown, v_flex, v_resizable};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::dialog::{DialogAction, DialogClose, DialogFooter};
use gpui_kit::component::input::{Input, Textarea, TextareaState};
use gpui_kit::component::{ActiveTheme, WindowExt};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{App, AppContext, Entity, IntoElement, ParentElement, SharedString, Styled, Window, div};
use ki_utils::TestPath;
use ki_workspace::test::TestsContainer;

/// Opens a dialog to rename the node at `path`, prefilled with `current_name`.
///
/// On confirmation the node is renamed in `tests`, and a notification is
/// pushed to `window` if that fails.
pub fn show_rename_dialog(
  current_name: SharedString,
  path: TestPath,
  tests: Entity<TestsContainer>,
  window: &mut Window,
  cx: &mut App,
) {
  let input = cx.new(|cx| {
    let mut state = InputState::new(window, cx);
    state.set_value(current_name, window, cx);
    state
  });
  window.open_dialog(cx, move |dialog, window, cx| {
    input.update(cx, |input, cx| {
      input.focus(window, cx);
    });
    dialog
      .title("Rename test")
      .child(v_flex().gap_3().child("Enter the node's name:").child(Input::new(&input)))
      .footer(
        DialogFooter::new()
          .child(DialogClose::new().child(Button::new("cancel").label("Cancel").outline()))
          .child(DialogAction::new().child(Button::new("confirm").primary().label("Rename"))),
      )
      .on_ok({
        let name = input.read(cx).value();
        let tests = tests.clone();
        move |_, window, cx| {
          if let Err(err) = tests.update(cx, |tests, cx| tests.rename_at(path, name.clone(), cx)) {
            window.push_notification(err, cx);
          }
          true
        }
      })
  });
}

/// Opens a dialog to edit the description of the node at `path`, prefilled
/// with `current_desc`. The text is edited as markdown, next to a live preview
/// of its rendering.
///
/// On confirmation the description is set in `tests` (removed if left empty),
/// and a notification is pushed to `window` if that fails.
pub fn show_edit_desc_dialog(
  current_desc: SharedString,
  path: TestPath,
  tests: Entity<TestsContainer>,
  window: &mut Window,
  cx: &mut App,
) {
  let text_area_state = cx.new(|cx| {
    let mut state = TextareaState::new(window, cx);
    state.set_value(current_desc, window, cx);
    state
  });
  window.open_dialog(cx, move |dialog, window, cx| {
    text_area_state.update(cx, |input, cx| {
      input.focus(window, cx);
    });
    let size = window.viewport_size().map(|size| size * 0.7);
    dialog
      .title("Edit Description")
      .w(size.width)
      .h(size.height)
      .child(
        v_resizable("desc-edit")
          .child(
            div()
              .p_1()
              .w_full()
              .border_color(cx.theme().border)
              .border_1()
              .rounded_md()
              .child(Textarea::new(&text_area_state).h_full())
              .into_any_element(),
          )
          .child(
            div()
              .p_1()
              .size_full()
              .border_color(cx.theme().border)
              .border_1()
              .rounded_md()
              .child(markdown(text_area_state.read(cx).value()))
              .into_any_element(),
          ),
      )
      .footer(
        DialogFooter::new()
          .child(DialogClose::new().child(Button::new("cancel").label("Cancel").outline()))
          .child(DialogAction::new().child(Button::new("confirm").primary().label("Confirm"))),
      )
      .on_ok({
        let desc = text_area_state
          .read(cx)
          .value()
          .map(|value| if value.is_empty() { None } else { Some(value) });
        let tests = tests.clone();
        move |_, window, cx| {
          if let Err(err) = tests.update(cx, |tests, cx| tests.edit_description_at(path, desc.clone(), cx)) {
            window.push_notification(err, cx);
          }
          true
        }
      })
  });
}
