use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px,
};
use std::sync::LazyLock;

pub static EMPTY_SHARED_STRING: LazyLock<SharedString> = LazyLock::new(|| SharedString::new(""));

pub enum CellState<T> {
    Unselected,
    CellSelected(usize),
    CellEdited(usize, usize, T),
}

/// Computes a name based on the given name, appended with '_X' where X is an integer and where the `name_X` is the first available in the collection.
///
/// X is incremented at each attempt
///
/// If the name is available without any suffix in the collection, returns *name*
pub fn next_available_name<'a>(
    name: &str,
    collection: &mut (impl Iterator<Item = &'a SharedString> + Clone),
) -> SharedString {
    if collection.clone().find(|p| p.as_ref() == name).is_none() {
        return SharedString::new(name);
    }

    let mut final_name = name.to_string();
    {
        let mut i = 1;
        loop {
            if collection.any(|p| *p == final_name) {
                final_name = format!("{name}_{i}");
                i += 1;
            } else {
                break;
            }
        }
    }
    SharedString::new(final_name)
}

/// Used as visual aid when dragging a piece of UI. Must be built and used in StatefulInteractiveElement.on_drag()
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

#[cfg(test)]
mod test {
    use crate::ui::next_available_name;
    use gpui_kit::SharedString;

    #[test]
    fn next_available_name_no_append() {
        let reference_collection = [SharedString::new("old_item")];
        let new_item = next_available_name("new_item", &mut reference_collection.iter());
        assert_eq!(new_item, "new_item");
    }

    #[test]
    fn next_available_name_append() {
        let mut reference_collection = vec![SharedString::new("item")];
        let new_item = next_available_name("item", &mut reference_collection.iter());
        assert_eq!(new_item, "item_1");

        reference_collection.push(SharedString::new(new_item));
        let new_item = next_available_name("item", &mut reference_collection.iter());
        assert_eq!(new_item, "item_2");
    }
}
