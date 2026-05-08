mod sidebar;
mod title_bar;

use std::rc::Rc;
use crate::event::SidebarCollapseStateChanged;
use crate::views::sidebar::ProjectSidebar;
use crate::views::title_bar::AppTitleBar;
use gpui::*;
use gpui_component::resizable::{h_resizable, resizable_panel};
use gpui_component::{Root, v_flex};
use kworkspace::Workspace;
use settings::app_state::AppState;

pub struct WorkspaceView {
    workspace: Rc<Entity<Workspace>>,
    title_bar: Entity<AppTitleBar>,
    project_sidebar: Entity<ProjectSidebar>,
    sidebar_collapsed: bool,
    _subscriptions: Vec<Subscription>,
}

impl WorkspaceView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe_window_bounds(window, move |_this, window, cx| {
            AppState::update(cx, |app_state, cx| app_state.update_bounds(window, cx))
        })
        .detach();

        let mut _subscriptions = Vec::new();

        let sidebar_collapsed = AppState::read(cx, |app_state| app_state.sidebar.collapsed);

        let workspace = Rc::new(cx.new(|cx| Workspace::init(cx)));
        let project_sidebar = cx.new(|cx| ProjectSidebar::new(cx, workspace.clone()));
        let title_bar = cx.new(|cx| AppTitleBar::new(cx));

        _subscriptions.push(cx.subscribe(
            &title_bar,
            |this, _, e: &SidebarCollapseStateChanged, _| {
                this.sidebar_collapsed = e.0;
            },
        ));

        Self {
            workspace,
            title_bar,
            project_sidebar,
            sidebar_collapsed,
            _subscriptions,
        }
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar_width = AppState::read(cx, |app_state| app_state.sidebar.width);

        div()
            .id("kinaro-root")
            .size_full()
            .child(
                v_flex().size_full().child(self.title_bar.clone()).child(
                    h_resizable("kinaro-main-view")
                        .on_resize(|state, _, cx| {
                            let width = state.read_with(cx, |state, _| state.sizes()[0]);
                            AppState::update(cx, |app_state, _| {
                                app_state.sidebar.width = width.as_f32();
                            })
                        })
                        .child(
                            resizable_panel()
                                .visible(!self.sidebar_collapsed)
                                .size(px(sidebar_width))
                                .size_range(px(170.0)..px(350.0))
                                .child(self.project_sidebar.clone()),
                        )
                        .child(
                            v_flex()
                                .size_full()
                                .items_center()
                                .justify_center()
                                .child("Hello, World!")
                                .into_any_element(),
                        ),
                ),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
