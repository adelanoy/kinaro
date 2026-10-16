use std::fmt::Debug;

use crate::test::{FileTestMetadata, test_case::FileTestCase, test_step::FileTestStep};
use serde::{Deserialize, Serialize};

/// A direct child of a suite, tagged by its kind.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum FileSuiteChild {
  /// A test case, holding its own steps
  Case(FileTestCase),
  /// A test step located directly in the suite
  Step(FileTestStep),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestSuite {
  #[serde(flatten)]
  pub info: FileTestMetadata,
  pub children: Vec<FileSuiteChild>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::step_delay::FileDelayStep;
  use crate::test::test_step::FileStepData;
  use ki_utils::time::TimeUnit;
  use serde_json::json;
  use uuid::Uuid;

  fn meta(id: Uuid, name: &str) -> FileTestMetadata {
    FileTestMetadata {
      id,
      name: name.into(),
      description: vec![],
      disabled: false,
    }
  }

  #[test]
  fn children_are_tagged_by_kind() {
    let (suite_id, case_id, step_id) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let step = |id| FileTestStep {
      info: meta(id, "Step"),
      data: FileStepData::Delay(FileDelayStep {
        duration: 10,
        unit: TimeUnit::Second,
      }),
    };
    let suite = FileTestSuite {
      info: meta(suite_id, "Suite"),
      children: vec![
        FileSuiteChild::Case(FileTestCase {
          info: meta(case_id, "Case"),
          steps: vec![step(step_id)],
        }),
        FileSuiteChild::Step(step(step_id)),
      ],
    };

    let value = serde_json::to_value(&suite).unwrap();

    let step_value = json!({ "id": step_id, "name": "Step", "data": { "Delay": { "duration": 10, "unit": "Second" } } });
    assert_eq!(
      value,
      json!({
        "id": suite_id,
        "name": "Suite",
        "children": [
          { "type": "Case", "id": case_id, "name": "Case", "steps": [step_value] },
          { "type": "Step", "id": step_id, "name": "Step", "data": { "Delay": { "duration": 10, "unit": "Second" } } },
        ]
      })
    );
    let parsed: FileTestSuite = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
  }
}
