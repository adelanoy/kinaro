use crate::actions::{Delete, Duplicate, New};
use gpui_kit::base::h_flex;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::{Disableable, IconName, Sizable};
use gpui_kit::{App, FocusHandle, IntoElement, ParentElement, RenderOnce, Styled, Window};

/// A table cell being edited, along with its edited value not yet written back.
pub struct EditedCell<T> {
  pub row: usize,
  pub col: usize,
  pub data: T,
}

impl<T> EditedCell<T> {
  /// Whether this is the cell at `row`/`col`.
  pub fn is_at(&self, row: usize, col: usize) -> bool {
    (self.row, self.col) == (row, col)
  }
}

/// Generic DataTable control bar with Add/Duplicate/Delete buttons emitting each their related [`gpui_kit::Action`] from the actions module
#[derive(IntoElement)]
pub struct DataTableControlBar {
  focus_handle: FocusHandle,
  is_editing: bool,
  is_selection: bool,
}

impl DataTableControlBar {
  pub fn new(focus_handle: FocusHandle, is_editing: bool, is_selection: bool) -> Self {
    Self {
      focus_handle,
      is_editing,
      is_selection,
    }
  }
}

impl RenderOnce for DataTableControlBar {
  fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
    h_flex()
      .gap_x_2()
      .child(
        Button::new("btn-add-row")
          .ghost()
          .small()
          .disabled(self.is_editing)
          .icon(IconName::Plus)
          .tooltip_with_action("Add a new row", &New, None)
          .on_click({
            let focus_handle = self.focus_handle.clone();
            move |_, window, cx| {
              focus_handle.focus(window, cx);
              window.dispatch_action(Box::new(New), cx)
            }
          }),
      )
      .child(
        Button::new("btn-duplicate-row")
          .ghost()
          .small()
          .disabled(self.is_editing || !self.is_selection)
          .icon(IconName::Copy)
          .tooltip_with_action("Duplicate the selected row", &Duplicate, None)
          .on_click({
            let focus_handle = self.focus_handle.clone();
            move |_, window, cx| {
              focus_handle.focus(window, cx);
              window.dispatch_action(Box::new(Duplicate), cx)
            }
          }),
      )
      .child(Separator::vertical())
      .child(
        Button::new("btn-delete-row")
          .ghost()
          .small()
          .disabled(self.is_editing || !self.is_selection)
          .icon(IconName::Delete)
          .tooltip_with_action("Delete the selected row", &Delete, None)
          .on_click({
            move |_, window, cx| {
              self.focus_handle.focus(window, cx);
              window.dispatch_action(Box::new(Delete), cx)
            }
          }),
      )
  }
}
