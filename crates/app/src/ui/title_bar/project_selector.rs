use crate::actions::{CreateProject, OpenProject};
use ki_workspace::{Project, Workspace, WorkspaceEvent, WorkspaceProjectInfo};
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariants};
use gpui_kit::component::dialog::{DialogAction, DialogClose, DialogFooter};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::label::Label;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::{
  Disableable, Icon, IconName, Sizable, WindowExt, gray_500, gray_600, h_flex, red_300, red_400,
  v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use ki_assets::icon::IconAsset;
use std::path::PathBuf;

struct ProjectManagementPopover {
  workspace: Entity<Workspace>,
  project_infos: Vec<WorkspaceProjectInfo>,
  open: bool,
  _workspace_event_sub: Subscription,
}

impl ProjectManagementPopover {
  fn new(workspace: Entity<Workspace>, cx: &mut Context<Self>) -> Self {
    let project_infos = workspace.read(cx).all_project_infos();
    let _workspace_event_sub = cx.subscribe(
      &workspace,
      |this, workspace, e: &WorkspaceEvent, cx| match e {
        WorkspaceEvent::ProjectsChanged | WorkspaceEvent::ActiveProjectChanged => {
          this.project_infos = workspace.read(cx).all_project_infos();
        }
      },
    );

    Self {
      workspace,
      project_infos,
      open: false,
      _workspace_event_sub,
    }
  }

  fn on_reload_project(
    &mut self,
    project_path: PathBuf,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.workspace.update(cx, |workspace, cx| {
      match workspace.reload_project(project_path, cx) {
        Ok(operation_occurred) => {
          if operation_occurred {
            window.push_notification(
              Notification::success("Project has been successfully reloaded"),
              cx,
            );
          }
        }
        Err(err) => window.push_notification(err, cx),
      };
    });
    self.open = false;
    cx.notify();
  }

  fn on_remove_project(
    &mut self,
    project_path: &PathBuf,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.workspace.update(cx, |workspace, cx| {
      workspace.remove_project(project_path, cx);
      window.push_notification(Notification::success("Project has been removed"), cx);
    });
    self.open = false;
    cx.notify();
  }

  fn on_switch_project(
    &mut self,
    project_path: PathBuf,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    if let Err(err) = self
      .workspace
      .update(cx, move |this, cx| this.switch_project(project_path, cx))
    {
      window.push_notification(err, cx);
    }
    self.open = false;
    cx.notify();
  }

  fn on_rename_project(
    &mut self,
    name: SharedString,
    project_path: PathBuf,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let workspace = self.workspace.clone();
    let input = cx.new(|cx| {
      let mut state = InputState::new(window, cx);
      state.set_value(name, window, cx);
      state
    });

    window.open_dialog(cx, move |dialog, _, cx| {
      dialog
        .title("Change Project Name")
        .child(
          v_flex()
            .gap_3()
            .child("Enter the project's name:")
            .child(Input::new(&input)),
        )
        .footer(
          DialogFooter::new()
            .child(
              DialogClose::new()
                .child(Button::new("cancel").label("Cancel").outline()),
            )
            .child(
              DialogAction::new()
                .child(Button::new("confirm").primary().label("Rename")),
            ),
        )
        .on_ok({
          let name = input.clone().read(cx).value();
          let project_path = project_path.clone();
          let workspace = workspace.clone();
          move |_, _, cx| {
            workspace.update(cx, |workspace, cx| {
              workspace.rename_project(&project_path, name.clone(), cx)
            });
            true
          }
        })
    });
  }
}

impl Render for ProjectManagementPopover {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let project_menu: Vec<AnyElement> = self
      .project_infos
      .iter()
      .enumerate()
      .map(|(ix, info)| {
        let info = info.clone();

        Button::new(ElementId::NamedInteger(
          SharedString::new("switch_project"),
          ix as u64,
        ))
          .ghost()
          .w_full()
          .h_12()
          .disabled(!info.loaded)
          .when_else(
            info.loaded,
            |button| {
              button.child(
                div()
                  .flex()
                  .gap_x_2()
                  .size_full()
                  .items_center()
                  .justify_between()
                  .when_else(
                    info.active,
                    |this| this.child(IconName::Check),
                    |this| this.child(Icon::empty()),
                  )
                  .child(
                    div()
                      .flex_col()
                      .child(Label::new(info.name.clone()).text_sm())
                      .child(
                        Label::new(info.path.clone().to_string_lossy())
                          .text_ellipsis()
                          .text_xs()
                          .text_color(gray_600()),
                      ),
                  )
                  .child(
                    Button::new("rename")
                      .custom(ButtonCustomVariant::new(cx).color(gray_500()))
                      .xsmall()
                      .icon(IconAsset::Rename)
                      .on_click({
                        let path = info.path.clone();
                        let name = info.name.clone();
                        cx.listener(move |this, _, window, cx| {
                          cx.stop_propagation();
                          this.on_rename_project(
                            name.clone(),
                            path.clone(),
                            window,
                            cx,
                          );
                        })
                      }),
                  )
                  .child(
                    Button::new("remove")
                      .custom(ButtonCustomVariant::new(cx).color(gray_500()))
                      .xsmall()
                      .icon(IconName::Close)
                      .on_click({
                        let path = info.path.clone();
                        cx.listener(move |this, _, window, cx| {
                          cx.stop_propagation();
                          this.on_remove_project(&path, window, cx);
                        })
                      }),
                  ),
              )
            },
            |button| {
              button.child(
                div()
                  .flex()
                  .gap_x_2()
                  .size_full()
                  .items_center()
                  .justify_between()
                  .child(Icon::empty())
                  .child(
                    div()
                      .flex_col()
                      .child(
                        Label::new(info.name.clone())
                          .text_sm()
                          .text_color(red_400()),
                      )
                      .child(
                        Label::new(info.path.clone().to_string_lossy())
                          .text_ellipsis()
                          .text_xs()
                          .text_color(red_300()),
                      ),
                  )
                  .child(
                    Button::new("reload")
                      .custom(ButtonCustomVariant::new(cx).color(gray_500()))
                      .xsmall()
                      .icon(IconAsset::Refresh)
                      .on_click({
                        let path = info.path.clone();
                        cx.listener(move |this, _, window, cx| {
                          cx.stop_propagation();
                          this.on_reload_project(path.clone(), window, cx);
                        })
                      }),
                  )
                  .child(
                    Button::new("remove")
                      .custom(ButtonCustomVariant::new(cx).color(gray_500()))
                      .xsmall()
                      .icon(IconName::Close)
                      .on_click({
                        let path = info.path.clone();
                        cx.listener(move |this, _, window, cx| {
                          cx.stop_propagation();
                          this.on_remove_project(&path, window, cx);
                        })
                      }),
                  ),
              )
            },
          )
          .on_click(cx.listener(move |this, _, window, cx| {
            this.on_switch_project(info.path.clone(), window, cx);
          }))
          .into_any_element()
      })
      .collect();

    div()
      .flex()
      .flex_col()
      .min_w(px(200.0))
      .w_full()
      .children(project_menu)
      .child(Separator::horizontal().py_2())
      .child(
        Button::new("open-project-menu")
          .ghost()
          .w_full()
          .h_10()
          .child(
            h_flex()
              .w_full()
              .justify_between()
              .child(
                h_flex()
                  .gap_x_2()
                  .child(IconName::FolderOpen)
                  .child(Label::new("Open Project").text_sm()),
              )
              .when_some(
                Kbd::binding_for_action(&OpenProject, None, window),
                |this, kbd| this.child(kbd),
              ),
          )
          .on_click(cx.listener(move |this, _, window, cx| {
            this.open = false;
            cx.notify();
            window.dispatch_action(Box::new(OpenProject), cx);
          })),
      )
      .child(
        Button::new("create-project-menu")
          .ghost()
          .w_full()
          .h_10()
          .child(
            h_flex()
              .w_full()
              .justify_between()
              .child(
                h_flex()
                  .gap_x_2()
                  .child(IconName::Plus)
                  .child(Label::new("Create Project").text_sm()),
              )
              .when_some(
                Kbd::binding_for_action(&CreateProject, None, window),
                |this, kbd| this.child(kbd),
              ),
          )
          .on_click(cx.listener(move |this, _, window, cx| {
            this.open = false;
            cx.notify();
            window.dispatch_action(Box::new(CreateProject), cx);
          })),
      )
  }
}

pub(super) struct ProjectSelector {
  active_project: Option<Entity<Project>>,
  menu_content: Entity<ProjectManagementPopover>,
  focus_handle: FocusHandle,
  _workspace_event_sub: Subscription,
}

impl ProjectSelector {
  pub(super) fn new(workspace: Entity<Workspace>, cx: &mut Context<Self>) -> Self {
    let active_project = workspace.read(cx).active_project();
    let _workspace_event_sub =
      cx.subscribe(&workspace, |this, workspace, e: &WorkspaceEvent, cx| {
        if let WorkspaceEvent::ActiveProjectChanged = e {
          this.active_project = workspace.read(cx).active_project()
        }
      });
    let menu_content = cx.new(|cx| ProjectManagementPopover::new(workspace.clone(), cx));

    Self {
      active_project,
      menu_content,
      focus_handle: cx.focus_handle(),
      _workspace_event_sub,
    }
  }
}

impl Render for ProjectSelector {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let current_project_name = self
      .active_project
      .as_ref()
      .map(|project| project.read(cx).name.clone())
      .unwrap_or(SharedString::new("--"));
    let popover_open = self.menu_content.read(cx).open;
    div()
      .track_focus(&self.focus_handle)
      .h_full()
      .min_w_32()
      .child(
        Popover::new("project-selector-popover")
          .p_1()
          .open(popover_open)
          .on_open_change(cx.listener(|this, open: &bool, _, cx| {
            this.menu_content.update(cx, |menu, _| menu.open = *open);
            cx.notify();
          }))
          .trigger(
            Button::new("btn-project-selector")
              .secondary()
              .small()
              .rounded(ButtonRounded::Small)
              .child(
                div()
                  .flex()
                  .size_full()
                  .items_center()
                  .justify_between()
                  .child(current_project_name)
                  .child(IconName::ChevronsUpDown),
              ),
          )
          .child(self.menu_content.clone()),
      )
  }
}
