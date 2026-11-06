use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme, WindowExt};
use gpui_kit::{
  App, ClipboardItem, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, div, px,
};
use ki_utils::TestPath;

pub mod actions;
pub mod dialog;
pub mod panel;
pub mod table;

/// Builds the context menu of a tree node.
pub type ContextMenuBuilder = dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu;

/// Struct maintaining an entity and its subscription
pub struct SubbedEntity<T> {
  pub entity: Entity<T>,
  pub _sub: Subscription,
}

/// Used as visual aid when dragging a piece of UI.
#[derive(Clone)]
pub struct MovingLabel<T> {
  pub data: T,
  pub label: SharedString,
}

impl<T: 'static> Render for MovingLabel<T> {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .px_4()
      .py_1()
      .bg(cx.theme().table_head)
      .text_color(cx.theme().muted_foreground)
      .opacity(0.9)
      .border_1()
      .border_color(cx.theme().border)
      .rounded_md()
      .shadow_md()
      .min_w(px(100.))
      .max_w(px(450.))
      .child(self.label.clone())
  }
}

pub fn copy_id(path: TestPath, window: &mut Window, cx: &mut App) {
  cx.write_to_clipboard(ClipboardItem::new_string(path.id().to_string()));
  window.push_notification(Notification::info("Id copied!"), cx);
}

pub fn copy_path(path: TestPath, window: &mut Window, cx: &mut App) {
  cx.write_to_clipboard(ClipboardItem::new_string(path.to_string()));
  window.push_notification(Notification::info("Path copied!"), cx);
}
