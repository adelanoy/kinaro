use crate::event::SidebarCollapseStateChanged;
use gpui::*;
use gpui_component::button::Toggle;
use gpui_component::label::Label;
use gpui_component::switch::Switch;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, StyledExt, Theme, ThemeMode, TitleBar,
};
use kassets::icon::IconAsset;
use settings::app_state::AppState;

pub(crate) struct AppTitleBar {
    sidebar_collapsed: bool,
}

impl AppTitleBar {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let sidebar_collapsed = AppState::read(cx, |app_state| app_state.sidebar.collapsed);
        Self { sidebar_collapsed }
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark_mode = cx.theme().mode == ThemeMode::Dark;
        let theme_toggler_icon = if self.sidebar_collapsed {
            IconAsset::SidebarCollapsed
        } else {
            IconAsset::SidebarOpen
        };

        TitleBar::new()
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(Label::new("Kinaro").font_semibold().text_2xl()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        Toggle::new("sidebar-toggle")
                            .icon(theme_toggler_icon)
                            .checked(!self.sidebar_collapsed)
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                this.sidebar_collapsed = !this.sidebar_collapsed;
                                AppState::update(cx, |state, _| {
                                    state.sidebar.collapsed = this.sidebar_collapsed
                                });
                                cx.emit(SidebarCollapseStateChanged(this.sidebar_collapsed));
                            })),
                    )
                    .child(Icon::new(IconName::Moon))
                    .child(
                        Switch::new("theme-mode-selector")
                            .checked(is_dark_mode)
                            .small()
                            .on_click(|checked, _, cx| {
                                let mode = if *checked {
                                    ThemeMode::Dark
                                } else {
                                    ThemeMode::Light
                                };
                                Theme::change(mode, None, cx);
                                cx.refresh_windows();
                                AppState::update(cx, |app_state, _cx| app_state.theme = mode)
                            }),
                    ),
            )
    }
}

impl EventEmitter<SidebarCollapseStateChanged> for AppTitleBar {}
