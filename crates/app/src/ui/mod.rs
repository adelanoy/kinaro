mod components;
mod side_bar;
mod title_bar;

use crate::ui::side_bar::ProjectSidebar;
use crate::ui::title_bar::AppTitleBar;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::dialog::{DialogAction, DialogClose, DialogFooter};
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle};
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::{ActiveTheme, Disableable, Icon, IconName, Sizable, Size, WindowExt, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
  App, AppContext, Axis, ClickEvent, Context, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement,
  PathPromptOptions, Render, SharedString, Styled, Subscription, Window, div, px,
};
use ki_app_shared::actions::{CreateProject, OpenProject};
use ki_editor::{EditorEvent, EditorView};
use ki_settings::app_state::AppState;
use ki_workspace::project::PROJECT_FILE_EXT;
use ki_workspace::{Workspace, WorkspaceEvent};
use std::path::PathBuf;

struct MainContent {
  project_sidebar: Entity<ProjectSidebar>,
  editor: Entity<EditorView>,
  _editor_sub: Subscription,
}

pub struct WorkspaceView {
  workspace: Entity<Workspace>,
  title_bar: Entity<AppTitleBar>,
  sidebar_collapsed: bool,
  sidebar_width: f32,
  main_content: Option<MainContent>,
  focus_handle: FocusHandle,
  _workspace_sub: Subscription,
}

impl WorkspaceView {
  pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    cx.observe_window_bounds(window, move |_this, window, cx| {
      AppState::update(cx, |app_state, cx| app_state.update_bounds(window, cx))
    })
    .detach();

    // Workspace
    let workspace = cx.new(Workspace::init);
    let _workspace_sub = cx.subscribe_in(&workspace, window, |this, workspace, event, window, cx| {
      if let WorkspaceEvent::ActiveProjectChanged = event {
        this.main_content = Self::load_project(workspace, window, cx);
        cx.notify();
      }
    });

    // Title bar
    let title_bar = cx.new(|cx| AppTitleBar::new(workspace.clone(), cx));

    // Sidebar
    let sidebar_state = AppState::read(cx, |app_state| app_state.sidebar);
    let sidebar_collapsed = sidebar_state.collapsed;
    let mut sidebar_width = sidebar_state.width;
    let max_sidebar_width = window.bounds().size.width.as_f32() * 0.8;
    if sidebar_width > max_sidebar_width {
      sidebar_width = max_sidebar_width;
    }
    let main_content = Self::load_project(&workspace, window, cx);

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
      workspace: workspace.clone(),
      title_bar,
      sidebar_collapsed,
      sidebar_width,
      main_content,
      focus_handle,
      _workspace_sub,
    }
  }

  fn load_project(workspace: &Entity<Workspace>, window: &mut Window, cx: &mut Context<Self>) -> Option<MainContent> {
    workspace.read(cx).active_project().map(|project| {
      let project_sidebar = cx.new(|cx| ProjectSidebar::new(project.clone(), window, cx));
      let editor = cx.new(|cx| EditorView::new(project.clone(), window, cx));
      let _editor_sub = cx.subscribe(&editor, |this, _, _: &EditorEvent, cx| {
        this.sidebar_collapsed = !this.sidebar_collapsed;
        AppState::update(cx, |state, _| {
          state.sidebar.collapsed = this.sidebar_collapsed;
          true
        });
        cx.notify();
      });
      MainContent {
        project_sidebar,
        editor,
        _editor_sub,
      }
    })
  }

  fn on_action_create_project(&mut self, _: &CreateProject, window: &mut Window, cx: &mut Context<Self>) {
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
            .child(field().label("Name").child(Input::new(&name_input)).required(true))
            .child(
              field().label("Path").required(true).child(
                h_flex()
                  .gap_2()
                  .border_1()
                  .border_color(cx.theme().input)
                  .bg(cx.theme().input_background())
                  .rounded(cx.theme().radius)
                  .child(div().flex_1().child(Input::new(&path_input).pl_0().appearance(false)))
                  .child(
                    Button::new("file")
                      .ghost()
                      .icon(IconName::FolderOpen)
                      .on_click(prompt_to_save_project(path_input.clone(), cx)),
                  ),
              ),
            ),
        )
        .footer(
          DialogFooter::new()
            .child(DialogClose::new().child(Button::new("cancel").label("Cancel").outline()))
            .child(
              DialogAction::new().child(Button::new("confirm").primary().label("Create").disabled(
                name_input.read(cx).value().is_empty()
                  || path_input.read_with(cx, |state, _| {
                    state.value().is_empty() || !state.value().ends_with(PROJECT_FILE_EXT)
                  }),
              )),
            ),
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
              window.push_notification(Notification::error("Cannot create project: invalid location"), cx);
              return false;
            }

            workspace.update(cx, |workspace, cx| workspace.create_project(project_name, project, cx));
            true
          }
        })
    });
  }

  fn on_action_open_project(&mut self, _: &OpenProject, window: &mut Window, cx: &mut Context<Self>) {
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
          if let Err(err) = this.workspace.update(cx, |this, cx| this.open_project(paths[0].clone(), cx)) {
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
    let max_sidebar_width = window.bounds().size.width * 0.8;

    div()
      .id("kinaro-root")
      .track_focus(&self.focus_handle)
      .size_full()
      .on_action(cx.listener(Self::on_action_create_project))
      .on_action(cx.listener(Self::on_action_open_project))
      .child(
        v_flex()
          .size_full()
          .child(self.title_bar.clone())
          .child(h_flex().size_full().when_else(
            self.main_content.is_some(),
            |this| {
              this.child(
                h_resizable("kinaro-main-view")
                  .on_resize({
                    let this = cx.entity();
                    move |state, _, cx| {
                      let width = state.read_with(cx, |state, _| state.sizes()[0]).as_f32();
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
                      .child(self.main_content.as_ref().unwrap().project_sidebar.clone()),
                  )
                  .child(self.main_content.as_ref().unwrap().editor.clone().into_any_element()),
              )
            },
            |this| {
              this.child(
                Empty::new()
                  .header(
                    EmptyHeader::new()
                      .media(
                        EmptyMedia::new()
                          .with_variant(EmptyMediaVariant::Icon)
                          .child(Icon::new(IconName::FolderOpen)),
                      )
                      .title(EmptyTitle::new().child("No projects yet"))
                      .description(EmptyDescription::new().child("Create your first project to get started.")),
                  )
                  .content(
                    EmptyContent::new()
                      .flex_row()
                      .flex_wrap()
                      .justify_center()
                      .gap_2()
                      .child(
                        Button::new("create-project")
                          .label("Create project…")
                          .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(CreateProject), cx);
                          }),
                      )
                      .child(
                        Button::new("import-project")
                          .outline()
                          .label("Import project…")
                          .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(OpenProject), cx);
                          }),
                      ),
                  ),
              )
            },
          )),
      )
  }
}

fn prompt_to_save_project(path_input: Entity<InputState>, cx: &mut App) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
  let last_path = AppState::read(cx, |state| state.last_dir_path.clone());
  move |_, window, cx| {
    let path = cx.prompt_for_new_path(&last_path, Some(&format!("project.{}", PROJECT_FILE_EXT)));
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
              path_input.update(cx, |state, cx| state.set_value(path_str, window, cx));
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
