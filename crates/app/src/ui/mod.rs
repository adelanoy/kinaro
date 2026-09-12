mod side_bar;
mod title_bar;
mod components;

use crate::actions::{CreateProject, OpenProject};
use crate::ui::side_bar::ProjectSidebar;
use crate::ui::title_bar::AppTitleBar;
use ki_workspace::Workspace;
use ki_workspace::project::PROJECT_FILE_EXT;
use gpui_kit::component::button::{Button, ButtonVariants, Toggle};
use gpui_kit::component::dialog::{DialogAction, DialogClose, DialogFooter};
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{
    ActiveTheme, Disableable, IconName, Root, Sizable, Size, WindowExt, h_flex, v_flex,
};
use gpui_kit::{
    App, AppContext, Axis, ClickEvent, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, PathPromptOptions, Render, SharedString, Styled, Window, div, px,
};
use ki_assets::icon::IconAsset;
use ki_settings::app_state::AppState;
use std::path::PathBuf;

pub struct WorkspaceView {
  workspace: Entity<Workspace>,
  title_bar: Entity<AppTitleBar>,
  project_sidebar: Entity<ProjectSidebar>,
  sidebar_collapsed: bool,
  sidebar_width: f32,
  focus_handle: FocusHandle,
}

impl WorkspaceView {
  pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    cx.observe_window_bounds(window, move |_this, window, cx| {
      AppState::update(cx, |app_state, cx| app_state.update_bounds(window, cx))
    })
      .detach();
    let sidebar_collapsed = AppState::read(cx, |app_state| app_state.sidebar.collapsed);

    let workspace = cx.new(Workspace::init);
    let project_sidebar = cx.new(|cx| ProjectSidebar::new(workspace.clone(), window, cx));
    let title_bar = cx.new(|cx| AppTitleBar::new(workspace.clone(), cx));
    let mut sidebar_width = AppState::read(cx, |app_state| app_state.sidebar.width);
    let max_sidebar_width = window.bounds().size.width.as_f32() * 0.8;
    if sidebar_width > max_sidebar_width {
      sidebar_width = max_sidebar_width;
    }

    // Check if any project failed to load. If there is display a notif at end of render cycle
    let project_errors = workspace.read(cx).all_failed_projects();
    window.defer(cx, |window, cx| {
      for project_error in project_errors {
        window.push_notification(
          Notification::error(format!(
            "Failed to load project {} at path {}.\nDetails: {}",
            project_error.0,
            project_error.1.to_string_lossy(),
            project_error.2
          )),
          cx,
        );
      }
    });

    // Request focus at startup for global shortcut to work
    let focus_handle = cx.focus_handle();
    window.focus(&focus_handle, cx);

    Self {
      workspace,
      title_bar,
      project_sidebar,
      sidebar_collapsed,
      sidebar_width,
      focus_handle,
    }
  }
  fn on_create_project(
    &mut self,
    _: &CreateProject,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let workspace = self.workspace.clone();
    let name_input = cx.new(|cx| InputState::new(window, cx));
    let path_input = cx.new(|cx| InputState::new(window, cx));
    window.open_dialog(cx, move |dialog, _, cx| {
      dialog
        .title("Create Project")
        .child(
          v_form()
            .layout(Axis::Horizontal)
            .label_width(px(100.))
            .with_size(Size::Small)
            .child(
              field()
                .label("Name")
                .child(Input::new(&name_input))
                .required(true),
            )
            .child(
              field().label("Path").required(true).child(
                h_flex()
                  .gap_2()
                  .border_1()
                  .border_color(cx.theme().input)
                  .bg(cx.theme().input_background())
                  .rounded(cx.theme().radius)
                  .child(
                    div().flex_1().child(
                      Input::new(&path_input).pl_0().appearance(false),
                    ),
                  )
                  .child(
                    Button::new("file")
                      .ghost()
                      .icon(IconName::FolderOpen)
                      .on_click(prompt_to_save_project(
                        path_input.clone(),
                        cx,
                      )),
                  ),
              ),
            ),
        )
        .footer(
          DialogFooter::new()
            .child(
              DialogClose::new()
                .child(Button::new("cancel").label("Cancel").outline()),
            )
            .child(DialogAction::new().child(
              Button::new("confirm").primary().label("Create").disabled(
                name_input.read(cx).value().is_empty()
                  || path_input.read_with(cx, |state, _| {
                  state.value().is_empty()
                    || !state.value().ends_with(PROJECT_FILE_EXT)
                }),
              ),
            )),
        )
        .on_ok({
          let name = name_input.clone();
          let path = path_input.clone();
          let workspace = workspace.clone();
          move |_, window, cx| {
            let project_name = name.read(cx).value();
            let project = PathBuf::from(path.read(cx).value().to_string());
            let project_dir = project.parent();
            if project_dir.is_none() || !project_dir.unwrap().exists() {
              window.push_notification(
                Notification::error("Cannot create project: invalid location"),
                cx,
              );
              return false;
            }

            workspace.update(cx, |workspace, cx| {
              workspace.create_project(project_name, project, cx)
            });
            true
          }
        })
    });
  }

  fn on_open_project(&mut self, _: &OpenProject, window: &mut Window, cx: &mut Context<Self>) {
    let focus_handle = self.focus_handle.clone();
    let path = cx.prompt_for_paths(PathPromptOptions {
      files: true,
      directories: false,
      multiple: false,
      prompt: None,
    });
    cx.spawn_in(window, async move |this, cx| {
      let Ok(result) = path.await else {
        return;
      };
      let window_handle = cx.window_handle();
      if let Some(paths) = result.ok().flatten()
        && !paths.is_empty()
      {
        _ = this.update(cx, |this, cx| {
          if let Err(err) = this
            .workspace
            .update(cx, |this, cx| this.open_project(paths[0].clone(), cx))
          {
            _ = window_handle.update(cx, |_, window, cx| {
              window.push_notification(format!("{:?}", err), cx);
            });
          }
          AppState::update(cx, |state, _| {
            state.last_dir_path = paths[0].clone();
            true
          });
        });
      }
      _ = window_handle.update(cx, |_, window, cx| window.focus(&focus_handle, cx));
    })
      .detach();
  }
}

impl Render for WorkspaceView {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme_toggler_icon = if self.sidebar_collapsed {
      IconAsset::SidebarCollapsed
    } else {
      IconAsset::SidebarOpen
    };
    let max_sidebar_width = window.bounds().size.width * 0.8;

    div()
      .id("kinaro-root")
      .track_focus(&self.focus_handle)
      .size_full()
      .on_action(cx.listener(Self::on_create_project))
      .on_action(cx.listener(Self::on_open_project))
      .child(
        v_flex().size_full().child(self.title_bar.clone()).child(
          h_resizable("kinaro-main-view")
            .on_resize({
              let this = cx.entity();
              move |state, _, cx| {
                let width =
                  state.read_with(cx, |state, _| state.sizes()[0]).as_f32();
                this.update(cx, |this, _| this.sidebar_width = width);
                AppState::update(cx, |app_state, _| {
                  app_state.sidebar.width = width;
                  true
                })
              }
            })
            .child(
              resizable_panel()
                .visible(!self.sidebar_collapsed)
                .size(self.sidebar_width)
                .size_range(px(250.0)..max_sidebar_width)
                .child(self.project_sidebar.clone()),
            )
            .child(
              v_flex()
                .size_full()
                .child(
                  h_flex()
                    .gap_x_2()
                    .px_2()
                    .pt_1()
                    .child(
                      Toggle::new("sidebar-toggle")
                        .icon(theme_toggler_icon)
                        .checked(!self.sidebar_collapsed)
                        .on_click(cx.listener(|this, _, _, cx| {
                          this.sidebar_collapsed =
                            !this.sidebar_collapsed;
                          AppState::update(cx, |state, _| {
                            state.sidebar.collapsed =
                              this.sidebar_collapsed;
                            true
                          });
                        })),
                    )
                    .child(
                      TabBar::new("tabs")
                        .selected_index(0)
                        .child(
                          Tab::new()
                            .label("Custom Tab")
                            .suffix(
                              Button::new("inbox")
                                .ghost()
                                .xsmall()
                                .icon(IconName::Close)
                                .on_click(|_, _, cx| {
                                  println!(
                                    "Button close tab clicked"
                                  );
                                  cx.stop_propagation();
                                }),
                            )
                            .on_click(|_, _, _| {
                              println!("Custom tab clicked");
                            }),
                        )
                        .child(Tab::new().label("Profile"))
                        .child(Tab::new().label("Settings")),
                    ),
                )
                .into_any_element(),
            ),
        ),
      )
      .children(Root::render_dialog_layer(window, cx))
      .children(Root::render_notification_layer(window, cx))
  }
}

fn prompt_to_save_project(
  path_input: Entity<InputState>,
  cx: &mut App,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
  let last_path = AppState::read(cx, |state| state.last_dir_path.clone());
  move |_, window, cx| {
    let path =
      cx.prompt_for_new_path(&last_path, Some(&format!("project.{}", PROJECT_FILE_EXT)));
    window
      .spawn(cx, {
        let path_input = path_input.clone();
        async move |cx| {
          let Ok(result) = path.await else {
            return;
          };
          if let Some(path) = result.ok().flatten() {
            _ = cx.window_handle().update(cx, |_, window, cx| {
              let path_str = SharedString::new(path.to_string_lossy());
              path_input
                .update(cx, |state, cx| state.set_value(path_str, window, cx));
              AppState::update(cx, |state, _| {
                state.last_dir_path = path.parent().unwrap().to_owned();
                true
              });
            });
          }
        }
      })
      .detach();
  }
}
