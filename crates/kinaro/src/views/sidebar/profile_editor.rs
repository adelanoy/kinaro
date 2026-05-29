use gpui::*;
use gpui_component::{
    group_box::{GroupBox, GroupBoxVariants},
    h_flex,
    sheet::Sheet,
    table::{Column, TableDelegate},
    v_flex,
};

use crate::workspace::{WorkspaceProject, variable::WorkspaceProfile};

pub(super) struct ProfileVarEditor {
    project: Entity<WorkspaceProject>,
}

impl ProfileVarEditor {
    pub fn new(project: Entity<WorkspaceProject>, cx: &mut App) {}
}

impl ProfileVarEditor {
    pub fn render_profile_editor(
        project: Entity<WorkspaceProject>,
    ) -> impl Fn(Sheet, &mut Window, &mut App) -> Sheet {
        move |sheet, window, _cx| {
            let height = window.bounds().size.height;
            sheet
                .size(height / 2.0)
                .title("Profiles and variables")
                .overlay(false)
                .on_close({
                    //let project = project.clone();
                    move |_, _window, _cx| {}/*project.update(cx, |project, cx| project.save(window, cx))*/
                })
                .child(
                    h_flex()
                        .size_full()
                        .gap_x_2()
                        .justify_center()
                        .child(
                            GroupBox::new()
                                .title("Profiles")
                                .outline()
                                .h_full()
                                .child(v_flex()
                                    .h_128()
                                    .w_64()
                                    .items_center()
                                    .child("Profile table")),
                        )
                        .child(
                            GroupBox::new()
                                .title("Variables")
                                .outline()
                                .h_full()
                                .child(v_flex()
                                    .h_full()
                                    .w_64()
                                    .items_center()
                                    .child("Vars table")),
                        ),
                )
        }
    }
}

struct ProfileDataTableDelegate {
    profiles: Vec<WorkspaceProfile>,
    edited_cell: Option<(usize, usize)>,
}

impl TableDelegate for ProfileDataTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        2
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.profiles.len()
    }

    fn column(&self, col_ix: usize, cx: &App) -> Column {
        match col_ix {
            0 => Column::new("", ""),
            1 => Column::new("", ""),
            _ => unreachable!(),
        }
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut Window,
        cx: &mut Context<gpui_component::table::TableState<Self>>,
    ) -> impl IntoElement {
        if Some((row_ix, col_ix)) == self.edited_cell {
            div()
        } else {
            div()
        }
    }
}
