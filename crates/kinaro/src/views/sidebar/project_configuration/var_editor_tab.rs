use crate::ui_utils::CellState;
use crate::views::sidebar::project_configuration::ProjectConfigurationTab;
use crate::workspace::Project;
use crate::workspace::variable::{ProfileInfo, ProjectVariables, VariableReference};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::label::Label;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::switch::Switch;
use gpui_component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, IndexPath, Sizable, WindowExt, h_flex, v_flex,
};
use uuid::Uuid;
use ki_assets::icon::IconAsset;
use ki_project::VariableKind;

pub(super) struct VariableEditor {
    focus_handle: FocusHandle,
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
            TableEvent::SelectCell(row_ix, col_ix) => {
                if *col_ix == 0 {
                    this.clear_selection(cx);
                    return;
                }
                this.delegate_mut()
                    .on_cell_selected(*row_ix, *col_ix, window, cx)
            }
            TableEvent::DoubleClickedCell(row_ix, col_ix) => {
                if *col_ix == 0 {
                    this.clear_selection(cx);
                    return;
                }
                this.delegate_mut()
                    .on_cell_edited(*row_ix, *col_ix, window, cx)
            }
            TableEvent::SelectRow(ix) => {
                this.delegate_mut().on_row_selected(*ix, window, cx);
            }
            TableEvent::ClearSelection => {
                this.delegate_mut().cell_state = CellState::Unselected;
                this.delegate_mut()._cell_input_sub = None;
            }
            _ => {}
        });
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
                TableState::new(
                    VariableDataTableDelegate::new(project, window, cx),
                    window,
                    cx,
                )
                .row_selectable(false)
                .col_selectable(false)
                .cell_selectable(true)
            });
            cx.subscribe_in(&table_state, window, Self::on_table_event)
                .detach();

            Self {
                focus_handle: cx.focus_handle(),
                table_state,
            }
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
    switch_on: bool,
}

impl VariableDataTableDelegate {
    fn new(
        project: Entity<Project>,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> Self {
        let table_columns = vec![
            Column::new("var_action", "action")
                .width(px(75.0))
                .fixed_left(),
            Column::new("var_name", "Name").width(px(150.0)),
            Column::new("var_kind", "Type").width(px(120.0)),
            Column::new("var_value", "Value").width(px(200.0)),
            Column::new("var_description", "Description").width(px(250.0)),
        ];
        let project_vars = project.read(cx).variables.clone();
        let profile_infos = project_vars
            .read(cx)
            .profiles
            .iter()
            .map(|p| ProfileInfo {
                id: p.id,
                name: p.name.clone(),
                description: p.description.clone(),
            })
            .collect();

        let profile_select_state =
            cx.new(|cx| SelectState::new(profile_infos, Some(IndexPath::default()), window, cx));
        cx.subscribe_in(
            &profile_select_state,
            window,
            |_, _, e: &SelectEvent<Vec<ProfileInfo>>, _, _| match e {
                SelectEvent::Confirm(value) => {
                    if let Some(selected_value) = value {
                        println!("Selected: {:?}", selected_value);
                    } else {
                        println!("Selection cleared");
                    }
                }
            },
        )
        .detach();

        let cell_input_state = cx.new(|cx| InputState::new(window, cx));
        let var_kind_select_state = cx.new(|cx| {
            SelectState::new(
                vec![
                    VariableKind::Text,
                    VariableKind::PasswordClear,
                    VariableKind::PasswordEncrypt,
                ],
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
                    match &mut table.delegate_mut().cell_state {
                        CellState::CellEdited(_, _, data) => {
                            data.kind = value.unwrap();
                        }
                        _ => {}
                    }
                }
                SelectEvent::Confirm(_) => {}
            },
        )
        .detach();

        Self {
            project_vars,
            table_columns,
            cell_state: CellState::Unselected,
            profile_select_state,
            cell_input_state,
            _cell_input_sub: None,
            var_kind_select_state,
            switch_on: false,
        }
    }

    fn add_variable(&self, cx: &mut Context<TableState<Self>>) {
        let current_row = match self.cell_state {
            CellState::RowSelected(row_ix) | CellState::CellSelected(row_ix) => Some(row_ix),
            CellState::Unselected => None,
            _ => {
                return;
            }
        };
        let name = "new variable";
        self.project_vars
            .update(cx, |this, cx| this.add_variable(current_row, name, cx));
    }

    fn delete_variable(
        &mut self,
        row_ix: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> bool {
        let result = self
            .project_vars
            .update(cx, |this, cx| this.delete_variable(row_ix, cx));
        match result {
            Ok(_) => true,
            Err(err) => {
                window.push_notification(err, cx);
                false
            }
        }
    }

    fn delete_selected_variable(
        &mut self,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        let current_row = match self.cell_state {
            CellState::RowSelected(row_ix) | CellState::CellSelected(row_ix) => row_ix,
            _ => {
                return;
            }
        };

        if self.delete_variable(current_row, window, cx) {
            if current_row >= self.project_vars.read(cx).references.len() {
                self.cell_state = CellState::Unselected
            }
        }
    }

    fn duplicate_profile(
        &mut self,
        row_ix: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        if let Err(err) = self
            .project_vars
            .update(cx, |this, cx| this.duplicate_variable(row_ix, cx))
        {
            window.push_notification(err, cx);
        }
    }

    fn revert_profile(
        &mut self,
        row_ix: usize,
        profile_id: Uuid,
        cx: &mut Context<TableState<Self>>,
    ) {
        let var_id = self.project_vars.read(cx).references[row_ix].id;
        self
            .project_vars
            .update(cx, |this, cx| this.revert_profile(var_id, profile_id, cx));
    }

    fn on_cell_selected(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        match self.cell_state {
            // Same cell being edited, return
            CellState::CellEdited(edit_row_ix, edit_col_ix, _)
                if edit_row_ix == row_ix && edit_col_ix == col_ix =>
            {
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

    fn on_row_selected(
        &mut self,
        row_ix: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        if matches!(self.cell_state, CellState::CellEdited(_, _, _)) {
            self.update_variable(window, cx);
            self._cell_input_sub = None;
        }
        self.cell_state = CellState::RowSelected(row_ix);
    }

    fn on_cell_edited(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        match self.cell_state {
            CellState::CellEdited(current_row_ix, current_col_ix, _) => {
                if current_row_ix == row_ix && current_col_ix == col_ix {
                    return;
                } else {
                    self.update_variable(window, cx);
                    self._cell_input_sub = None;
                }
            }
            _ => {}
        }

        let var = self
            .project_vars
            .read(cx)
            .references
            .get(row_ix)
            .unwrap()
            .clone();
        match col_ix {
            1 => {
                self.cell_input_state.update(cx, |state, cx| {
                    state.set_value(var.name.clone(), window, cx);
                    state.focus(window, cx);
                });
                self._cell_input_sub = Some(cx.subscribe_in(
                    &self.cell_input_state,
                    window,
                    move |table, input_state, event, window, cx| {
                        Self::on_cell_input_event(
                            table,
                            input_state,
                            event,
                            window,
                            cx,
                            |var, value| var.name = value,
                        )
                    },
                ));
            }
            2 => {
                self.var_kind_select_state.update(cx, |state, cx| {
                    state.set_selected_value(&var.kind, window, cx);
                    state.focus(window, cx);
                });
            }
            3 => {
                self.cell_input_state.update(cx, |state, cx| {
                    state.set_value(var.value.clone(), window, cx);
                    state.focus(window, cx);
                });
                self._cell_input_sub = Some(cx.subscribe_in(
                    &self.cell_input_state,
                    window,
                    move |table, input_state, event, window, cx| {
                        Self::on_cell_input_event(
                            table,
                            input_state,
                            event,
                            window,
                            cx,
                            |var, value| var.value = value,
                        )
                    },
                ));
            }
            4 => {
                self.cell_input_state.update(cx, |state, cx| {
                    state.set_value(var.description.clone(), window, cx);
                    state.focus(window, cx);
                });
                self._cell_input_sub = Some(cx.subscribe_in(
                    &self.cell_input_state,
                    window,
                    move |table, input_state, event, window, cx| {
                        Self::on_cell_input_event(
                            table,
                            input_state,
                            event,
                            window,
                            cx,
                            |var, value| var.description = value,
                        )
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
            InputEvent::Change => match &mut table.delegate_mut().cell_state {
                CellState::CellEdited(_, _, data) => {
                    let text = state.read(cx).value();
                    f(data, text);
                }
                _ => {}
            },
            InputEvent::PressEnter {
                secondary: _,
                shift: _,
            }
            | InputEvent::Blur => table.delegate_mut().end_edit_cell(window, cx),
            _ => {}
        }
    }

    fn end_edit_cell(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
        match self.cell_state {
            CellState::CellEdited(row, _, _) => {
                self.update_variable(window, cx);
                self._cell_input_sub = None;
                self.cell_state = CellState::CellSelected(row);
            }
            _ => {}
        }
    }

    fn update_variable(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
        let profile = self
            .switch_on
            .then(|| self.profile_select_state.read(cx).selected_value().copied())
            .unwrap_or_default();
        match &self.cell_state {
            CellState::CellEdited(_, _, data) => {
                if let Err(err) = self
                    .project_vars
                    .update(cx, |this, cx| this.update_variable(profile, &data, cx))
                {
                    window.push_notification(err, cx);
                }
            }
            _ => {}
        }
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

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let profile_id = if self.switch_on {
            self.profile_select_state.read(cx).selected_value().copied()
        } else {
            None
        };
        if col_ix == 0 {
            return h_flex()
                .gap_x_2()
                .items_center()
                .child(
                    Button::new("btn-delete-var")
                        .xsmall()
                        .ghost()
                        .icon(IconName::Delete)
                        .on_click({
                            let table = cx.entity();
                            move |_, window, cx| {
                                cx.stop_propagation();
                                table.update(cx, |table, cx| {
                                    table.delegate_mut().delete_variable(row_ix, window, cx);
                                });
                            }
                        }),
                )
                .child(
                    Button::new("btn-duplicate-var")
                        .xsmall()
                        .ghost()
                        .icon(IconName::Copy)
                        .on_click({
                            let table = cx.entity();
                            move |_, window, cx| {
                                cx.stop_propagation();
                                table.update(cx, |table, cx| {
                                    table.delegate_mut().duplicate_profile(row_ix, window, cx);
                                });
                            }
                        }),
                )
                .when_some(profile_id, |this, profile_id| {
                    this.child(
                        Button::new("btn-revert-var")
                            .xsmall()
                            .ghost()
                            .icon(IconName::Undo)
                            .on_click({
                                let table = cx.entity();
                                move |_, _, cx| {
                                    cx.stop_propagation();
                                    table.update(cx, |table, cx| {
                                        table.delegate_mut().revert_profile(row_ix, profile_id, cx);
                                    });
                                }
                            }),
                    )
                })
                .into_any_element();
        };

        let var = &self.project_vars.read(cx).references[row_ix];
        match &self.cell_state {
            CellState::CellEdited(row, col, _) if *row == row_ix && *col == col_ix => {
                match col_ix {
                    1 | 3 | 4 => Input::new(&self.cell_input_state)
                        .small()
                        .into_any_element(),
                    2 => Select::new(&self.var_kind_select_state)
                        .small()
                        .into_any_element(),
                    _ => unreachable!(),
                }
            }
            _ => match col_ix {
                1 => var.name.clone().into_any_element(),
                2 => SharedString::new(format!("{}", var.kind)).into_any_element(),
                3 => if let Some(profile_id) = profile_id {
                    if let Some(overridden_var) = self
                        .project_vars
                        .read(cx)
                        .find_override(var.id, profile_id)
                    {
                        Label::new(overridden_var).text_color(cx.theme().green)
                    } else {
                        Label::new(var.value.clone())
                    }
                } else {
                    Label::new(var.value.clone())
                }
                .into_any_element(),
                4 => var.description.clone().into_any_element(),
                _ => unreachable!(),
            },
        }
    }
}

impl Render for VariableEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let table_state = self.table_state.clone();
        let switch_on = table_state.read(cx).delegate().switch_on;
        let profile_select_state = &table_state.read(cx).delegate().profile_select_state;
        let (is_editing, no_row_selected) = self.table_state.read_with(cx, |this, _| {
            let delegate = this.delegate();
            (
                matches!(delegate.cell_state, CellState::CellEdited(_, _, _)),
                matches!(delegate.cell_state, CellState::Unselected),
            )
        });

        v_flex()
            .track_focus(&self.focus_handle)
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
                                    .checked(switch_on)
                                    .small()
                                    .on_click({
                                        let table_state = table_state.clone();
                                        move |value, _, cx| {
                                            table_state.update(cx, |state, _| {
                                                state.delegate_mut().switch_on = *value
                                            })
                                        }
                                    }),
                            )
                            .when(switch_on, |this| {
                                this.child(Select::new(profile_select_state).small().min_w_32())
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_x_2()
                            .flex_row_reverse()
                            .child(
                                Button::new("btn-add-variable")
                                    .ghost()
                                    .small()
                                    .disabled(is_editing)
                                    .icon(IconName::Plus)
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
                                Button::new("btn-delete-variable")
                                    .ghost()
                                    .small()
                                    .disabled(is_editing || no_row_selected)
                                    .icon(IconName::Delete)
                                    .on_click({
                                        let table_state = table_state.clone();
                                        move |_, window, cx| {
                                            table_state.update(cx, |this, cx| {
                                                this.delegate_mut()
                                                    .delete_selected_variable(window, cx);
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
            .child(DataTable::new(&self.table_state).small())
    }
}
