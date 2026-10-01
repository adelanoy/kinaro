pub mod conversion;
pub mod date_time;
pub mod shared;
pub mod ui;

pub use conversion::{from_multiline, to_multiline};
pub use date_time::*;
pub use shared::*;
pub use ui::{CellState, next_available_name};

/// Represent an offset forward one direction
pub enum Offset {
  Plus,
  Minus,
}
