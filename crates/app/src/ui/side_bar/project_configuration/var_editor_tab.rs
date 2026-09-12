use crate::actions::{Delete, Duplicate, Escape};
use crate::ui::side_bar::project_configuration::ProjectConfigurationTab;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::label::Label;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{ActiveTheme, Disableable, Icon, IconName, IndexPath, Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use ki_assets::icon::IconAsset;
use ki_utils::ui::{CellState, MovingLabel};
use ki_workspace::Project;
use ki_workspace::variable::{ProfileInfo, ProjectVariables, VariableKind, VariableReference};
use log::warn;
use uuid::Uuid;

pub(super) struct VariableEditor {
  table_state: Entity<TableState<VariableDataTableDelegate>>,
}

impl VariableEditor {
  fn on_table_event(
    &mut self,
    table: &Entity<TableState<VariableDataTableDelegate>>,
    event: &TableEvent,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    table.update(cx, |this, cx| match event {
      TableEvent::SelectCell(row_ix, col_ix) => this.delegate_mut().on_cell_selected(*row_ix, *col_ix, window, cx),
      TableEvent::DoubleClickedCell(row_ix, col_ix) => this.delegate_mut().on_cell_edited(*row_ix, *col_ix, window, cx),
      TableEvent::ClearSelection => {
        this.delegate_mut().cell_state = CellState::Unselected;
        this.delegate_mut()._cell_input_sub = None;
      }
      _ => {}
    });
  }

  fn on_delete_row_action(&mut self, _action: &Delete, window: &mut Window, cx: &mut Context<Self>) {
    self.table_state.update(cx, |this, cx| {
      this.delegate_mut().delete_variable(window, cx);
    });
  }

  fn on_duplicate_row_action(&mut self, _action: &Duplicate, window: &mut Window, cx: &mut Context<Self>) {
    self.table_state.update(cx, |table_state, cx| {
      table_state.delegate_mut().duplicate_variable(window, cx);
    });
  }

  fn on_clear_selection(&mut self, _action: &Escape, _window: &mut Window, cx: &mut Context<Self>) {
    self.table_state.update(cx, |table_state, cx| table_state.clear_selection(cx));
  }
}

impl ProjectConfigurationTab for VariableEditor {
  fn name() -> &'static str {
    "Variables"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::Variable
  }

  fn new(project: Entity<Project>, window: &mut Window, cx: &mut App) -> Entity<impl Render> {
    cx.new(|cx| {
      let table_state = cx.new(|cx| {
        TableState::new(VariableDataTableDelegate::new(project, window, cx), window, cx)
          .row_selectable(false)
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
  cell_state: CellState<VariableReference>,
  profile_select_state: Entity<SelectState<Vec<ProfileInfo>>>,
  cell_input_state: Entity<InputState>,
  _cell_input_sub: Option<Subscription>,
  var_kind_select_state: Entity<SelectState<Vec<VariableKind>>>,
  viewing_profile: Option<Uuid>,
}

impl VariableDataTableDelegate {
  fn new(project: Entity<Project>, window: &mut Window, cx: &mut Context<TableState<Self>>) -> Self {
    let table_columns = vec![
      Column::new("var_name", "Name").width(px(150.0)),
      Column::new("var_kind", "Type").width(px(120.0)),
      Column::new("var_value", "Value").width(px(200.0)),
      Column::new("var_description", "Description").width(px(250.0)),
    ];
    let project_vars = project.read(cx).variables.clone();
    let profile_infos = project_vars.read(cx).profile_infos();

    let profile_select_state = cx.new(|cx| SelectState::new(profile_infos, Some(IndexPath::default()), window, cx));
    cx.subscribe_in(
      &profile_select_state,
      window,
      move |table, _, event: &SelectEvent<Vec<ProfileInfo>>, _, _| match event {
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
    cx.subscribe_in(
      &var_kind_select_state,
      window,
      move |table, _, event: &SelectEvent<Vec<VariableKind>>, _, _| match event {
        SelectEvent::Confirm(value) if value.is_some() => {
          if let CellState::CellEdited(_, _, data) = &mut table.delegate_mut().cell_state {
            data.kind = value.unwrap();
          }
        }
        SelectEvent::Confirm(_) => {}
      },
    )
    .detach();

    let cell_input_state = cx.new(|cx| InputState::new(window, cx));

    Self {
      project_vars,
      table_columns,
      cell_state: CellState::Unselected,
      profile_select_state,
      cell_input_state,
      _cell_input_sub: None,
      var_kind_select_state,
      viewing_profile: None,
    }
  }

  fn add_variable(&self, cx: &mut Context<TableState<Self>>) {
    let current_row = match self.cell_state {
      CellState::CellSelected(row_ix) => Some(row_ix),
      CellState::Unselected => None,
      _ => {
        return;
      }
    };
    let name = "new variable";
    self
      .project_vars
      .update(cx, |this, cx| this.add_variable(current_row, name, cx));
  }

  fn delete_variable(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    let row_ix = match self.cell_state {
      CellState::CellSelected(row_ix) => row_ix,
      _ => {
        return;
      }
    };
    if let Err(err) = self.project_vars.update(cx, |this, cx| this.delete_variable(row_ix, cx)) {
      window.push_notification(err, cx);
    }
    cx.notify();
  }

  fn duplicate_variable(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    let row_ix = match self.cell_state {
      CellState::CellSelected(row_ix) => row_ix,
      _ => {
        return;
      }
    };
    if let Err(err) = self.project_vars.update(cx, |this, cx| this.duplicate_variable(row_ix, cx)) {
      window.push_notification(err, cx);
    }
    cx.notify();
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

  fn on_cell_selected(&mut self, row_ix: usize, col_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    match self.cell_state {
      // Same cell being edited, return
      CellState::CellEdited(edit_row_ix, edit_col_ix, _) if edit_row_ix == row_ix && edit_col_ix == col_ix => {
        return;
      }
      // Other cell being edited, save before
      CellState::CellEdited(_, _, _) => {
        self.update_variable(window, cx);
        self._cell_input_sub = None;
      }
      _ => {}
    }
    self.cell_state = CellState::CellSelected(row_ix);
  }

  fn on_cell_edited(&mut self, row_ix: usize, col_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let CellState::CellEdited(current_row_ix, current_col_ix, _) = self.cell_state {
      if current_row_ix == row_ix && current_col_ix == col_ix {
        return;
      }
      self.update_variable(window, cx);
      self._cell_input_sub = None;
    }

    let mut var = self.project_vars.read(cx).references.get(row_ix).unwrap().clone();
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
        let (overridden, effective_value) = self.effective_value(&var);
        if overridden {
          var.value = effective_value;
        }
        self.cell_input_state.update(cx, |state, cx| {
          state.set_value(var.value.clone(), window, cx);
          state.focus(window, cx);
        });
        self._cell_input_sub = Some(cx.subscribe_in(
          &self.cell_input_state,
          window,
          move |table, input_state, event, window, cx| {
            Self::on_cell_input_event(table, input_state, event, window, cx, |var, value| var.value = value)
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

    self.cell_state = CellState::CellEdited(row_ix, col_ix, var);
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
        if let CellState::CellEdited(_, _, data) = &mut table.delegate_mut().cell_state {
          let text = state.read(cx).value();
          f(data, text);
        }
      }
      InputEvent::PressEnter { secondary: _, shift: _ } | InputEvent::Blur => {
        table.delegate_mut().end_edit_cell(window, cx);
        table.focus_handle(cx).focus(window, cx);
      }
      _ => {}
    }
  }

  fn end_edit_cell(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let CellState::CellEdited(row, _, _) = self.cell_state {
      self.update_variable(window, cx);
      self._cell_input_sub = None;
      self.cell_state = CellState::CellSelected(row);
    }
  }

  fn update_variable(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    let profile = self.viewing_profile;
    if let CellState::CellEdited(_, _, data) = &self.cell_state
      && let Err(err) = self
        .project_vars
        .update(cx, |this, cx| this.update_variable(profile, data, cx))
    {
      window.push_notification(err, cx);
    }
  }

  fn move_variable(&mut self, from: usize, to: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
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
    let is_editing = matches!(self.cell_state, CellState::CellEdited(_, _, _));
    div().id(("row", row_ix)).when(!is_editing, |this| {
      this
        .on_drag(
          MovingLabel {
            label: self.project_vars.read(cx).references.get(row_ix).unwrap().name.clone(),
            data: row_ix,
          },
          |drag, _, _, cx| {
            cx.stop_propagation();
            cx.new(|_| drag.clone())
          },
        )
        .on_drop(cx.listener(move |table, e: &MovingLabel<usize>, window, cx| {
          table.delegate_mut().move_variable(e.data, row_ix, window, cx);
        }))
    })
  }

  fn render_td(
    &mut self,
    row_ix: usize,
    col_ix: usize,
    _window: &mut Window,
    cx: &mut Context<TableState<Self>>,
  ) -> impl IntoElement {
    let var = &self.project_vars.read(cx).references[row_ix];
    match &self.cell_state {
      CellState::CellEdited(row, col, _) if *row == row_ix && *col == col_ix => match col_ix {
        0 | 2 | 3 => Input::new(&self.cell_input_state).small().into_any_element(),
        1 => Select::new(&self.var_kind_select_state).small().into_any_element(),
        _ => unreachable!(),
      },
      _ => match col_ix {
        0 => h_flex().size_full().child(var.name.clone()),
        1 => h_flex().size_full().child(SharedString::new(format!("{}", var.kind))),
        2 => {
          let (overridden, effective_value) = self.effective_value(var);
          h_flex().size_full().when_else(
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
        }
        3 => h_flex().size_full().child(var.description.clone()),
        _ => unreachable!(),
      }
      .into_any_element(),
    }
  }

  fn render_empty(&mut self, _window: &mut Window, _cx: &mut Context<TableState<Self>>) -> impl IntoElement {
    div().size_full().items_center().child("No Data").into_any_element()
  }
}

impl Render for VariableEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let table_state = self.table_state.clone();
    let switch_on = table_state.read(cx).delegate().viewing_profile;
    let profile_select_state = &table_state.read(cx).delegate().profile_select_state;
    let (is_editing, no_selection) = self.table_state.read_with(cx, |this, _| {
      let delegate = this.delegate();
      (
        matches!(delegate.cell_state, CellState::CellEdited(_, _, _)),
        matches!(delegate.cell_state, CellState::Unselected),
      )
    });

    v_flex()
      .track_focus(&table_state.focus_handle(cx))
      .on_action(cx.listener(Self::on_delete_row_action))
      .on_action(cx.listener(Self::on_duplicate_row_action))
      .on_action(cx.listener(Self::on_clear_selection))
      .p_1()
      .size_full()
      .gap_y_2()
      .child(
        h_flex()
          .w_full()
          .justify_between()
          .child(
            h_flex()
              .gap_x_2()
              .child(
                Switch::new("switch-show-profile")
                  .label(SharedString::new("Show for profile"))
                  .checked(switch_on.is_some())
                  .small()
                  .on_click({
                    let table_state = table_state.clone();
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
              .when(switch_on.is_some(), |this| {
                this.child(Select::new(profile_select_state).small().min_w_32())
              }),
          )
          .child(
            h_flex()
              .gap_x_2()
              .flex_row_reverse()
              .child(
                Button::new("btn-delete-variable")
                  .ghost()
                  .small()
                  .disabled(is_editing || no_selection)
                  .icon(IconName::Delete)
                  .tooltip_with_action("Delete the selected variable", &Delete, None)
                  .on_click(move |_, window, cx| {
                    window.dispatch_action(Box::new(Delete), cx);
                  }),
              )
              .child(Separator::vertical())
              .child(
                Button::new("btn-duplicate-variable")
                  .ghost()
                  .small()
                  .disabled(is_editing || no_selection)
                  .icon(IconName::Copy)
                  .tooltip_with_action("Duplicate the selected variable", &Duplicate, None)
                  .on_click({
                    let table_state = table_state.clone();
                    move |_, _, cx| {
                      table_state.update(cx, |this, cx| {
                        this.delegate_mut().add_variable(cx);
                      });
                    }
                  }),
              )
              .child(
                Button::new("btn-add-variable")
                  .ghost()
                  .small()
                  .disabled(is_editing)
                  .icon(IconName::Plus)
                  .tooltip("Adds a new variable")
                  .on_click({
                    let table_state = table_state.clone();
                    move |_, _, cx| {
                      table_state.update(cx, |this, cx| {
                        this.delegate_mut().add_variable(cx);
                      });
                    }
                  }),
              ),
          ),
      )
      .child(DataTable::new(&self.table_state).small())
  }
}
