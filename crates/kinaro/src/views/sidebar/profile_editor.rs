use crate::workspace::{
    variable::{WorkspaceProfile, WorkspaceVariable},
    WorkspaceProject,
};
use gpui::{
    div, px, App, AppContext, ClickEvent, Context, Entity, IntoElement, ParentElement,
    SharedString, Styled, Subscription, WeakEntity, Window,
};
use gpui_component::button::Button;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{
    h_flex, table::{Column, DataTable, TableDelegate, TableEvent, TableState}, v_flex, Disableable, IconName,
    Sizable,
    WindowExt,
};
use gpui_component::separator::Separator;
use uuid::Uuid;

pub struct ProfileVariableEditor {
    weak_self: WeakEntity<Self>,
    active_project: Option<Entity<WorkspaceProject>>,
    profiles_table_state: Entity<TableState<ProfileDataTableDelegate>>,
    variables_table_state: Entity<TableState<VariableDataTableDelegate>>,
    _subscriptions: Vec<Subscription>,
}

impl ProfileVariableEditor {
    pub fn new(
        active_project: Option<Entity<WorkspaceProject>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let profiles_table_state = cx.new(|cx| {
            TableState::new(
                ProfileDataTableDelegate::new(vec![], window, cx),
                window,
                cx,
            )
            .row_selectable(true)
            .col_selectable(false)
            .cell_selectable(true)
        });

        let variables_table_state = cx.new(|cx| {
            TableState::new(VariableDataTableDelegate::new(vec![]), window, cx)
                .row_selectable(true)
                .col_selectable(false)
                .cell_selectable(true)
        });

        let this = Self {
            weak_self: cx.weak_entity(),
            active_project,
            profiles_table_state,
            variables_table_state,
            _subscriptions: Vec::with_capacity(2),
        };

        this.update_profiles_table_state(cx);
        this.update_variables_table_state(cx);

        this
    }

    pub fn set_project(&mut self, active_project: Option<Entity<WorkspaceProject>>, cx: &mut App) {
        self.active_project = active_project;
        self.update_profiles_table_state(cx);
        self.update_variables_table_state(cx);
    }

    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.setup_subscribers(window, cx);
        let this = self.weak_self.clone();
        let profiles_table_state = self.profiles_table_state.clone();
        profiles_table_state.update(cx, |state, _| state.delegate_mut().reset_state());
        let variables_table_state = self.variables_table_state.clone();

        window.open_sheet_at(gpui_component::Placement::Right, cx, {
            move |sheet, window, cx| {
                let width = window.bounds().size.width;
                sheet
                    .size(width / 3.0)
                    .resizable(true)
                    .overlay(true)
                    .overlay_closable(true)
                    .on_close({
                        let this = this.upgrade().unwrap();
                        Self::on_close_editor(this)
                    })
                    .child(
                        v_flex()
                            .size_full()
                            .gap_3()
                            .child("Edit Profiles:")
                            .child(DataTable::new(&profiles_table_state))
                            .child(
                                h_flex()
                                    .gap_x_2()
                                    .w_full()
                                    .flex_row_reverse()
                                    .child(
                                        Button::new("btn-add-profile")
                                            .with_size(gpui_component::Size::Small)
                                            .icon(IconName::Plus)
                                            .on_click({
                                                let profiles_table_state =
                                                    profiles_table_state.clone();
                                                move |_, _, cx| {
                                                    profiles_table_state.update(cx, |this, _| {
                                                        this.delegate_mut().add_profile();
                                                    });
                                                }
                                            }),
                                    )
                                    .child(
                                        Button::new("btn-remove-profile")
                                            .with_size(gpui_component::Size::Small)
                                            .disabled(
                                                profiles_table_state
                                                    .read(cx)
                                                    .delegate()
                                                    .selected_row
                                                    .is_none(),
                                            )
                                            .icon(IconName::Delete),
                                    ),
                            )
                            .child(Separator::horizontal())
                            .child("Edit Variables:")
                            .child(DataTable::new(&variables_table_state))
                            .child(
                                h_flex()
                                    .gap_x_2()
                                    .w_full()
                                    .flex_row_reverse()
                                    .child(
                                        Button::new("btn-add-variable")
                                            .with_size(gpui_component::Size::Small)
                                            .icon(IconName::Plus),
                                    )
                                    .child(
                                        Button::new("btn-remove-variable")
                                            .with_size(gpui_component::Size::Small)
                                            .disabled(
                                                variables_table_state
                                                    .read(cx)
                                                    .delegate()
                                                    .selected_row
                                                    .is_none(),
                                            )
                                            .icon(IconName::Delete),
                                    ),
                            ),
                    )
            }
        });
    }

    fn setup_subscribers(&mut self, window: &Window, cx: &mut Context<Self>) {
        self._subscriptions.clear();
        self._subscriptions.push(cx.subscribe_in(
            &self.profiles_table_state,
            window,
            Self::on_profile_table_event,
        ));
        self._subscriptions.push(cx.subscribe_in(
            &self.variables_table_state,
            window,
            Self::on_variable_table_event,
        ));
    }

    fn update_profiles_table_state(&self, cx: &mut App) {
        let active_project = &self.active_project;
        let profiles = match active_project {
            Some(active_project) => active_project.read_with(cx, |project, _| {
                project
                    .data()
                    .variables
                    .profiles
                    .iter()
                    .map(|p| p.clone())
                    .collect::<Vec<WorkspaceProfile>>()
            }),
            None => vec![],
        };

        self.profiles_table_state.update(cx, |this, _| {
            this.delegate_mut().profiles = profiles;
        });
    }

    fn update_variables_table_state(&self, cx: &mut App) {
        let active_project = &self.active_project;
        let variables = match active_project {
            Some(active_project) => active_project.read_with(cx, |project, _| {
                project
                    .data()
                    .variables
                    .variables
                    .iter()
                    .map(|p| p.clone())
                    .collect::<Vec<WorkspaceVariable>>()
            }),
            None => vec![],
        };

        self.variables_table_state.update(cx, |this, _| {
            this.delegate_mut().variables = variables;
        });
    }

    fn on_profile_table_event(
        &mut self,
        table: &Entity<TableState<ProfileDataTableDelegate>>,
        event: &TableEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        table.update(cx, |this, cx| match event {
            TableEvent::SelectCell(row_ix, col_ix) => {
                this.delegate_mut().on_cell_selected(*row_ix, *col_ix)
            }
            TableEvent::DoubleClickedCell(row_ix, col_ix) => this
                .delegate_mut()
                .on_cell_double_clicked(*row_ix, *col_ix, window, cx),
            TableEvent::SelectRow(ix) => {
                this.delegate_mut().on_row_selected(*ix);
            }
            TableEvent::RightClickedRow(ix) => println!("Right clicked row: {:?}", ix),
            TableEvent::RightClickedCell(row_ix, col_ix) => {
                println!("Right clicked cell: row={}, col={}", row_ix, col_ix)
            }
            TableEvent::ClearSelection => {
                this.delegate_mut().edited_cell = None;
                this.delegate_mut().selected_row = None;
            }
            _ => {}
        });
    }

    fn on_variable_table_event(
        &mut self,
        _: &Entity<TableState<VariableDataTableDelegate>>,
        event: &TableEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            TableEvent::SelectCell(row_ix, col_ix) => {
                println!("Select cell: row={}, col={}", row_ix, col_ix)
            }
            TableEvent::DoubleClickedCell(row_ix, col_ix) => {
                println!("Double clicked cell: row={}, col={}", row_ix, col_ix)
            }
            TableEvent::SelectRow(ix) => println!("Select row: {}", ix),
            TableEvent::RightClickedRow(ix) => println!("Right clicked row: {:?}", ix),
            TableEvent::RightClickedCell(row_ix, col_ix) => {
                println!("Right clicked cell: row={}, col={}", row_ix, col_ix)
            }
            TableEvent::ClearSelection => {
                println!("Selection cleared");
            }
            _ => {}
        }
    }

    fn on_close_editor(
        this: Entity<ProfileVariableEditor>,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) {
        move |_, window, cx| {
            window
                .spawn(cx, {
                    let this = this.clone();
                    async move |cx| {
                        this.update_in(cx, |this, window, cx| {
                            this._subscriptions.clear();
                            let updated_profiles = this
                                .profiles_table_state
                                .read(cx)
                                .delegate()
                                .profiles
                                .clone();
                            let updated_variables = this
                                .variables_table_state
                                .read(cx)
                                .delegate()
                                .variables
                                .clone();
                            if let Some(project) = &this.active_project {
                                project.update(cx, |project, cx| {
                                    project.update_variables_and_profiles(
                                        updated_profiles,
                                        updated_variables,
                                        window,
                                        cx,
                                    );
                                });
                            }
                        })
                        .expect("Project updated and saved");
                    }
                })
                .detach();
        }
    }
}

pub struct ProfileDataTableDelegate {
    pub profiles: Vec<WorkspaceProfile>,
    table_columns: Vec<Column>,
    edited_cell: Option<(usize, usize)>,
    selected_row: Option<usize>,
    name_state: Entity<InputState>,
    name_state_sub: Option<Subscription>,
    description_state: Entity<InputState>,
    description_state_sub: Option<Subscription>,
}

impl ProfileDataTableDelegate {
    pub fn new(
        profiles: Vec<WorkspaceProfile>,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> Self {
        let table_columns = vec![
            Column::new("profile_short_name", "Name")
                .width(px(180.0))
                .resizable(true),
            Column::new("profile_description", "Description")
                .width(px(300.0))
                .resizable(false),
        ];

        let name_state = cx.new(|cx| InputState::new(window, cx));
        let description_state = cx.new(|cx| InputState::new(window, cx));
        Self {
            profiles,
            table_columns,
            edited_cell: None,
            selected_row: None,
            name_state,
            name_state_sub: None,
            description_state,
            description_state_sub: None,
        }
    }

    fn on_cell_selected(&mut self, row_ix: usize, col_ix: usize) {
        if Some((row_ix, col_ix)) != self.edited_cell {
            self.reset_state();
        }
    }

    fn reset_state(&mut self) {
        self.name_state_sub = None;
        self.description_state_sub = None;
        self.edited_cell = None;
    }

    fn on_row_selected(&mut self, row_ix: usize) {
        if Some(row_ix) != self.selected_row {
            self.selected_row = Some(row_ix);
            self.edited_cell = None;
        }
    }

    fn on_cell_double_clicked(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        self.edited_cell = Some((row_ix, col_ix));
        match col_ix {
            0 => {
                self.name_state.update(cx, |state, cx| {
                    let name = self.profiles.get(row_ix).unwrap().name.clone();
                    state.set_value(name, window, cx);
                    state.focus(window, cx);
                });
                self.name_state_sub = Some(cx.subscribe(
                    &self.name_state,
                    |table, state, event, cx| match event {
                        InputEvent::Change => {
                            let text = state.read(cx).value();
                            let row = table.delegate().edited_cell.unwrap().0;
                            table.delegate_mut().profiles.get_mut(row).unwrap().name = text;
                        }
                        InputEvent::PressEnter {
                            secondary: _,
                            shift: _,
                        } => table.delegate_mut().reset_state(),
                        _ => {}
                    },
                ));
            }
            1 => {
                self.description_state.update(cx, |state, cx| {
                    let description = self.profiles.get(row_ix).unwrap().description.clone();
                    state.set_value(description, window, cx);
                    state.focus(window, cx);
                });
                self.description_state_sub = Some(cx.subscribe(
                    &self.description_state,
                    |table, state, event, cx| match event {
                        InputEvent::Change => {
                            let text = state.read(cx).value();
                            let row = table.delegate().edited_cell.unwrap().0;
                            table
                                .delegate_mut()
                                .profiles
                                .get_mut(row)
                                .unwrap()
                                .description = text;
                        }
                        _ => {}
                    },
                ));
            }
            _ => unreachable!(),
        }
    }

    fn add_profile(&mut self) {
        let name = "new profile";
        let mut i = 1;
        let mut final_name = name.to_string();
        loop {
            if self.profiles.iter().any(|p| p.name == final_name) {
                final_name = format!("{}_{}", name, i);
                i += 1;
            } else {
                break;
            }
        }
        self.profiles.push(WorkspaceProfile {
            id: Uuid::new_v4(),
            name: SharedString::new(final_name),
            description: Default::default(),
        });
    }
}

impl TableDelegate for ProfileDataTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.table_columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.profiles.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.table_columns.get(col_ix).unwrap().clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let name = self.profiles.get(row_ix).unwrap().name.clone();
        let description = self.profiles.get(row_ix).unwrap().description.clone();

        if Some((row_ix, col_ix)) == self.edited_cell {
            match col_ix {
                0 => Input::new(&self.name_state),
                1 => Input::new(&self.description_state),
                _ => unreachable!(),
            }
            .into_any_element()
        } else {
            match col_ix {
                0 => div().child(name),
                1 => div().child(description),
                _ => unreachable!(),
            }
            .into_any_element()
        }
    }

    fn render_empty(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        h_flex().size_full().into_any_element()
    }
}

pub struct VariableDataTableDelegate {
    pub variables: Vec<WorkspaceVariable>,
    table_columns: Vec<Column>,
    edited_cell: Option<(usize, usize)>,
    selected_row: Option<usize>,
}

impl VariableDataTableDelegate {
    pub fn new(variables: Vec<WorkspaceVariable>) -> Self {
        let table_columns = vec![
            Column::new("short_name", "Name")
                .width(px(180.0))
                .resizable(true),
            Column::new("description", "Description")
                .width(px(300.0))
                .resizable(false),
        ];
        Self {
            variables,
            table_columns,
            edited_cell: None,
            selected_row: None,
        }
    }
}

impl TableDelegate for VariableDataTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.table_columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.variables.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.table_columns.get(col_ix).unwrap().clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        if Some((row_ix, col_ix)) == self.edited_cell {
            match col_ix {
                0 => div(),
                1 => div(),
                _ => unreachable!(),
            }
        } else {
            match col_ix {
                0 => div().child(self.variables.get(row_ix).unwrap().name.clone()),
                1 => div().child(self.variables.get(row_ix).unwrap().description.clone()),
                _ => unreachable!(),
            }
        }
    }

    fn render_empty(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        h_flex().size_full().into_any_element()
    }
}
