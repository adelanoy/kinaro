use ki_utils::time::TimeUnit;
use serde::{Deserialize, Serialize};

/// On-disk form of a delay step: waits for `duration` expressed in `unit`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileDelayStep {
  /// How long to wait, in `unit`
  pub duration: u32,
  /// The time unit `duration` is expressed in
  pub unit: TimeUnit,
}
