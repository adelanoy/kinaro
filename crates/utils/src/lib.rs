pub mod conversion;
pub mod date_time;
pub mod shared;
pub mod ui;

pub use conversion::{from_multiline, to_multiline};
pub use date_time::*;
pub use shared::*;
pub use ui::{CellState, next_available_name};

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
