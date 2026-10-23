use gpui_kit::component::WindowExt;
use gpui_kit::component::notification::Notification;
use gpui_kit::{App, ClipboardItem, Window};
use ki_utils::TestPath;

pub mod dialog;
pub mod panel;

pub fn copy_id(path: TestPath, window: &mut Window, cx: &mut App) {
  cx.write_to_clipboard(ClipboardItem::new_string(path.id().to_string()));
  window.push_notification(Notification::info("Id copied!"), cx);
}

pub fn copy_path(path: TestPath, window: &mut Window, cx: &mut App) {
  cx.write_to_clipboard(ClipboardItem::new_string(path.to_string()));
  window.push_notification(Notification::info("Path copied!"), cx);
}
