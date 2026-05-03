use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::StyledExt;
use kworkspace::Workspace;
use settings::GlobalSettings;

pub struct WorkspaceView {
    workspace: Entity<Workspace>,
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

        Self {
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
