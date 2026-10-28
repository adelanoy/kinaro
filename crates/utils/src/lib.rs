pub mod conversion;
pub mod date_time;
pub mod shared;
pub mod ui;

pub use conversion::{from_multiline, to_multiline};
pub use date_time::*;
pub use shared::*;
pub use ui::{EditedCell, next_available_name};

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

#[cfg(test)]
mod test {
  use crate::moved_index;

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
