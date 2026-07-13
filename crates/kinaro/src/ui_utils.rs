use gpui::SharedString;
use std::sync::LazyLock;

pub static EMPTY_SHARED_STRING: LazyLock<SharedString> = LazyLock::new(|| SharedString::new(""));

pub enum CellState<T> {
    Unselected,
    RowSelected(usize),
    CellSelected(usize),
    CellEdited(usize, usize, T),
}

/// Computes a name based on the given name, appended with '_X' where X is an integer and where the *name_X* is the first available in the collection.
///
/// X is incremented at each attempt
///
/// If the name is available without any suffix in the collection, returns *name*
pub fn next_available_name<'a>(
    name: &str,
    collection: &mut (impl Iterator<Item = &'a SharedString> + Clone)) -> SharedString {

    if let None = collection.clone().find(|p| p.as_ref() == name) {
        return SharedString::new(name);
    }

    let mut final_name = name.to_string();
    {
        let mut i = 1;
        loop {
            if collection.any(|p| p.as_ref() == &final_name) {
                final_name = format!("{}_{}", name, i);
                i += 1;
            } else {
                break;
            }
        }
    }
    SharedString::new(final_name)
}

#[cfg(test)]
mod test {
    use gpui::SharedString;
    use crate::ui_utils::next_available_name;

    #[test]
    fn next_available_name_no_append() {
        let reference_collection = vec![SharedString::new("old_item")];
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
