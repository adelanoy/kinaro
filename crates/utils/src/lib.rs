pub mod conversion;
pub mod date_time;
pub mod shared;
pub mod http;

pub use conversion::{from_multiline, to_multiline};
pub use date_time::*;
use gpui_kit::SharedString;
pub use shared::*;

/// Clipboard metadata of a copied test suite
pub const SUITE: &str = "Suite";
/// Clipboard metadata of a copied test case
pub const CASE: &str = "Case";
/// Clipboard metadata of a copied test step (located in a suite or in a case)
pub const STEP: &str = "Step";

/// Represent an offset forward one direction
pub enum Offset {
  Plus,
  Minus,
}

/// Returns the index where the item at `ix` ends up once the item at `from` is moved to `to`, the
/// items in between shifting by one to fill the gap.
pub fn moved_index(ix: usize, from: usize, to: usize) -> usize {
  match ix {
    _ if ix == from => to,
    _ if from < ix && ix <= to => ix - 1,
    _ if to <= ix && ix < from => ix + 1,
    _ => ix,
  }
}

/// Computes a name based on the given name, appended with '_X' where X is an integer and where the `name_X` is the first available in the collection.
///
/// X is incremented at each attempt
///
/// If the name is available without any suffix in the collection, returns *name*
pub fn next_available_name(name: &str, mut collection_it: impl Iterator<Item = SharedString> + Clone) -> SharedString {
  // clone the iterator so it can be used a second time after
  if collection_it.clone().find(|p| p == name).is_none() {
    return SharedString::new(name);
  }

  let mut final_name = name.to_string();
  {
    let mut i = 1;
    loop {
      if collection_it.any(|p| *p == final_name) {
        final_name = format!("{name}_{i}");
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
  use crate::{moved_index, next_available_name};
  use gpui_kit::SharedString;

  #[test]
  fn next_available_name_no_append() {
    let reference_collection = [SharedString::new("old_item")];
    let new_item = next_available_name("new_item", reference_collection.into_iter());
    assert_eq!(new_item, "new_item");
  }

  #[test]
  fn next_available_name_append() {
    let mut reference_collection = vec![SharedString::new("item")];
    let new_item = next_available_name("item", reference_collection.clone().into_iter());
    assert_eq!(new_item, "item_1");

    reference_collection.push(SharedString::new(new_item));
    let new_item = next_available_name("item", reference_collection.into_iter());
    assert_eq!(new_item, "item_2");
  }

  #[test]
  fn moved_index_forward() {
    // [a, b, c, d] moving b (1) to 3 gives [a, c, d, b]
    assert_eq!([0, 1, 2, 3].map(|ix| moved_index(ix, 1, 3)), [0, 3, 1, 2]);
  }

  #[test]
  fn moved_index_backward() {
    // [a, b, c, d] moving d (3) to 1 gives [a, d, b, c]
    assert_eq!([0, 1, 2, 3].map(|ix| moved_index(ix, 3, 1)), [0, 2, 3, 1]);
  }

  #[test]
  fn moved_index_in_place() {
    assert_eq!([0, 1, 2].map(|ix| moved_index(ix, 1, 1)), [0, 1, 2]);
  }
}
