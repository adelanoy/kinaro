use crate::actions::{Delete, Duplicate, Escape};
use crate::ui::side_bar::project_configuration::ProjectConfigurationTab;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::label::Label;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{Disableable, Icon, IconName, Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::*;
use ki_assets::icon::IconAsset;
use ki_utils::ui::{CellState, MovingLabel};
use ki_workspace::Project;
use ki_workspace::variable::{ProfileInfo, ProjectVariables};
use log::warn;

pub(super) struct ProfileEditor {
  focus_handle: FocusHandle,
  table_state: Entity<TableState<ProfileDataTableDelegate>>,
  _table_subscriptions: Subscription,
}

impl ProfileEditor {
  fn on_table_event(
    &mut self,
    table: &Entity<TableState<ProfileDataTableDelegate>>,
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

  fn on_delete_action(&mut self, _action: &Delete, window: &mut Window, cx: &mut Context<Self>) {
    self.table_state.update(cx, |this, cx| {
      this.delegate_mut().show_delete_profile_confirm_dialog(window, cx);
    });
  }

  fn on_duplicate_action(&mut self, _action: &Duplicate, window: &mut Window, cx: &mut Context<Self>) {
    self.table_state.update(cx, |table_state, cx| {
      table_state.delegate_mut().duplicate_profile(window, cx);
    });
  }

  fn on_escape_action(&mut self, _action: &Escape, _window: &mut Window, cx: &mut Context<Self>) {
    self.table_state.update(cx, |table_state, cx| table_state.clear_selection(cx));
  }
}

impl ProjectConfigurationTab for ProfileEditor {
  fn name() -> &'static str {
    "Profiles"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::Profile
  }

  fn new(project: Entity<Project>, window: &mut Window, cx: &mut App) -> Entity<impl Render> {
    cx.new(|cx| {
      let table_state = cx.new(|cx| {
        TableState::new(ProfileDataTableDelegate::new(project, window, cx), window, cx)
          .row_selectable(false)
          .col_selectable(false)
          .cell_selectable(true)
      });
      let _table_subscriptions = cx.subscribe_in(&table_state, window, Self::on_table_event);

      Self {
        focus_handle: cx.focus_handle(),
        table_state,
        _table_subscriptions,
      }
    })
  }
}

pub struct ProfileDataTableDelegate {
  project_vars: Entity<ProjectVariables>,
  table_columns: Vec<Column>,
  cell_state: CellState<ProfileInfo>,
  cell_input_state: Entity<InputState>,
  _cell_input_sub: Option<Subscription>,
}

impl ProfileDataTableDelegate {
  fn new(project: Entity<Project>, window: &mut Window, cx: &mut Context<TableState<Self>>) -> Self {
    let table_columns = vec![
      Column::new("profile_short_name", "Name").width(px(180.0)).resizable(true),
      Column::new("profile_description", "Description")
        .width(px(300.0))
        .resizable(false),
    ];

    let cell_input_state = cx.new(|cx| InputState::new(window, cx));
    let project_vars = project.read(cx).variables.clone();

    Self {
      project_vars,
      table_columns,
      cell_state: CellState::Unselected,
      cell_input_state,
      _cell_input_sub: None,
    }
  }

  fn on_cell_selected(&mut self, row_ix: usize, col_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    match self.cell_state {
      // Same cell being edited, return
      CellState::CellEdited(edit_row_ix, edit_col_ix, _) if edit_row_ix == row_ix && edit_col_ix == col_ix => {
        return;
      }
      // Other cell being edited, save before
      CellState::CellEdited(_, _, _) => {
        self.update_profile(window, cx);
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
      self.update_profile(window, cx);
      self._cell_input_sub = None;
    }

    let profile = self.project_vars.read(cx).profiles.get(row_ix).unwrap();
    let cell_data = ProfileInfo {
      id: profile.id,
      name: profile.name.clone(),
      description: profile.description.clone(),
    };
    match col_ix {
      0 => {
        self.cell_input_state.update(cx, |state, cx| {
          let name = cell_data.name.clone();
          state.set_value(name, window, cx);
          state.focus(window, cx);
        });
        self._cell_input_sub = Some(cx.subscribe_in(
          &self.cell_input_state,
          window,
          move |table, input_state, event, window, cx| {
            Self::on_cell_input_event(table, input_state, event, window, cx, |profile, value| profile.name = value)
          },
        ));
      }
      1 => {
        self.cell_input_state.update(cx, |state, cx| {
          let description = cell_data.description.clone();
          state.set_value(description, window, cx);
          state.focus(window, cx);
        });
        self._cell_input_sub = Some(cx.subscribe_in(
          &self.cell_input_state,
          window,
          move |table, input_state, event, window, cx| {
            Self::on_cell_input_event(table, input_state, event, window, cx, |profile, value| {
              profile.description = value
            })
          },
        ));
      }
      _ => unreachable!(),
    }

    self.cell_state = CellState::CellEdited(row_ix, col_ix, cell_data);
  }

  fn on_cell_input_event<F>(
    table: &mut TableState<Self>,
    input: &Entity<InputState>,
    event: &InputEvent,
    window: &mut Window,
    cx: &mut Context<TableState<Self>>,
    f: F,
  ) where
    F: FnOnce(&mut ProfileInfo, SharedString),
  {
    match event {
      InputEvent::Change => {
        let cell_state = &mut table.delegate_mut().cell_state;
        if let CellState::CellEdited(_, _, data) = cell_state {
          let text = input.read(cx).value();
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
      self.update_profile(window, cx);
      self._cell_input_sub = None;
      self.cell_state = CellState::CellSelected(row);
    }
  }

  fn update_profile(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let CellState::CellEdited(_, _, data) = &self.cell_state
      && let Err(err) = self.project_vars.update(cx, |this, cx| this.update_profile(data, cx))
    {
      window.push_notification(err, cx);
    }
  }

  fn add_profile(&mut self, cx: &mut Context<TableState<Self>>) {
    let current_row = match self.cell_state {
      CellState::CellSelected(row_ix) => Some(row_ix),
      CellState::Unselected => None,
      _ => {
        return;
      }
    };
    let name = "new profile";
    self
      .project_vars
      .update(cx, |this, cx| this.add_profile(current_row, name, cx));
  }

  fn show_delete_profile_confirm_dialog(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    let row_ix = match self.cell_state {
      CellState::CellSelected(row_ix) => row_ix,
      _ => {
        warn!("ProfileDataTableDelegate:show_delete_profile_confirm_dialog: no cell selected");
        return;
      }
    };
    let Some(name) = self.project_vars.read(cx).profiles.get(row_ix).map(|p| p.name.clone()) else {
      warn!("ProfileDataTableDelegate:show_delete_profile_confirm_dialog: invalid selection");
      return;
    };

    let this = cx.entity();
    window.open_alert_dialog(cx, {
      move |dialog, _, _| {
        dialog
          .title("Delete profile")
          .description(format!("You are about to delete the profile '{}', continue?", name))
          .show_cancel(true)
          .on_ok({
            let this = this.clone();
            move |_, window, cx| {
              this.update(cx, |this, cx| {
                this.delegate_mut().delete_profile(row_ix, window, cx);
              });
              true
            }
          })
      }
    });
  }

  fn delete_profile(&mut self, row_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let Err(err) = self.project_vars.update(cx, |this, cx| this.delete_profile(row_ix, cx)) {
      window.push_notification(err, cx);
    }
    cx.notify();
  }

  fn duplicate_profile(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    let row_ix = match self.cell_state {
      CellState::CellSelected(row_ix) => row_ix,
      _ => {
        return;
      }
    };
    if let Err(err) = self.project_vars.update(cx, |this, cx| this.duplicate_profile(row_ix, cx)) {
      window.push_notification(err, cx);
    }
    cx.notify();
  }

  fn move_profile(&mut self, from: usize, to: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let Err(err) = self.project_vars.update(cx, |this, cx| this.move_profile(from, to, cx)) {
      window.push_notification(err, cx);
    }
  }
}

impl TableDelegate for ProfileDataTableDelegate {
  fn columns_count(&self, _cx: &App) -> usize {
    self.table_columns.len()
  }

  fn rows_count(&self, cx: &App) -> usize {
    self.project_vars.read(cx).profiles.len()
  }

  fn column(&self, col_ix: usize, _: &App) -> Column {
    self.table_columns.get(col_ix).unwrap().clone()
  }

  fn render_tr(&mut self, row_ix: usize, _window: &mut Window, cx: &mut Context<TableState<Self>>) -> Stateful<Div> {
    let is_editing = matches!(self.cell_state, CellState::CellEdited(_, _, _));
    div().id(("row", row_ix)).when(!is_editing, |this| {
      this
        .on_drag(
          MovingLabel {
            label: self.project_vars.read(cx).profiles.get(row_ix).unwrap().name.clone(),
            data: row_ix,
          },
          |drag, _, _, cx| {
            cx.stop_propagation();
            cx.new(|_| drag.clone())
          },
        )
        .on_drop(cx.listener(move |table, e: &MovingLabel<usize>, window, cx| {
          table.delegate_mut().move_profile(e.data, row_ix, window, cx);
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
    match &self.cell_state {
      CellState::CellEdited(row, col, _) if *row == row_ix && *col == col_ix => match col_ix {
        0 | 1 => Input::new(&self.cell_input_state).small(),
        _ => unreachable!(),
      }
      .into_any_element(),
      _ => {
        let profile = self.project_vars.read(cx).profiles.get(row_ix).unwrap();
        let value = match col_ix {
          0 => profile.name.clone(),
          1 => profile.description.clone(),
          _ => unreachable!(),
        };
        h_flex().size_full().child(Label::new(value)).into_any_element()
      }
    }
  }

  fn render_empty(&mut self, _window: &mut Window, _cx: &mut Context<TableState<Self>>) -> impl IntoElement {
    div().size_full().items_center().child("No Data").into_any_element()
  }
}

impl Render for ProfileEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let (is_editing, no_selection) = self.table_state.read_with(cx, |this, _| {
      let delegate = this.delegate();
      (
        matches!(delegate.cell_state, CellState::CellEdited(_, _, _)),
        matches!(delegate.cell_state, CellState::Unselected),
      )
    });
    v_flex()
      .track_focus(&self.focus_handle)
      .on_action(cx.listener(Self::on_delete_action))
      .on_action(cx.listener(Self::on_duplicate_action))
      .on_action(cx.listener(Self::on_escape_action))
      .p_1()
      .size_full()
      .gap_y_2()
      .child(
        h_flex()
          .gap_x_2()
          .w_full()
          .flex_row_reverse()
          .child(
            Button::new("btn-delete-profile")
              .ghost()
              .small()
              .disabled(is_editing || no_selection)
              .icon(IconName::Delete)
              .tooltip_with_action("Delete the selected profile", &Delete, None)
              .on_click(move |_, window, cx| {
                window.dispatch_action(Box::new(Delete), cx);
              }),
          )
          .child(Separator::vertical())
          .child(
            Button::new("btn-duplicate-profile")
              .ghost()
              .small()
              .disabled(is_editing || no_selection)
              .icon(IconName::Copy)
              .tooltip_with_action("Duplicate the selected profile", &Duplicate, None)
              .on_click(move |_, window, cx| {
                window.dispatch_action(Box::new(Duplicate), cx);
              }),
          )
          .child(
            Button::new("btn-add-profile")
              .ghost()
              .small()
              .disabled(is_editing)
              .icon(IconName::Plus)
              .tooltip("Adds a new profile")
              .on_click({
                let table_state = self.table_state.clone();
                move |_, _, cx| {
                  table_state.update(cx, |this, cx| {
                    this.delegate_mut().add_profile(cx);
                  });
                }
              }),
          ),
      )
      .child(DataTable::new(&self.table_state).small())
  }
}
