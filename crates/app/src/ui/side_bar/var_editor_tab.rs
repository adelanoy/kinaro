use crate::ui::components::profile_select;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::empty::{EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::label::Label;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{
  ActiveTheme, Disableable, FocusableExt, Icon, IconName, IndexPath, Sizable, WindowExt, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use ki_app_shared::actions::{Delete, Duplicate, Edit, New};
use ki_app_shared::panel::PanelDescriptor;
use ki_app_shared::{ContextMenuBuilder, EditedCell, MovingLabel};
use ki_assets::icon::IconAsset;
use ki_utils::SidebarPanel;
use ki_workspace::Project;
use ki_workspace::variable::{DraggedVar, Profile, ProjectVariables, VariableKind, VariableReference};
use log::warn;
use uuid::Uuid;

pub struct VariableEditor {
  table_state: Entity<TableState<VariableDataTableDelegate>>,
}

impl VariableEditor {
  fn on_action_delete(&mut self, _action: &Delete, window: &mut Window, cx: &mut Context<Self>) {
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
            .title("Delete Variable")
            .description("Are you sure you want to delete the variable?")
            .show_cancel(true)
            .on_ok({
              let this = table_state.clone();
              move |_, window, cx| {
                this.update(cx, |this, cx| {
                  this.delegate_mut().delete_variable(row_ix, window, cx);
                  this.clear_selection(cx);
                });
                true
              }
            })
        }
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

  fn on_action_duplicate(&mut self, _action: &Duplicate, _window: &mut Window, cx: &mut Context<Self>) {
    if let Some(row_ix) = self
      .table_state
      .read(cx)
      .selected_row()
      .or_else(|| self.table_state.read(cx).selected_cell().map(|(row, _)| row))
    {
      self.table_state.update(cx, |this, cx| {
        this.delegate_mut().duplicate_variable(row_ix, cx);
      });
    }
  }

  fn on_action_new(&mut self, _action: &New, _window: &mut Window, cx: &mut Context<Self>) {
    self.table_state.update(cx, |table_state, cx| {
      let row_ix = table_state
        .selected_row()
        .or_else(|| table_state.selected_cell().map(|(row, _)| row));
      table_state.delegate_mut().add_variable(row_ix, cx);
    });
  }

  fn on_table_event(
    &mut self,
    table: &Entity<TableState<VariableDataTableDelegate>>,
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

impl PanelDescriptor<SidebarPanel, Project> for VariableEditor {
  fn name() -> &'static str {
    "Variables"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::Variable
  }

  fn panel_type() -> SidebarPanel {
    SidebarPanel::Variable
  }

  fn new(project: Entity<Project>, window: &mut Window, cx: &mut App) -> Entity<Self> {
    cx.new(|cx| {
      let table_state = cx.new(|cx| {
        TableState::new(VariableDataTableDelegate::new(project, window, cx), window, cx)
          .row_selectable(true)
          .col_selectable(false)
          .cell_selectable(true)
      });
      cx.subscribe_in(&table_state, window, Self::on_table_event).detach();

      Self { table_state }
    })
  }
}

pub struct VariableDataTableDelegate {
  project_vars: Entity<ProjectVariables>,
  table_columns: Vec<Column>,
  edited_cell: Option<EditedCell<VariableReference>>,
  profile_select_state: Entity<SelectState<Vec<Profile>>>,
  cell_input_state: Entity<InputState>,
  _cell_input_sub: Option<Subscription>,
  var_kind_select_state: Entity<SelectState<Vec<VariableKind>>>,
  _var_kind_sub: Subscription,
  viewing_profile: Option<Uuid>,
}

impl VariableDataTableDelegate {
  fn add_variable(&self, current_row: Option<usize>, cx: &mut Context<TableState<Self>>) {
    self.project_vars.update(cx, |this, cx| this.add_variable(current_row, cx));
    cx.notify();
  }

  fn build_cell_context_menu() -> Box<ContextMenuBuilder> {
    let builder = move |menu: PopupMenu, _window: &mut Window, _cx: &mut Context<PopupMenu>| {
      let menu = menu.menu_with_icon("Edit", IconAsset::Pencil, Box::new(Edit)).separator();
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

  fn begin_edit(&mut self, row_ix: usize, col_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if self.edited_cell.is_none() {
      let var = self.project_vars.read(cx).references.get(row_ix).unwrap().clone();
      match col_ix {
        0 => {
          self.cell_input_state.update(cx, |state, cx| {
            state.set_value(var.name.clone(), window, cx);
            state.focus(window, cx);
          });
          self._cell_input_sub = Some(cx.subscribe_in(
            &self.cell_input_state,
            window,
            move |table, input_state, event, window, cx| {
              Self::on_cell_input_event(table, input_state, event, window, cx, |var, value| var.name = value)
            },
          ));
        }
        1 => {
          self.var_kind_select_state.update(cx, |state, cx| {
            state.set_selected_value(&var.kind, window, cx);
            state.focus(window, cx);
          });
        }
        2 => {
          let (_, effective_value) = self.effective_value(&var);
          self.cell_input_state.update(cx, |state, cx| {
            state.set_value(effective_value, window, cx);
            state.focus(window, cx);
          });
          let current_profile = self.viewing_profile;
          self._cell_input_sub = Some(cx.subscribe_in(
            &self.cell_input_state,
            window,
            move |table, input_state, event, window, cx| {
              Self::on_cell_input_event(table, input_state, event, window, cx, |var, value| {
                var.set_value(value, current_profile)
              })
            },
          ));
        }
        3 => {
          self.cell_input_state.update(cx, |state, cx| {
            state.set_value(var.description.clone(), window, cx);
            state.focus(window, cx);
          });
          self._cell_input_sub = Some(cx.subscribe_in(
            &self.cell_input_state,
            window,
            move |table, input_state, event, window, cx| {
              Self::on_cell_input_event(table, input_state, event, window, cx, |var, value| var.description = value)
            },
          ));
        }
        _ => unreachable!(),
      }

      self.edited_cell = Some(EditedCell {
        row: row_ix,
        col: col_ix,
        data: var,
      });
    }
  }

  fn clear_selection(&mut self) {
    self.edited_cell = None;
    self._cell_input_sub = None;
  }

  fn delete_variable(&mut self, row_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let Err(err) = self.project_vars.update(cx, |this, cx| this.delete_variable(row_ix, cx)) {
      window.push_notification(err, cx);
    }
    cx.notify();
  }

  fn duplicate_variable(&mut self, row_ix: usize, cx: &mut Context<TableState<Self>>) {
    self.project_vars.update(cx, |this, cx| this.duplicate_variable(row_ix, cx));
    cx.notify();
  }

  fn new(project: Entity<Project>, window: &mut Window, cx: &mut Context<TableState<Self>>) -> Self {
    let table_columns = vec![
      Column::new("var_name", "Name").width(px(150.0)),
      Column::new("var_kind", "Type").width(px(120.0)),
      Column::new("var_value", "Value").width(px(200.0)),
      Column::new("var_description", "Description").width(px(250.0)),
    ];
    let vars = project.read(cx).variables.clone();

    let profile_select_state = profile_select(&vars, window, cx);
    cx.subscribe_in(
      &profile_select_state,
      window,
      move |table, _, event: &SelectEvent<Vec<Profile>>, _, _| match event {
        SelectEvent::Confirm(profile) => table.delegate_mut().viewing_profile = *profile,
      },
    )
    .detach();

    let var_kind_select_state = cx.new(|cx| {
      SelectState::new(
        vec![VariableKind::Text, VariableKind::PasswordClear, VariableKind::PasswordEncrypt],
        Some(IndexPath::default()),
        window,
        cx,
      )
    });
    let _var_kind_sub = cx.subscribe_in(
      &var_kind_select_state,
      window,
      move |table, _, event: &SelectEvent<Vec<VariableKind>>, _, _| match event {
        SelectEvent::Confirm(value) if value.is_some() => {
          if let Some(edited) = &mut table.delegate_mut().edited_cell {
            edited.data.kind = value.unwrap();
          }
        }
        SelectEvent::Confirm(_) => {}
      },
    );

    let cell_input_state = cx.new(|cx| InputState::new(window, cx));

    Self {
      project_vars: vars,
      table_columns,
      edited_cell: None,
      profile_select_state,
      cell_input_state,
      _cell_input_sub: None,
      var_kind_select_state,
      _var_kind_sub,
      viewing_profile: None,
    }
  }

  fn revert_override(&mut self, row_ix: usize, cx: &mut Context<TableState<Self>>) {
    let Some(profile_id) = self.viewing_profile else {
      return;
    };
    self.project_vars.update(cx, |this, cx| {
      let Some(var) = this.references.get_mut(row_ix) else {
        warn!("revert_override: Unknown ix: {}", row_ix);
        return;
      };
      var.revert_override(profile_id, cx);
    });
  }

  fn on_cell_input_event<F>(
    table: &mut TableState<Self>,
    state: &Entity<InputState>,
    event: &InputEvent,
    window: &mut Window,
    cx: &mut Context<TableState<Self>>,
    f: F,
  ) where
    F: FnOnce(&mut VariableReference, SharedString),
  {
    match event {
      InputEvent::Change => {
        if let Some(edited) = &mut table.delegate_mut().edited_cell {
          let text = state.read(cx).value();
          f(&mut edited.data, text);
        }
      }
      InputEvent::PressEnter { .. } | InputEvent::Blur => table.delegate_mut().save_cell(window, cx),
      _ => {}
    }
  }

  fn select_cell(&mut self, row_ix: usize, col_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if self.edited_cell.as_ref().is_some_and(|edited| edited.is_at(row_ix, col_ix)) {
      return;
    }
    self.save_cell(window, cx);
  }

  fn save_cell(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let Some(edited) = self.edited_cell.take() {
      if let Err(err) = self.project_vars.update(cx, |vars, cx| vars.update_variable(edited.data, cx)) {
        window.push_notification(err, cx);
      }
      self._cell_input_sub = None;
      if self.cell_input_state.read(cx).focus_handle(cx).is_focused(window)
        || self
          .var_kind_select_state
          .read(cx)
          .focus_handle(cx)
          .contains_focused(window, cx)
      {
        cx.focus_self(window);
      }
      cx.notify();
    }
  }

  fn move_variable(&mut self, from: Uuid, to: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let Err(err) = self.project_vars.update(cx, |this, cx| this.move_variable(from, to, cx)) {
      window.push_notification(err, cx);
    }
  }

  fn effective_value(&self, var: &VariableReference) -> (bool, SharedString) {
    let Some(profile_id) = self.viewing_profile else {
      return (false, var.value.clone());
    };
    var
      .find_override(&profile_id)
      .map_or_else(|| (false, var.value.clone()), |value| (true, value.clone()))
  }
}

impl TableDelegate for VariableDataTableDelegate {
  fn columns_count(&self, _cx: &App) -> usize {
    self.table_columns.len()
  }

  fn rows_count(&self, cx: &App) -> usize {
    self.project_vars.read(cx).references.len()
  }

  fn column(&self, col_ix: usize, _cx: &App) -> Column {
    self.table_columns[col_ix].clone()
  }

  fn render_tr(&mut self, row_ix: usize, _window: &mut Window, cx: &mut Context<TableState<Self>>) -> Stateful<Div> {
    let is_editing = self.edited_cell.is_some();
    let id = self.project_vars.read(cx).references.get(row_ix).unwrap().id;
    div().id(("row", row_ix)).when(!is_editing, |this| {
      this
        .on_drag(
          MovingLabel {
            label: self.project_vars.read(cx).references.get(row_ix).unwrap().name.clone(),
            data: DraggedVar(id),
          },
          |drag, _, _, cx| {
            cx.stop_propagation();
            cx.new(|_| drag.clone())
          },
        )
        .on_drop(cx.listener(move |table, e: &MovingLabel<DraggedVar>, window, cx| {
          table.delegate_mut().move_variable(e.data.0, row_ix, window, cx);
        }))
    })
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
    let var = &self.project_vars.read(cx).references[row_ix];
    match &self.edited_cell {
      Some(edited) if edited.is_at(row_ix, col_ix) => match col_ix {
        0 | 2 | 3 => Input::new(&self.cell_input_state).small().into_any_element(),
        1 => Select::new(&self.var_kind_select_state).small().into_any_element(),
        _ => unreachable!(),
      },
      _ => match col_ix {
        0 => h_flex()
          .size_full()
          .child(var.name.clone())
          // Force selection on right click, or the .context_menu below will consume the event, hence the table never receives the rightClick event
          .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |table, _, window, cx| {
              table.focus_handle(cx).focus(window, cx);
              table.set_selected_cell(row_ix, col_ix, cx);
            }),
          )
          .context_menu(Self::build_cell_context_menu())
          .into_any_element(),
        1 => h_flex()
          .size_full()
          .child(SharedString::new(format!("{}", var.kind)))
          .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |table, _, window, cx| {
              table.focus_handle(cx).focus(window, cx);
              table.set_selected_cell(row_ix, col_ix, cx);
            }),
          )
          .context_menu(Self::build_cell_context_menu())
          .into_any_element(),
        2 => {
          let (overridden, effective_value) = self.effective_value(var);
          h_flex()
            .size_full()
            .on_mouse_down(
              MouseButton::Right,
              cx.listener(move |table, _, window, cx| {
                table.focus_handle(cx).focus(window, cx);
                table.set_selected_cell(row_ix, col_ix, cx);
              }),
            )
            .context_menu(Self::build_cell_context_menu())
            .when_else(
              overridden,
              |this| {
                this
                  .justify_between()
                  .child(Label::new(effective_value.clone()).text_color(cx.theme().green))
                  .child(Button::new("btn-revert-var").xsmall().ghost().icon(IconName::Undo).on_click({
                    let table = cx.entity();
                    move |_, _, cx| {
                      cx.stop_propagation();
                      table.update(cx, |table, cx| {
                        table.delegate_mut().revert_override(row_ix, cx);
                      });
                    }
                  }))
              },
              |this| this.child(Label::new(effective_value.clone())),
            )
            .into_any_element()
        }
        3 => h_flex()
          .size_full()
          .child(var.description.clone())
          .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |table, _, window, cx| {
              table.focus_handle(cx).focus(window, cx);
              table.set_selected_cell(row_ix, col_ix, cx);
            }),
          )
          .context_menu(Self::build_cell_context_menu())
          .into_any_element(),
        _ => unreachable!(),
      }
      .into_any_element(),
    }
  }

  fn render_empty(&mut self, _window: &mut Window, _cx: &mut Context<TableState<Self>>) -> impl IntoElement {
    component::empty::Empty::new().header(
      EmptyHeader::new()
        .title(EmptyTitle::new().child("No variable defined"))
        .description(EmptyDescription::new().child("Add a variable by clicking the '+' button")),
    )
  }
}

impl Render for VariableEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let (switch_on, is_editing, is_selection) = self.table_state.read_with(cx, |table, _| {
      (
        table.delegate().viewing_profile.is_some(),
        table.delegate().edited_cell.is_some(),
        table.selected_cell().is_some() || table.selected_row().is_some(),
      )
    });
    let focus_handle = self.table_state.focus_handle(cx);
    v_flex()
      .track_focus(&focus_handle)
      .on_action(cx.listener(Self::on_action_delete))
      .on_action(cx.listener(Self::on_action_duplicate))
      .on_action(cx.listener(Self::on_action_edit))
      .on_action(cx.listener(Self::on_action_new))
      .p_1()
      .size_full()
      .gap_y_2()
      .child(
        h_flex().w_full().justify_between().child(
          h_flex()
            .gap_x_2()
            .h_5()
            .child(
              Switch::new("switch-show-profile")
                .label(SharedString::new("Show for profile"))
                .checked(switch_on)
                .small()
                .on_click({
                  let table_state = self.table_state.clone();
                  move |value, _, cx| {
                    table_state.update(cx, |state, cx| {
                      let delegate = state.delegate_mut();
                      if *value {
                        delegate.viewing_profile = delegate.profile_select_state.read(cx).selected_value().copied();
                      } else {
                        delegate.viewing_profile = None;
                      }
                    })
                  }
                }),
            )
            .when(switch_on, |this| {
              this.child({
                let profile_select_state = &self.table_state.read(cx).delegate().profile_select_state;
                Select::new(profile_select_state).small().min_w_32()
              })
            }),
        ),
      )
      .child(
        h_flex()
          .gap_x_2()
          .w_full()
          .justify_end()
          .child(
            Button::new("btn-add-var")
              .ghost()
              .small()
              .disabled(is_editing)
              .icon(IconName::Plus)
              .tooltip_with_action("Add a new variable", &New, None)
              .on_click({
                let focus_handle = focus_handle.clone();
                move |_, window, cx| {
                  focus_handle.focus(window, cx);
                  window.dispatch_action(Box::new(New), cx)
                }
              }),
          )
          .child(
            Button::new("btn-duplicate-var")
              .ghost()
              .small()
              .disabled(is_editing || !is_selection)
              .icon(IconName::Copy)
              .tooltip_with_action("Duplicate the selected var", &Duplicate, None)
              .on_click({
                let focus_handle = focus_handle.clone();
                move |_, window, cx| {
                  focus_handle.focus(window, cx);
                  window.dispatch_action(Box::new(Duplicate), cx)
                }
              }),
          )
          .child(Separator::vertical())
          .child(
            Button::new("btn-delete-var")
              .ghost()
              .small()
              .disabled(is_editing || !is_selection)
              .icon(IconName::Delete)
              .tooltip_with_action("Delete the selected variable", &Delete, None)
              .on_click({
                move |_, window, cx| {
                  focus_handle.focus(window, cx);
                  window.dispatch_action(Box::new(Delete), cx)
                }
              }),
          ),
      )
      .child(DataTable::new(&self.table_state).small().focus_ring(false))
  }
}
