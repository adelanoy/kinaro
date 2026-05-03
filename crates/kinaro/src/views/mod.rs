use gpui::*;
use gpui_component::StyledExt;
use gpui_component::button::{Button, ButtonVariants};
use kworkspace::Workspace;
use settings::GlobalSettings;
use std::time::Duration;

pub struct WorkspaceView {
    bounds_save_task_queued: Option<Task<()>>,
    workspace: Entity<Workspace>,
}

impl WorkspaceView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Observe the window movement and throttle the saving of its bounds every 200ms. Heavily inspired by Zed's Workspace impl
        cx.observe_window_bounds(window, move |this, window, cx| {
            if this.bounds_save_task_queued.is_some() {
                return;
            }
            this.bounds_save_task_queued = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                this.update_in(cx, |this, window, cx| {
                    cx.update_global(|settings: &mut GlobalSettings, cx| {
                        settings.window_settings.save(window, cx)
                    });
                    this.bounds_save_task_queued.take();
                })
                .ok();
            }));
            cx.notify();
        })
        .detach();

        let workspace = cx.new(|cx| Workspace::init(cx));

        Self {
            bounds_save_task_queued: None,
            workspace,
        }
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}
