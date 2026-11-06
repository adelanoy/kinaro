use gpui_kit::base::input::{InputEvent, InputState};
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::Input;
use gpui_kit::component::label::Label;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu};
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{ActiveTheme, FocusableExt, IconName, WindowExt};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
  App, AppContext, Context, Entity, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, Render, SharedString,
  Styled, Subscription, Window, px,
};
use log::error;
use ki_app_shared::ContextMenuBuilder;
use ki_app_shared::actions::{Delete, Duplicate, Edit, New};
use ki_app_shared::table::{DataTableControlBar, EditedCell};
use ki_assets::icon::IconAsset;
use ki_workspace::test::step_rest::{FormField, RestBody, RestData};

pub(super) struct UrlEncodedTable {
  table_state: Entity<TableState<UrlEncodedTableDelegate>>,
  _table_sub: Subscription,
}

impl UrlEncodedTable {
  pub(super) fn new(rest_data: Entity<RestData>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let table_state = cx.new(|cx| {
      TableState::new(UrlEncodedTableDelegate::new(rest_data, window, cx), window, cx)
        .cell_selectable(true)
        .row_selectable(true)
        .col_selectable(false)
    });
    let _table_sub = cx.subscribe_in(&table_state, window, Self::on_table_event);
    Self { table_state, _table_sub }
  }

  fn on_action_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(row_ix) = self
      .table_state
      .read(cx)
      .selected_row()
      .or_else(|| self.table_state.read(cx).selected_cell().map(|(row, _)| row))
    {
      let table_state = self.table_state.clone();
      window.open_alert_dialog(cx, {
        move |dialog, _, _| {
          dialog
            .title("Delete Field")
            .description("Confirm to delete the URL-Encoded field")
            .show_cancel(true)
            .on_ok({
              let this = table_state.clone();
              move |_, _, cx| {
                this.update(cx, |this, cx| {
                  this.delegate_mut().delete_field(row_ix, cx);
                  this.clear_selection(cx);
                });
                true
              }
            })
        }
      });
    }
  }

  fn on_action_duplicate(&mut self, _: &Duplicate, _window: &mut Window, cx: &mut Context<Self>) {
    if let Some(row_ix) = self
      .table_state
      .read(cx)
      .selected_row()
      .or_else(|| self.table_state.read(cx).selected_cell().map(|(row, _)| row))
    {
      self.table_state.update(cx, |this, cx| {
        this.delegate_mut().duplicate_field(row_ix, cx);
      });
    }
  }

  fn on_action_edit(&mut self, _: &Edit, window: &mut Window, cx: &mut Context<Self>) {
    self.table_state.update(cx, |this, cx| {
      let Some(cell) = this.selected_cell() else {
        return;
      };
      this.delegate_mut().begin_edit(cell.0, cell.1, window, cx)
    });
  }

  fn on_action_new(&mut self, _: &New, _window: &mut Window, cx: &mut Context<Self>) {
    let row_ix = self
      .table_state
      .read(cx)
      .selected_row()
      .or_else(|| self.table_state.read(cx).selected_cell().map(|(row, _)| row));
    self.table_state.update(cx, |this, cx| {
      this.delegate_mut().add_field(row_ix, cx);
    });
  }

  fn on_table_event(
    &mut self,
    table: &Entity<TableState<UrlEncodedTableDelegate>>,
    event: &TableEvent,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    table.update(cx, |this, cx| match event {
      TableEvent::SelectCell(row_ix, col_ix) => this.delegate_mut().select_cell(*row_ix, *col_ix, window, cx),
      TableEvent::DoubleClickedCell(row_ix, col_ix) => this.delegate_mut().begin_edit(*row_ix, *col_ix, window, cx),
      TableEvent::ClearSelection => this.delegate_mut().clear_selection(),
      _ => {}
    });
  }
}

struct UrlEncodedTableDelegate {
  rest_data: Entity<RestData>,
  table_columns: Vec<Column>,
  edited_cell: Option<EditedCell<FormField>>,
  cell_input_state: Entity<InputState>,
  _cell_input_sub: Option<Subscription>,
}

impl UrlEncodedTableDelegate {
  fn add_field(&mut self, row_ix: Option<usize>, cx: &mut Context<TableState<Self>>) {
    self
      .rest_data
      .update(cx, |params, cx| params.add_urlencoded_field(row_ix, cx));
  }

  fn begin_edit(&mut self, row_ix: usize, col_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if col_ix != 0 && self.edited_cell.is_none() {
      let fields = self.with_fields(cx, |fields| fields[row_ix].clone());
      match col_ix {
        1 => {
          self.cell_input_state.update(cx, |state, cx| {
            state.set_value(fields.name.clone(), window, cx);
            state.focus(window, cx);
          });
          self._cell_input_sub = Some(cx.subscribe_in(
            &self.cell_input_state,
            window,
            move |table, state, e: &InputEvent, window, cx| {
              Self::on_cell_input_event(table, state, e, window, cx, |param, value| param.name = value)
            },
          ));
        }
        2 => {
          self.cell_input_state.update(cx, |state, cx| {
            state.set_value(fields.value.clone(), window, cx);
            state.focus(window, cx);
          });
          self._cell_input_sub = Some(cx.subscribe_in(
            &self.cell_input_state,
            window,
            move |table, state, e: &InputEvent, window, cx| {
              Self::on_cell_input_event(table, state, e, window, cx, |param, value| param.value = value)
            },
          ));
        }
        _ => unreachable!(),
      }
      self.edited_cell = Some(EditedCell {
        row: row_ix,
        col: col_ix,
        data: fields,
      });
    }
  }

  fn build_cell_context_menu(col_ix: usize) -> Box<ContextMenuBuilder> {
    let builder = move |menu: PopupMenu, _window: &mut Window, _cx: &mut Context<PopupMenu>| {
      let menu = if col_ix != 0 {
        menu.menu_with_icon("Edit", IconAsset::Pencil, Box::new(Edit)).separator()
      } else {
        menu
      };
      Self::build_row_context_menu(menu)
    };
    Box::new(builder)
  }

  fn build_row_context_menu(menu: PopupMenu) -> PopupMenu {
    menu
      .menu_with_icon("New", IconName::Plus, Box::new(New))
      .menu_with_icon("Duplicate", IconName::Copy, Box::new(Duplicate))
      .separator()
      .menu_with_icon("Delete", IconName::Delete, Box::new(Delete))
  }

  fn clear_selection(&mut self) {
    self.edited_cell = None;
    self._cell_input_sub = None;
  }

  fn delete_field(&mut self, row_ix: usize, cx: &mut App) {
    self
      .rest_data
      .update(cx, |params, cx| params.delete_urlencoded_field(row_ix, cx));
  }

  fn duplicate_field(&mut self, row_ix: usize, cx: &mut Context<TableState<Self>>) {
    self
      .rest_data
      .update(cx, |params, cx| params.duplicate_urlencoded_field(row_ix, cx));
  }

  fn new(rest_data: Entity<RestData>, window: &mut Window, cx: &mut Context<TableState<Self>>) -> Self {
    let table_columns = vec![
      Column::new("field_enabled", "Enable").width(px(75.0)),
      Column::new("field_name", "Name").width(px(200.0)),
      Column::new("field_value", "Value").width(px(300.0)),
    ];

    let cell_input_state = cx.new(|cx| InputState::new(window, cx));

    Self {
      rest_data,
      table_columns,
      edited_cell: None,
      cell_input_state,
      _cell_input_sub: None,
    }
  }

  fn on_cell_input_event<F>(
    table: &mut TableState<Self>,
    state: &Entity<InputState>,
    event: &InputEvent,
    window: &mut Window,
    cx: &mut Context<TableState<Self>>,
    update_func: F,
  ) where
    F: FnOnce(&mut FormField, SharedString),
  {
    match event {
      InputEvent::Change => {
        if let Some(edited) = &mut table.delegate_mut().edited_cell {
          let text = state.read(cx).value();
          update_func(&mut edited.data, text);
        }
      }
      InputEvent::PressEnter { .. } | InputEvent::Blur => table.delegate_mut().save_cell(window, cx),
      _ => {}
    }
  }

  fn save_cell(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let Some(edited) = self.edited_cell.take() {
      self
        .rest_data
        .update(cx, |rest, cx| rest.update_urlencoded_field(edited.row, edited.data, cx));
      self._cell_input_sub = None;
      if self.cell_input_state.read(cx).focus_handle(cx).is_focused(window) {
        cx.focus_self(window);
      }
      cx.notify();
    }
  }

  fn select_cell(&mut self, row_ix: usize, col_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if self.edited_cell.as_ref().is_some_and(|edited| edited.is_at(row_ix, col_ix)) {
      return;
    }
    self.save_cell(window, cx);
  }

  fn with_fields<T>(&self, cx: &App, f: impl Fn(&Vec<FormField>) -> T) -> T {
    match self.rest_data.read(cx).body() {
      RestBody::FormUrlEncoded { fields, .. } => f(fields),
      _ => {
        error!("The UrlEncodedTable is being used with a Rest Step other than FormUrlEncoded");
        f(&vec![])
      },
    }
  }
}

impl TableDelegate for UrlEncodedTableDelegate {
  fn columns_count(&self, _cx: &App) -> usize {
    self.table_columns.len()
  }

  fn rows_count(&self, cx: &App) -> usize {
    self.with_fields(cx, |fields| fields.len())
  }

  fn column(&self, col_ix: usize, _cx: &App) -> Column {
    self.table_columns[col_ix].clone()
  }

  fn context_menu(
    &mut self,
    _row_ix: usize,
    menu: PopupMenu,
    _window: &mut Window,
    _cx: &mut Context<TableState<Self>>,
  ) -> PopupMenu {
    Self::build_row_context_menu(menu)
  }

  fn render_td(
    &mut self,
    row_ix: usize,
    col_ix: usize,
    _window: &mut Window,
    cx: &mut Context<TableState<Self>>,
  ) -> impl IntoElement {
    if let Some(edited) = &self.edited_cell
      && col_ix != 0
      && edited.is_at(row_ix, col_ix)
    {
      match col_ix {
        1 | 2 => Input::new(&self.cell_input_state).into_any_element(),
        _ => unreachable!(),
      }
    } else {
      match self.rest_data.read(cx).body() {
        RestBody::FormUrlEncoded { fields, .. } => {
          let field = &fields[row_ix];
          let is_disabled = field.disabled;
          match col_ix {
            0 => h_flex()
              .size_full()
              .items_center()
              // Force selection on right click, or the .context_menu below will consume the event, hence the table never receives the rightClick event
              .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |table, _, window, cx| {
                  table.focus_handle(cx).focus(window, cx);
                  table.set_selected_cell(row_ix, col_ix, cx);
                }),
              )
              .child(
                Checkbox::new(format!("field-enabled-{}", row_ix))
                  .checked(!is_disabled)
                  .on_change(cx.listener(move |table, value, _, cx| {
                    let delegate = table.delegate_mut();
                    let mut field = delegate.with_fields(cx, |fields| fields[row_ix].clone());
                    field.disabled = !value;
                    delegate
                      .rest_data
                      .update(cx, |rest, cx| rest.update_urlencoded_field(row_ix, field, cx));
                    cx.notify();
                  })),
              )
              .context_menu(Self::build_cell_context_menu(0))
              .into_any_element(),
            1 => h_flex()
              .size_full()
              .child(Label::new(field.name.clone()).when(is_disabled, |label| label.text_color(cx.theme().muted_foreground)))
              .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |table, _, window, cx| {
                  table.focus_handle(cx).focus(window, cx);
                  table.set_selected_cell(row_ix, col_ix, cx);
                }),
              )
              .context_menu(Self::build_cell_context_menu(1))
              .into_any_element(),
            2 => h_flex()
              .size_full()
              .child(Label::new(field.value.clone()).when(is_disabled, |label| label.text_color(cx.theme().muted_foreground)))
              .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |table, _, window, cx| {
                  table.focus_handle(cx).focus(window, cx);
                  table.set_selected_cell(row_ix, col_ix, cx);
                }),
              )
              .context_menu(Self::build_cell_context_menu(2))
              .into_any_element(),
            _ => unreachable!(),
          }
        }
        _ => unreachable!(),
      }
    }
  }
}

impl Render for UrlEncodedTable {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let focus_handle = self.table_state.focus_handle(cx);
    let (is_editing, is_selection) = self.table_state.read_with(cx, |table, _| {
      (
        table.delegate().edited_cell.is_some(),
        table.selected_cell().is_some() || table.selected_row().is_some(),
      )
    });

    v_flex()
      .track_focus(&focus_handle)
      .on_action(cx.listener(Self::on_action_delete))
      .on_action(cx.listener(Self::on_action_duplicate))
      .on_action(cx.listener(Self::on_action_edit))
      .on_action(cx.listener(Self::on_action_new))
      .size_full()
      .p_1()
      .gap_y_2()
      .child(
        h_flex()
          .w_full()
          .justify_end()
          .child(DataTableControlBar::new(focus_handle, is_editing, is_selection)),
      )
      .child(DataTable::new(&self.table_state).focus_ring(false))
  }
}
