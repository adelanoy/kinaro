use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::empty::{EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{Disableable, FocusableExt, Icon, IconName, Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::*;
use uuid::Uuid;
use ki_app_shared::actions::{Delete, Duplicate, Edit, New};
use ki_app_shared::{ContextMenuBuilder, EditedCell, MovingLabel};
use ki_app_shared::panel::PanelDescriptor;
use ki_assets::icon::IconAsset;
use ki_utils::SidebarPanel;
use ki_workspace::Project;
use ki_workspace::variable::{DraggedProfile, Profile, ProjectVariables};

pub(in crate::ui::side_bar) struct ProfileEditor {
  table_state: Entity<TableState<ProfileDataTableDelegate>>,
  _table_subscriptions: Subscription,
}

impl ProfileEditor {
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
            .title("Delete Profile")
            .description("Are you sure you want to delete the profile?")
            .show_cancel(true)
            .on_ok({
              let this = table_state.clone();
              move |_, window, cx| {
                this.update(cx, |this, cx| {
                  this.delegate_mut().delete_profile(row_ix, window, cx);
                  this.clear_selection(cx);
                });
                true
              }
            })
        }
      });
    }
  }

  fn on_action_duplicate(&mut self, _action: &Duplicate, _window: &mut Window, cx: &mut Context<Self>) {
    if let Some(row_ix) = self
      .table_state
      .read(cx)
      .selected_row()
      .or_else(|| self.table_state.read(cx).selected_cell().map(|(row, _)| row))
    {
      self.table_state.update(cx, |this, cx| {
        this.delegate_mut().duplicate_profile(row_ix, cx);
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

  fn on_action_new(&mut self, _action: &New, _window: &mut Window, cx: &mut Context<Self>) {
    let row_ix = self
      .table_state
      .read(cx)
      .selected_row()
      .or_else(|| self.table_state.read(cx).selected_cell().map(|(row, _)| row));
    self.table_state.update(cx, |this, cx| {
      this.delegate_mut().add_profile(row_ix, cx);
    });
  }

  fn on_table_event(
    &mut self,
    table: &Entity<TableState<ProfileDataTableDelegate>>,
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

impl PanelDescriptor<SidebarPanel, Project> for ProfileEditor {
  fn name() -> &'static str {
    "Profiles"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::Profile
  }

  fn panel_type() -> SidebarPanel {
    SidebarPanel::Profile
  }

  fn new(project: Entity<Project>, window: &mut Window, cx: &mut App) -> Entity<Self> {
    cx.new(|cx| {
      let table_state = cx.new(|cx| {
        TableState::new(ProfileDataTableDelegate::new(project, window, cx), window, cx)
          .row_selectable(true)
          .col_selectable(false)
          .cell_selectable(true)
      });
      let _table_subscriptions = cx.subscribe_in(&table_state, window, Self::on_table_event);

      Self {
        table_state,
        _table_subscriptions,
      }
    })
  }
}

pub struct ProfileDataTableDelegate {
  project_vars: Entity<ProjectVariables>,
  table_columns: Vec<Column>,
  edited_cell: Option<EditedCell<Profile>>,
  cell_input_state: Entity<InputState>,
  _cell_input_sub: Option<Subscription>,
}

impl ProfileDataTableDelegate {
  fn add_profile(&mut self, current_row: Option<usize>, cx: &mut Context<TableState<Self>>) {
    self.project_vars.update(cx, |this, cx| this.add_profile(current_row, cx));
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
      let profile = self.project_vars.read(cx).profiles[row_ix].clone();
      match col_ix {
        0 => {
          self.cell_input_state.update(cx, |state, cx| {
            let name = profile.name.clone();
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
            let description = profile.description.clone();
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

      self.edited_cell = Some(EditedCell {
        row: row_ix,
        col: col_ix,
        data: profile,
      });
    }
  }

  fn clear_selection(&mut self) {
    self.edited_cell = None;
    self._cell_input_sub = None;
  }

  fn delete_profile(&mut self, row_ix: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
    if let Err(err) = self.project_vars.update(cx, |this, cx| this.delete_profile(row_ix, cx)) {
      window.push_notification(err, cx);
    }
    cx.notify();
  }

  fn duplicate_profile(&mut self, row_ix: usize, cx: &mut Context<TableState<Self>>) {
    self.project_vars.update(cx, |this, cx| this.duplicate_profile(row_ix, cx));
    cx.notify();
  }

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
      edited_cell: None,
      cell_input_state,
      _cell_input_sub: None,
    }
  }

  fn on_cell_input_event<F>(
    table: &mut TableState<Self>,
    input: &Entity<InputState>,
    event: &InputEvent,
    window: &mut Window,
    cx: &mut Context<TableState<Self>>,
    f: F,
  ) where
    F: FnOnce(&mut Profile, SharedString),
  {
    match event {
      InputEvent::Change => {
        if let Some(edited) = &mut table.delegate_mut().edited_cell {
          let text = input.read(cx).value();
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
      if let Err(err) = self.project_vars.update(cx, |vars, cx| vars.update_profile(edited.data, cx)) {
        window.push_notification(err, cx);
      }
      self._cell_input_sub = None;
      if self.cell_input_state.read(cx).focus_handle(cx).is_focused(window) {
        cx.focus_self(window);
      }
      cx.notify();
    }
  }

  fn move_profile(&mut self, from: Uuid, to: usize, window: &mut Window, cx: &mut Context<TableState<Self>>) {
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
    let is_editing = self.edited_cell.is_some();
    let id = self.project_vars.read(cx).profiles.get(row_ix).unwrap().id;
    div().id(("row", row_ix)).when(!is_editing, |this| {
      this
        .on_drag(
          MovingLabel {
            label: self.project_vars.read(cx).profiles.get(row_ix).unwrap().name.clone(),
            data: DraggedProfile(id),
          },
          |drag, _, _, cx| {
            cx.stop_propagation();
            cx.new(|_| drag.clone())
          },
        )
        .on_drop(cx.listener(move |table, e: &MovingLabel<DraggedProfile>, window, cx| {
          table.delegate_mut().move_profile(e.data.0, row_ix, window, cx);
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
    match &self.edited_cell {
      Some(edited) if edited.is_at(row_ix, col_ix) => match col_ix {
        0 | 1 => Input::new(&self.cell_input_state).small(),
        _ => unreachable!(),
      }
      .into_any_element(),
      _ => {
        let profile = self.project_vars.read(cx).profiles.get(row_ix).unwrap();
        match col_ix {
          0 => h_flex()
            .size_full()
            .child(profile.name.clone())
            // Force selection on right click, or the .context_menu below will consume the event, hence the table never receives the rightClick event
            .on_mouse_down(
              MouseButton::Right,
              cx.listener(move |table, _, window, cx| {
                table.focus_handle(cx).focus(window, cx);
                table.set_selected_cell(row_ix, col_ix, cx)
              }),
            )
            .context_menu(Self::build_cell_context_menu())
            .into_any_element(),
          1 => h_flex()
            .size_full()
            .child(profile.description.clone())
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
      }
    }
  }

  fn render_empty(&mut self, _window: &mut Window, _cx: &mut Context<TableState<Self>>) -> impl IntoElement {
    component::empty::Empty::new().header(
      EmptyHeader::new()
        .title(EmptyTitle::new().child("No profile defined"))
        .description(EmptyDescription::new().child("Add a profile by clicking the '+' button")),
    )
  }
}

impl Render for ProfileEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let (is_editing, is_selection) = self.table_state.read_with(cx, |table, _| {
      (
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
        h_flex()
          .gap_x_2()
          .w_full()
          .justify_end()
          .child(
            Button::new("btn-add-profile")
              .ghost()
              .small()
              .disabled(is_editing)
              .icon(IconName::Plus)
              .tooltip_with_action("Adds a new profile", &New, None)
              .on_click({
                let focus_handle = focus_handle.clone();
                move |_, window, cx| {
                  focus_handle.focus(window, cx);
                  window.dispatch_action(Box::new(New), cx)
                }
              }),
          )
          .child(
            Button::new("btn-duplicate-profile")
              .ghost()
              .small()
              .disabled(is_editing || !is_selection)
              .icon(IconName::Copy)
              .tooltip_with_action("Duplicate the selected profile", &Duplicate, None)
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
            Button::new("btn-delete-profile")
              .ghost()
              .small()
              .disabled(is_editing || !is_selection)
              .icon(IconName::Delete)
              .tooltip_with_action("Delete the selected profile", &Delete, None)
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
