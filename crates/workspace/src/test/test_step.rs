use crate::test::{TestInfo, TestInfoId};
use gpui_kit::SharedString;
use ki_project::FileTestStep;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestStep {
  pub info: TestInfo,
  pub data: String,
}

impl TestStep {
  pub(super) fn duplicate(&self, name: SharedString) -> Self {
    Self {
      info: self.info.duplicate(name),
      data: self.data.clone(),
    }
  }
  
  pub fn from_file(file_test_step: FileTestStep, suite_id: Uuid, case_id: Uuid) -> TestStep {
    Self {
        info: TestInfo::new_step(file_test_step.info, suite_id, case_id),
        data: file_test_step.data.clone(),
      }
  }
  pub fn get_file(&self) -> FileTestStep {
    FileTestStep {
      info: self.info.to_file(),
      data: self.data.clone(),
    }
  }
  
  pub fn new(suite_id: Uuid, case_id: Uuid, name: SharedString) -> Self {
    Self {
      info: TestInfo {
        info_id: TestInfoId::Step(suite_id, case_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      data: "".to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::test::TestInfoId;
  use crate::test::test_step::TestStep;
  use gpui_kit::SharedString;
  use ki_project::{FileTestInfo, FileTestStep};
  use uuid::Uuid;

  /// A step built through [`TestStep::from_file`] so that every id is known
  /// upfront and the step carries non-default data/description/disabled
  /// values, to make sure they all survive the round trip.
  struct Fixture {
    step: TestStep,
    suite_id: Uuid,
    case_id: Uuid,
    step_id: Uuid,
  }

  fn file_step(id: Uuid, name: &str, data: &str) -> FileTestStep {
    FileTestStep {
      info: FileTestInfo {
        id,
        name: name.to_string(),
        description: vec!["a description".to_string(), "with two lines".to_string()],
        disabled: true,
      },
      data: data.to_string(),
    }
  }

  fn fixture() -> Fixture {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();

    let step = TestStep::from_file(file_step(step_id, "step a", "step data"), suite_id, case_id);

    Fixture {
      step,
      suite_id,
      case_id,
      step_id,
    }
  }

  #[test]
  fn from_file_builds_step_with_expected_info() {
    let f = fixture();
    assert_eq!(f.step.info.name, SharedString::new("step a"));
    assert_eq!(f.step.info.description, Some(SharedString::new("a description\nwith two lines")));
    assert!(f.step.info.disabled);
    assert_eq!(f.step.data, "step data");
    assert!(
      matches!(f.step.info.info_id(), TestInfoId::Step(suite, case, step) if suite == f.suite_id && case == f.case_id && step == f.step_id)
    );
  }

  #[test]
  fn get_file_round_trips() {
    let f = fixture();
    let file = f.step.get_file();
    let rebuilt = TestStep::from_file(file, f.suite_id, f.case_id);

    assert_eq!(rebuilt, f.step);
  }

  #[test]
  fn new_creates_step_with_generated_id_and_empty_data() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step = TestStep::new(suite_id, case_id, SharedString::new("new step"));

    assert_eq!(step.info.name, SharedString::new("new step"));
    assert_eq!(step.info.description, None);
    assert!(!step.info.disabled);
    assert_eq!(step.data, "");
    assert!(matches!(step.info.info_id(), TestInfoId::Step(suite, case, _) if suite == suite_id && case == case_id));
  }

  #[test]
  fn duplicate_generates_a_fresh_id_but_keeps_suite_and_case() {
    let f = fixture();
    let dup = f.step.duplicate(SharedString::new("step a copy"));

    assert!(
      matches!(dup.info.info_id(), TestInfoId::Step(suite, case, step) if suite == f.suite_id && case == f.case_id && step != f.step_id)
    );
  }

  #[test]
  fn duplicate_uses_the_given_name_but_keeps_description_and_disabled() {
    let f = fixture();
    let dup = f.step.duplicate(SharedString::new("step a copy"));

    assert_eq!(dup.info.name, SharedString::new("step a copy"));
    assert_eq!(dup.info.description, f.step.info.description);
    assert_eq!(dup.info.disabled, f.step.info.disabled);
  }

  #[test]
  fn duplicate_preserves_data() {
    let f = fixture();
    let dup = f.step.duplicate(SharedString::new("step a copy"));

    assert_eq!(dup.data, f.step.data);
  }

  #[test]
  fn eq_is_true_for_an_identical_clone() {
    let f = fixture();
    assert_eq!(f.step, f.step.clone());
  }

  #[test]
  fn eq_is_false_when_only_the_name_differs() {
    let f = fixture();
    let mut same_id_other_name = f.step.clone();
    same_id_other_name.info.name = SharedString::new("a totally different name");
    assert_ne!(f.step, same_id_other_name);
  }

  #[test]
  fn eq_is_false_when_only_the_id_differs() {
    let f = fixture();
    let other_id = TestStep::new(f.suite_id, f.case_id, f.step.info.name.clone());
    assert_ne!(f.step, other_id);
  }
}