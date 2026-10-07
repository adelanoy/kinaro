use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
  Profile,
  Variable,
  ProjectTree,
}

/// Id-based address of a node in the test tree (Suite → Case → Step).
///
/// A node is addressed by its own id and the ids of all its ancestors rather than by index,
/// so a path stays valid while the tree is being edited (indices shift, ids don't).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TestPath {
  /// A test suite: `Suite(suite_id)`.
  Suite(Uuid),
  /// A test case located in a suite: `Case(suite_id, case_id)`.
  Case(Uuid, Uuid),
  /// A test step located in a case: `Step(suite_id, case_id, step_id)`.
  Step(Uuid, Uuid, Uuid),
}

impl TestPath {
  /// Returns the id of the node this path points to (the last segment of the path).
  pub fn id(&self) -> Uuid {
    match *self {
      TestPath::Suite(id) => id,
      TestPath::Case(_, id) => id,
      TestPath::Step(_, _, id) => id,
    }
  }

  /// Returns `true` if this path points to a test case.
  #[inline]
  pub fn is_case(&self) -> bool {
    matches!(self, TestPath::Case(_, _))
  }

  /// Returns the id of the case this path points to or goes through,
  /// or `None` for a suite path.
  pub fn case_id(&self) -> Option<Uuid> {
    match *self {
      TestPath::Suite(_) => None,
      TestPath::Case(_, id) => Some(id),
      TestPath::Step(_, id, _) => Some(id),
    }
  }

  /// Returns `true` if this path points to a test step.
  #[inline]
  pub fn is_step(&self) -> bool {
    matches!(self, TestPath::Step(_, _, _))
  }

  /// Returns the id of the step this path points to, or `None` for a suite or case path.
  pub fn step_id(&self) -> Option<Uuid> {
    match *self {
      TestPath::Suite(_) | TestPath::Case(_, _) => None,
      TestPath::Step(_, _, id) => Some(id),
    }
  }

  /// Returns `true` if this path points to a test suite.
  #[inline]
  pub fn is_suite(&self) -> bool {
    matches!(self, TestPath::Suite(_))
  }

  /// Returns the id of the suite this path points to or goes through.
  /// Every path has a suite, so this never fails.
  pub fn suite_id(&self) -> Uuid {
    match *self {
      TestPath::Suite(id) => id,
      TestPath::Case(id, _) => id,
      TestPath::Step(id, _, _) => id,
    }
  }

  /// Returns `true` if `other` is this node itself or one of its descendants.
  ///
  /// The relation is reflexive: every path is a parent of itself. Beyond that:
  /// - a suite is a parent of every case and step located in it;
  /// - a case is a parent of every step located in it;
  /// - a step has no descendants.
  pub fn is_parent(&self, other: &TestPath) -> bool {
    match self {
      TestPath::Suite(_) => match other {
        TestPath::Suite(_) => self == other,
        TestPath::Case(suite_id, _) => self.suite_id() == *suite_id,
        TestPath::Step(suite_id, _, _) => self.suite_id() == *suite_id,
      },
      TestPath::Case(_, _) => match other {
        TestPath::Suite(_) => false,
        TestPath::Case(_, _) => self == other,
        TestPath::Step(suite_id, case_id, _) => self.suite_id() == *suite_id && self.case_id() == Some(*case_id),
      },
      TestPath::Step(_, _, _) => match other {
        TestPath::Suite(_) => false,
        TestPath::Case(_, _) => false,
        TestPath::Step(_, _, _) => self == other,
      },
    }
  }
}

/// Formats the path as `Suite:<id>`, `Suite:<id>/Case:<id>` or `Suite:<id>/Case:<id>/Step:<id>`.
impl Display for TestPath {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      TestPath::Suite(suite_id) => f.write_fmt(format_args!("/{}", suite_id)),
      TestPath::Case(suite_id, case_id) => f.write_fmt(format_args!("/{}/{}", suite_id, case_id)),
      TestPath::Step(suite_id, case_id, step_id) => f.write_fmt(format_args!("/{}/{}/{}", suite_id, case_id, step_id)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const SUITE: Uuid = Uuid::from_u128(1);
  const CASE: Uuid = Uuid::from_u128(2);
  const STEP: Uuid = Uuid::from_u128(3);
  const OTHER: Uuid = Uuid::from_u128(4);

  const SUITE_PATH: TestPath = TestPath::Suite(SUITE);
  const CASE_PATH: TestPath = TestPath::Case(SUITE, CASE);
  const STEP_PATH: TestPath = TestPath::Step(SUITE, CASE, STEP);

  #[test]
  fn id_returns_last_segment() {
    assert_eq!(SUITE_PATH.id(), SUITE);
    assert_eq!(CASE_PATH.id(), CASE);
    assert_eq!(STEP_PATH.id(), STEP);
  }

  #[test]
  fn kind_predicates() {
    assert!(SUITE_PATH.is_suite() && !SUITE_PATH.is_case() && !SUITE_PATH.is_step());
    assert!(!CASE_PATH.is_suite() && CASE_PATH.is_case() && !CASE_PATH.is_step());
    assert!(!STEP_PATH.is_suite() && !STEP_PATH.is_case() && STEP_PATH.is_step());
  }

  #[test]
  fn segment_accessors() {
    assert_eq!(SUITE_PATH.suite_id(), SUITE);
    assert_eq!(CASE_PATH.suite_id(), SUITE);
    assert_eq!(STEP_PATH.suite_id(), SUITE);

    assert_eq!(SUITE_PATH.case_id(), None);
    assert_eq!(CASE_PATH.case_id(), Some(CASE));
    assert_eq!(STEP_PATH.case_id(), Some(CASE));

    assert_eq!(SUITE_PATH.step_id(), None);
    assert_eq!(CASE_PATH.step_id(), None);
    assert_eq!(STEP_PATH.step_id(), Some(STEP));
  }

  #[test]
  fn is_parent_is_reflexive() {
    assert!(SUITE_PATH.is_parent(&SUITE_PATH));
    assert!(CASE_PATH.is_parent(&CASE_PATH));
    assert!(STEP_PATH.is_parent(&STEP_PATH));
  }

  #[test]
  fn suite_is_parent_of_its_descendants_only() {
    assert!(SUITE_PATH.is_parent(&CASE_PATH));
    assert!(SUITE_PATH.is_parent(&STEP_PATH));

    assert!(!SUITE_PATH.is_parent(&TestPath::Suite(OTHER)));
    assert!(!SUITE_PATH.is_parent(&TestPath::Case(OTHER, CASE)));
    assert!(!SUITE_PATH.is_parent(&TestPath::Step(OTHER, CASE, STEP)));
  }

  #[test]
  fn case_is_parent_of_its_steps_only() {
    assert!(CASE_PATH.is_parent(&STEP_PATH));

    assert!(!CASE_PATH.is_parent(&SUITE_PATH));
    assert!(!CASE_PATH.is_parent(&TestPath::Case(SUITE, OTHER)));
    assert!(!CASE_PATH.is_parent(&TestPath::Step(SUITE, OTHER, STEP)));
    // Same case id, but in another suite
    assert!(!CASE_PATH.is_parent(&TestPath::Case(OTHER, CASE)));
    assert!(!CASE_PATH.is_parent(&TestPath::Step(OTHER, CASE, STEP)));
  }

  #[test]
  fn step_is_parent_of_itself_only() {
    assert!(!STEP_PATH.is_parent(&SUITE_PATH));
    assert!(!STEP_PATH.is_parent(&CASE_PATH));
    assert!(!STEP_PATH.is_parent(&TestPath::Step(SUITE, CASE, OTHER)));
    assert!(!STEP_PATH.is_parent(&TestPath::Step(SUITE, OTHER, STEP)));
  }

  #[test]
  fn display() {
    assert_eq!(SUITE_PATH.to_string(), format!("/{SUITE}"));
    assert_eq!(CASE_PATH.to_string(), format!("/{SUITE}/{CASE}"));
    assert_eq!(STEP_PATH.to_string(), format!("/{SUITE}/{CASE}/{STEP}"));
  }
}
