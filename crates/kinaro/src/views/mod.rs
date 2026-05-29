mod sidebar;
mod title_bar;

use crate::views::sidebar::ProjectSidebar;
use crate::views::title_bar::AppTitleBar;
use crate::workspace::Workspace;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants, Toggle};
use gpui_component::resizable::{h_resizable, resizable_panel};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{h_flex, v_flex, IconName, Root, Sizable};
use kassets::icon::IconAsset;
use settings::app_state::AppState;

pub struct WorkspaceView {
    _workspace: Entity<Workspace>,
    title_bar: Entity<AppTitleBar>,
    project_sidebar: Entity<ProjectSidebar>,
    sidebar_collapsed: bool,
    sidebar_width: f32,
}

impl WorkspaceView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe_window_bounds(window, move |_this, window, cx| {
            AppState::update(cx, |app_state, cx| app_state.update_bounds(window, cx))
        })
        .detach();
        let sidebar_collapsed = AppState::read(cx, |app_state| app_state.sidebar.collapsed);

        let workspace = cx.new(|cx| Workspace::init(cx));
        let project_sidebar = cx.new(|cx| ProjectSidebar::new(workspace.clone(), window, cx));
        let title_bar = cx.new(|_| AppTitleBar::new());
        let sidebar_width = AppState::read(cx, |app_state| app_state.sidebar.width);

        Self {
            _workspace: workspace,
            title_bar,
            project_sidebar,
            sidebar_collapsed,
            sidebar_width,
        }
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme_toggler_icon = if self.sidebar_collapsed {
            IconAsset::SidebarCollapsed
        } else {
            IconAsset::SidebarOpen
        };

        div()
            .id("kinaro-root")
            .size_full()
            .child(
                v_flex().size_full().child(self.title_bar.clone()).child(
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
                                .size_range(px(250.0)..px(500.0))
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
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
