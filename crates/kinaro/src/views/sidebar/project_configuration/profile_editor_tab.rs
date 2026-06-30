use crate::actions::EscAction;
use crate::views::sidebar::project_configuration::ProjectConfigurationTab;
use crate::workspace::Project;
use crate::workspace::variable::Profile;
use gpui::prelude::*;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::PopupMenu;
use gpui_component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, WindowExt, h_flex, v_flex,
};
use ki_assets::icon::IconAsset;
use serde::Deserialize;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = profile, no_json)]
struct DeleteProfileAction(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = profile, no_json)]
struct DuplicateProfileAction(usize);

pub(super) struct ProfileEditor {
    focus_handle: FocusHandle,
    table_state: Option<Entity<TableState<ProfileDataTableDelegate>>>,
    _table_subscriptions: Option<Subscription>,
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
            TableEvent::SelectCell(row_ix, col_ix) => this
                .delegate_mut()
                .on_cell_selected(*row_ix, *col_ix, window, cx),
            TableEvent::DoubleClickedCell(row_ix, col_ix) => this
                .delegate_mut()
                .on_cell_edited(*row_ix, *col_ix, window, cx),
            TableEvent::SelectRow(ix) => {
                this.delegate_mut().on_row_selected(*ix);
            }
            TableEvent::ClearSelection => {
                this.delegate_mut().reset_state();
            }
            _ => {}
        });
    }

    fn on_delete_row_action(
        &mut self,
        row: &DeleteProfileAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let row_ix = row.0;
        self.table_state.as_ref().unwrap().update(cx, |this, cx| {
            this.delegate_mut().delete_row(row_ix, window, cx);
        });
    }

    fn on_duplicate_row_action(
        &mut self,
        row: &DuplicateProfileAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.table_state
            .as_ref()
            .unwrap()
            .update(cx, |table_state, cx| {
                table_state
                    .delegate_mut()
                    .duplicate_profile(row.0, window, cx);
            });
    }

    fn on_clear_selection(
        &mut self,
        _action: &EscAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.table_state
            .as_ref()
            .unwrap()
            .update(cx, |table_state, cx| table_state.clear_selection(cx));
    }
}

impl ProjectConfigurationTab for ProfileEditor {
    fn name() -> &'static str {
        "Profiles"
    }

    fn icon() -> impl Into<Icon> {
        IconAsset::Variable
    }

    fn new(
        project: Entity<Project>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<impl Render> {
        cx.new(|cx| {
            let profiles_table_state = cx.new(|cx| {
                TableState::new(
                    ProfileDataTableDelegate::new(project, window, cx),
                    window,
                    cx,
                )
                .row_selectable(true)
                .col_selectable(false)
                .cell_selectable(true)
            });
            let _table_subscriptions =
                Some(cx.subscribe_in(&profiles_table_state, window, Self::on_table_event));
            let table_state = Some(profiles_table_state);

            Self {
                focus_handle: cx.focus_handle(),
                table_state,
                _table_subscriptions,
            }
        })
    }
}

pub struct ProfileDataTableDelegate {
    active_project: Entity<Project>,
    profiles: Vec<Profile>,
    table_columns: Vec<Column>,
    edited_cell: Option<(usize, usize)>,
    selected_cell: Option<(usize, usize)>,
    selected_row: Option<usize>,
    name_state: Entity<InputState>,
    _name_state_sub: Option<Subscription>,
    description_state: Entity<InputState>,
    _description_state_sub: Option<Subscription>,
}

impl ProfileDataTableDelegate {
    fn new(
        active_project: Entity<Project>,
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
        let profiles =
            active_project.read_with(cx, |project, _| project.data().variables.profiles.clone());

        Self {
            active_project,
            profiles,
            table_columns,
            edited_cell: None,
            selected_cell: None,
            selected_row: None,
            name_state,
            _name_state_sub: None,
            description_state,
            _description_state_sub: None,
        }
    }

    fn reset_state(&mut self) {
        self._name_state_sub = None;
        self._description_state_sub = None;
        self.edited_cell = None;
        self.selected_cell = None;
        self.selected_row = None;
    }

    fn on_cell_selected(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        self.save_profiles(window, cx);
        let selected_cell = Some((row_ix, col_ix));
        if self.selected_cell != selected_cell && self.edited_cell != selected_cell {
            self.reset_state();
            self.selected_cell = Some((row_ix, col_ix));
        }
    }

    fn on_row_selected(&mut self, row_ix: usize) {
        let selected_row = Some(row_ix);
        if selected_row != self.selected_row {
            self.reset_state();
            self.selected_row = selected_row;
        }
    }

    fn on_cell_edited(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        let edited_cell = Some((row_ix, col_ix));
        if self.edited_cell == edited_cell {
            return;
        }
        self.reset_state();
        self.edited_cell = edited_cell;
        match col_ix {
            0 => {
                self.name_state.update(cx, |state, cx| {
                    let name = self.profiles.get(row_ix).unwrap().name.clone();
                    state.set_value(name, window, cx);
                    state.focus(window, cx);
                });
                self._name_state_sub = Some(cx.subscribe_in(
                    &self.name_state,
                    window,
                    |table, state, event, window, cx| match event {
                        InputEvent::Change => {
                            let text = state.read(cx).value();
                            let row = table.delegate().edited_cell.unwrap().0;
                            table.delegate_mut().profiles.get_mut(row).unwrap().name = text;
                        }
                        InputEvent::PressEnter {
                            secondary: _,
                            shift: _,
                        } => {
                            table.delegate_mut().save_profiles(window, cx);
                            table.delegate_mut().reset_state();
                        }
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
                self._description_state_sub = Some(cx.subscribe_in(
                    &self.description_state,
                    window,
                    |table, state, event, window, cx| match event {
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
                        InputEvent::PressEnter {
                            secondary: _,
                            shift: _,
                        } => {
                            table.delegate_mut().save_profiles(window, cx);
                            table.delegate_mut().reset_state();
                        }
                        _ => {}
                    },
                ));
            }
            _ => unreachable!(),
        }
    }

    fn save_profiles(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
        let index = self.selected_row_index();
        if let Some(row) = index {
            let profile = self.profiles.get(row).unwrap();
            match self.active_project.update(cx, |project, cx| {
                project.update_profile(profile, window, cx)
            }) {
                Ok(profiles) => self.profiles = profiles,
                Err(err) => window.push_notification(err, cx),
            }
        }
    }

    fn add_profile(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
        let name = "new profile";
        let index = self.selected_row_index();
        match self.active_project.update(cx, |project, cx| {
            project.add_profile(index, name, window, cx)
        }) {
            Ok(profiles) => self.profiles = profiles,
            Err(err) => window.push_notification(err, cx),
        }
    }

    fn duplicate_profile(
        &mut self,
        position: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        match self.active_project.update(cx, |project, cx| {
            project.duplicate_profile(position, window, cx)
        }) {
            Ok(profiles) => self.profiles = profiles,
            Err(err) => window.push_notification(err, cx),
        }
    }

    fn delete_row(
        &mut self,
        row_ix: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> bool {
        let result = self
            .active_project
            .update(cx, |project, cx| project.delete_profile(row_ix, window, cx));
        match result {
            Ok(profiles) => {
                self.profiles = profiles;
                self.reset_state();
                true
            }
            Err(err) => {
                window.push_notification(err, cx);
                false
            }
        }
    }

    fn delete_selected_row(&mut self, window: &mut Window, cx: &mut Context<TableState<Self>>) {
        let Some(row_ix) = self.selected_row_index() else {
            return;
        };
        if self.delete_row(row_ix, window, cx) {
            self.selected_row = if row_ix >= self.profiles.len() {
                None
            } else {
                Some(row_ix)
            }
        }
    }

    fn move_row(
        &mut self,
        from: usize,
        to: usize,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        match self
            .active_project
            .update(cx, |project, cx| project.move_profile(from, to, window, cx))
        {
            Ok(profiles) => self.profiles = profiles,
            Err(err) => window.push_notification(err, cx),
        }
    }

    fn selected_row_index(&self) -> Option<usize> {
        if let Some(row_ix) = self.selected_row {
            Some(row_ix)
        } else if let Some((row_ix, _)) = self.selected_cell {
            Some(row_ix)
        } else {
            None
        }
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

    fn render_tr(
        &mut self,
        row_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> Stateful<Div> {
        div()
            .id(("row", row_ix))
            .on_drag(
                MovingProfile {
                    name: self.profiles.get(row_ix).unwrap().name.clone(),
                    row: row_ix,
                },
                |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                },
            )
            .on_drop(cx.listener(move |table, e: &MovingProfile, window, cx| {
                table.delegate_mut().move_row(e.row, row_ix, window, cx);
            }))
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        menu.menu_with_icon(
            SharedString::new("Delete"),
            IconName::Delete,
            Box::new(DeleteProfileAction(row_ix)),
        )
        .menu_with_icon(
            SharedString::new("Duplicate"),
            IconName::Copy,
            Box::new(DuplicateProfileAction(row_ix)),
        )
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
                0 => Input::new(&self.name_state).small(),
                1 => Input::new(&self.description_state).small(),
                _ => unreachable!(),
            }
            .into_any_element()
        } else {
            match col_ix {
                0 => name,
                1 => description,
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

impl Render for ProfileEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_delete_row_action))
            .on_action(cx.listener(Self::on_duplicate_row_action))
            .on_action(cx.listener(Self::on_clear_selection))
            .p_1()
            .size_full()
            .gap_y_2()
            .when_some(self.table_state.clone(), |div, table| {
                let (is_editing, current_row_selection) = table.read_with(cx, |this, _| {
                    let delegate = this.delegate();
                    (
                        delegate.edited_cell.is_some(),
                        delegate.selected_row_index(),
                    )
                });
                div.child(
                    h_flex()
                        .gap_x_2()
                        .w_full()
                        .flex_row_reverse()
                        .child(
                            Button::new("btn-add-profile")
                                .ghost()
                                .with_size(gpui_component::Size::Small)
                                .disabled(is_editing)
                                .icon(IconName::Plus)
                                .on_click({
                                    let table_state = table.clone();
                                    move |_, window, cx| {
                                        table_state.update(cx, |this, cx| {
                                            this.delegate_mut().add_profile(window, cx);
                                        });
                                    }
                                }),
                        )
                        .child(
                            Button::new("btn-remove-profile")
                                .ghost()
                                .with_size(gpui_component::Size::Small)
                                .disabled(is_editing || current_row_selection.is_none())
                                .icon(IconName::Delete)
                                .on_click({
                                    let table_state = table.clone();
                                    move |_, window, cx| {
                                        table_state.update(cx, |this, cx| {
                                            this.delegate_mut().delete_selected_row(window, cx);
                                        });
                                    }
                                }),
                        ),
                )
                .child(DataTable::new(&table).small())
            })
            .when_none(&self.table_state, |div| {
                div.items_center()
                    .justify_center()
                    .child("Select a project")
            })
    }
}

#[derive(Clone)]
struct MovingProfile {
    row: usize,
    name: SharedString,
}

impl Render for MovingProfile {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_4()
            .py_1()
            .bg(cx.theme().table_head)
            .text_color(cx.theme().muted_foreground)
            .opacity(0.9)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .shadow_md()
            .min_w(px(100.))
            .max_w(px(450.))
            .child(self.name.clone())
    }
}
