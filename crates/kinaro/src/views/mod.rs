mod title_bar;

use gpui::*;
use gpui_component::{v_flex, Root};
use kworkspace::Workspace;
use settings::GlobalSettings;
use crate::views::title_bar::AppTitleBar;

pub struct WorkspaceView {
    workspace: Entity<Workspace>,
    title_bar: Entity<AppTitleBar>,
}

impl WorkspaceView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe_window_bounds(window, move |_this, window, cx| {
            cx.update_global(|settings: &mut GlobalSettings, cx| {
                settings.update_app_state(cx, |app_state, cx| app_state.update_bounds(window, cx))
            })
        })
        .detach();

        let workspace = cx.new(|cx| Workspace::init(cx));
        let title_bar = cx.new(|cx| AppTitleBar::new(cx));

        Self {
            workspace,
            title_bar
        }
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("kinaro-root")
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(self.title_bar.clone())
                    .child(
                        v_flex()
                            .size_full()
                            .items_center()
                            .justify_center()
                            .child("Hello, World!")
                    )
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
