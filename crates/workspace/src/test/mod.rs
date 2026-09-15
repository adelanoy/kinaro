use crate::error::{ProjectError::TestNotFound, ProjectResult};
use crate::{FileProjectMetadata, TestSuite};
use gpui_kit::{Context, EventEmitter, SharedString};
use ki_project::{FileTestInfo, FileTestsContainer};
use log::warn;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

pub mod test_case;
pub mod test_step;
pub mod test_suite;

/// Enum wrapping the type of test Item (Suite, Case or Step), it's id and it's path
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TestInfoId {
  Suite(Uuid),
  CaseMulti(Uuid, Uuid),
  CaseStep(Uuid, Uuid),
  Step(Uuid, Uuid, Uuid),
}

impl TestInfoId {
  pub fn id(&self) -> Uuid {
    match *self {
      TestInfoId::Suite(id) => id,
      TestInfoId::CaseMulti(_, id) | TestInfoId::CaseStep(_, id) => id,
      TestInfoId::Step(_, _, id) => id,
    }
  }

  pub fn suite_id(&self) -> Uuid {
    match *self {
      TestInfoId::Suite(id) => id,
      TestInfoId::CaseMulti(id, _) | TestInfoId::CaseStep(id, _) => id,
      TestInfoId::Step(id, _, _) => id,
    }
  }

  pub fn case_id(&self) -> Option<Uuid> {
    match *self {
      TestInfoId::Suite(_) => None,
      TestInfoId::CaseMulti(_, id) | TestInfoId::CaseStep(_, id) => Some(id),
      TestInfoId::Step(_, id, _) => Some(id),
    }
  }

  pub fn step_id(&self) -> Option<Uuid> {
    match *self {
      TestInfoId::Suite(_) => None,
      TestInfoId::CaseMulti(_, _) | TestInfoId::CaseStep(_, _) => None,
      TestInfoId::Step(_, _, id) => Some(id),
    }
  }

  pub fn is_parent(&self, other: &TestInfoId) -> bool {
    match self {
      TestInfoId::Suite(_) => match other {
        TestInfoId::Suite(_) => self == other,
        TestInfoId::CaseMulti(suite_id, _) | TestInfoId::CaseStep(suite_id, _) => self.suite_id() == *suite_id,
        TestInfoId::Step(suite_id, _, _) => self.suite_id() == *suite_id,
      },
      TestInfoId::CaseMulti(_, _) => match other {
        TestInfoId::Suite(_) => false,
        TestInfoId::CaseMulti(_, _) => self == other,
        TestInfoId::CaseStep(_, _) => false,
        TestInfoId::Step(suite_id, case_id, _) => self.suite_id() == *suite_id && self.case_id() == Some(*case_id),
      },
      TestInfoId::CaseStep(_, _) => match other {
        TestInfoId::Suite(_) => false,
        TestInfoId::CaseMulti(_, _) => false,
        TestInfoId::CaseStep(_, _) => self == other,
        TestInfoId::Step(_, _, _) => false,
      },
      TestInfoId::Step(_, _, _) => match other {
        TestInfoId::Suite(_) => false,
        TestInfoId::CaseMulti(_, _) | TestInfoId::CaseStep(_, _) => false,
        TestInfoId::Step(_, _, _) => self == other,
      },
    }
  }
}

impl Display for TestInfoId {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      TestInfoId::Suite(suite_id) => f.write_fmt(format_args!("Suite:{}", suite_id)),
      TestInfoId::CaseMulti(suite_id, case_id) | TestInfoId::CaseStep(suite_id, case_id) => {
        f.write_fmt(format_args!("Suite:{}/Case:{}", suite_id, case_id))
      }
      TestInfoId::Step(suite_id, case_id, step_id) => {
        f.write_fmt(format_args!("Suite:{}/Case:{}/Step:{}", suite_id, case_id, step_id))
      }
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestInfo {
  info_id: TestInfoId,
  pub name: SharedString,
  pub description: Option<SharedString>,
  pub disabled: bool,
}

impl TestInfo {
  pub fn id(&self) -> Uuid {
    self.info_id.id()
  }

  pub fn info_id(&self) -> TestInfoId {
    self.info_id
  }

  pub(crate) fn new_suite(file_test_info: FileTestInfo) -> Self {
    Self {
      info_id: TestInfoId::Suite(file_test_info.id),
      name: file_test_info.name.into(),
      description: file_test_info.description.map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  pub(crate) fn new_case(file_test_info: FileTestInfo, suite_id: Uuid) -> Self {
    Self {
      info_id: TestInfoId::CaseMulti(suite_id, file_test_info.id),
      name: file_test_info.name.into(),
      description: file_test_info.description.map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  pub(crate) fn new_case_step(file_test_info: FileTestInfo, suite_id: Uuid) -> Self {
    Self {
      info_id: TestInfoId::CaseStep(suite_id, file_test_info.id),
      name: file_test_info.name.into(),
      description: file_test_info.description.map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  pub(crate) fn new_step(file_test_info: FileTestInfo, suite_id: Uuid, case_id: Uuid) -> Self {
    Self {
      info_id: TestInfoId::Step(suite_id, case_id, file_test_info.id),
      name: file_test_info.name.into(),
      description: file_test_info.description.map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  pub(crate) fn duplicate(&self, name: SharedString) -> Self {
    let info_id = match self.info_id {
      TestInfoId::Suite(_) => TestInfoId::Suite(Uuid::new_v4()),
      TestInfoId::CaseMulti(suite_id, _) => TestInfoId::CaseMulti(suite_id, Uuid::new_v4()),
      TestInfoId::CaseStep(suite_id, _) => TestInfoId::CaseStep(suite_id, Uuid::new_v4()),
      TestInfoId::Step(suite_id, case_id, _) => TestInfoId::Step(suite_id, case_id, Uuid::new_v4()),
    };
    Self {
      info_id,
      name,
      description: self.description.clone(),
      disabled: self.disabled,
    }
  }

  pub(crate) fn to_file(&self) -> FileTestInfo {
    FileTestInfo {
      id: self.id(),
      name: self.name.to_string(),
      description: self.description.as_ref().map(|s| s.to_string()),
      disabled: self.disabled,
    }
  }
}

pub enum TestsContainerEvent {
  TestsModified,
  TreeNodesChanged,
}

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct TestsContainer {
  pub suites: Vec<TestSuite>,
  opened_tree_nodes: HashSet<Uuid>,
}

impl TestsContainer {
  pub fn add_test_case(&mut self, info_id: &TestInfoId, cx: &mut Context<Self>) -> ProjectResult<()> {
    let suite_id = info_id.suite_id();
    let Some(suite) = self.suites.iter_mut().find(|suite| suite.info.id() == suite_id) else {
      warn!("TestsContainer:add_test_case: unknow path: {}", info_id);
      return Err(TestNotFound);
    };
    suite.add_test_case(info_id);

    cx.emit(TestsContainerEvent::TestsModified);
    Ok(())
  }

  pub fn add_test_step(&mut self, info_id: &TestInfoId, cx: &mut Context<Self>) -> ProjectResult<()> {
    let suite_id = info_id.suite_id();
    let Some(suite) = self.suites.iter_mut().find(|suite| suite.info.id() == suite_id) else {
      warn!("TestsContainer:add_test_step: unknow path: {}", info_id);
      return Err(TestNotFound);
    };
    suite.add_test_step(info_id)?;

    cx.emit(TestsContainerEvent::TestsModified);
    Ok(())
  }

  pub fn add_test_suite(&mut self, info_id: Option<&TestInfoId>, cx: &mut Context<Self>) {
    let position = match info_id {
      None => None,
      Some(info_id) => {
        let suite_id = info_id.suite_id();
        self.suites.iter().position(|suite| suite.info.id() == suite_id)
      }
    };

    let name = ki_utils::next_available_name("New Suite", self.suites.iter().map(|suite| suite.info.name.clone()));
    match position {
      Some(ix) => self.suites.insert(ix + 1, TestSuite::new(name)),
      None => self.suites.push(TestSuite::new(name)),
    }
    cx.emit(TestsContainerEvent::TestsModified);
  }

  pub fn collapse_tree_node(&mut self, id: &Uuid, cx: &mut Context<Self>) {
    self.opened_tree_nodes.remove(id);
    cx.emit(TestsContainerEvent::TreeNodesChanged);
  }

  pub fn duplicate_test(&mut self, info_id: &TestInfoId, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(ix) = self.suites.iter().position(|suite| suite.info.id() == info_id.suite_id()) else {
      warn!("TestsContainer:duplicate_test: unknow path: {}", info_id);
      return Err(TestNotFound);
    };

    if matches!(info_id, TestInfoId::Suite(_)) {
      let new_name = ki_utils::next_available_name(
        &self.suites[ix].info.name,
        self.suites.iter().map(|suite| suite.info.name.clone()),
      );
      let duplicate = self.suites[ix].duplicate(new_name);
      if ix == self.suites.len() - 1 {
        self.suites.push(duplicate);
      } else {
        self.suites.insert(ix + 1, duplicate);
      }
    } else {
      self.suites[ix].duplicate_child(info_id)?;
    }
    cx.emit(TestsContainerEvent::TestsModified);
    Ok(())
  }

  pub fn expand_tree_node(&mut self, id: Uuid, cx: &mut Context<Self>) {
    self.opened_tree_nodes.insert(id);
    cx.emit(TestsContainerEvent::TreeNodesChanged);
  }

  pub(super) fn from_file(file_container: FileTestsContainer, metadata: &FileProjectMetadata) -> Self {
    Self {
      suites: file_container.suites.into_iter().map(TestSuite::from_file).collect(),
      opened_tree_nodes: metadata.opened_tree_nodes.clone(),
    }
  }

  #[allow(unused)]
  pub fn info_from_path(&self, info_id: &TestInfoId) -> Option<&TestInfo> {
    let suite = self.suites.iter().find(|suite| suite.info.info_id == *info_id)?;
    match info_id {
      TestInfoId::Suite(_) => Some(&suite.info),
      _ => suite.info_from_path(info_id),
    }
  }

  pub fn info_mut_from_path(&mut self, info_id: &TestInfoId) -> Option<&mut TestInfo> {
    let suite = self.suites.iter_mut().find(|suite| suite.info.info_id == *info_id)?;
    match info_id {
      TestInfoId::Suite(_) => Some(&mut suite.info),
      _ => suite.info_mut_from_path(info_id),
    }
  }

  #[inline]
  pub fn opened_tree_nodes(&self) -> &HashSet<Uuid> {
    &self.opened_tree_nodes
  }

  pub fn rename_at(&mut self, info_id: &TestInfoId, name: SharedString, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(test_info) = self.info_mut_from_path(info_id) else {
      warn!("TestsContainer:rename_at: unknow path: {}", info_id);
      return Err(TestNotFound);
    };

    if test_info.name != name {
      test_info.name = name;
      cx.emit(TestsContainerEvent::TestsModified);
    }

    Ok(())
  }

  pub fn switch_node_enable_status(&mut self, info_id: &TestInfoId, cx: &mut Context<Self>) {
    let Some(info) = self.info_mut_from_path(info_id) else {
      warn!("TestsContainer:switch_active_status: unknow path: {}", info_id);
      return;
    };
    info.disabled = !info.disabled;
    cx.emit(TestsContainerEvent::TestsModified);
  }

  pub fn to_file(&self) -> FileTestsContainer {
    FileTestsContainer {
      suites: self.suites.iter().map(|suite| suite.get_file()).collect(),
    }
  }

  /*pub fn move_test(&mut self, action: &DragAndDrop<Vec<Uuid>>) -> Result<()> {
      if action.target.len() == 1 {
          self.move_to_suite(action)
      } else {
          self.move_to_case(action)
      }
  }

  fn move_to_suite(&mut self, action: &DragAndDrop<Vec<Uuid>>) -> Result<()> {
      let target_path = self
          .get_indices(&action.target)
          .map_err(|_| WorkspaceError::from(TestError::InvalidMoveTarget))?;

      for source in action.source.iter() {
          let source_path = self.get_indices(source);
          if source_path.is_err() {
              continue;
          }
          let source_path = source_path?;
          match source_path.len() {
              2 => {
                  let case = self[source_path[0]].cases.remove(source_path[1]);
                  self.attach_case_to_suite(target_path[0], &action.position, case);
              }
              3 => {
                  if self[source_path[0]].cases[source_path[1]].is_anonymous {
                      let case = self[source_path[0]].cases.remove(source_path[1]);
                      self.attach_case_to_suite(target_path[0], &action.position, case);
                  } else {
                      let step = self[source_path[0]].cases[source_path[1]]
                          .steps
                          .remove(source_path[2]);
                      let case = WorkspaceTestCase {
                          info: WorkspaceTestInfo {
                              id: Uuid::new_v4(),
                              name: "".into(),
                              description: None,
                              active: true,
                          },
                          is_anonymous: true,
                          steps: vec![step],
                      };
                      self.attach_case_to_suite(target_path[0], &action.position, case);
                  }
              }
              _ => {}
          }
      }

      Ok(())
  }

  fn attach_case_to_suite(
      &mut self,
      suite_index: usize,
      position: &DirPosition<Vec<Uuid>>,
      case: WorkspaceTestCase,
  ) {
      match position {
          DirPosition::First => self[suite_index].cases.insert(0, case),
          DirPosition::Last => self[suite_index].cases.push(case),
          DirPosition::After(path) => {
              if let Some(case_index) = self[suite_index]
                  .cases
                  .iter()
                  .position(|case| case.info.id == path[1])
              {
                  if case_index >= self[suite_index].cases.len() {
                      self[suite_index].cases.push(case);
                  } else {
                      self[suite_index].cases.insert(case_index, case);
                  }
              }
          }
          DirPosition::Before(path) => {
              if let Some(case_index) = self[suite_index]
                  .cases
                  .iter()
                  .position(|case| case.info.id == path[1])
              {
                  self[suite_index].cases.insert(case_index, case);
              }
          }
      }
  }

  fn move_to_case(&mut self, action: &DragAndDrop<Vec<Uuid>>) -> Result<()> {
      let target = self
          .get_indices(&action.target)
          .map_err(|_| WorkspaceError::from(TestError::InvalidMoveTarget))?;

      for source in action.source.iter() {
          let source_path = self.get_indices(source);
          if source_path.is_err() {
              continue;
          }
          let source_path = source_path?;
          if source_path.len() != 3 {
              continue;
          }
          let step = self[source_path[0]].cases[source_path[1]]
              .steps
              .remove(source_path[2]);
          match &action.position {
              DirPosition::First => self[target[0]].cases[target[1]]
                  .steps
                  .insert(0, step),
              DirPosition::Last => self[target[0]].cases[target[1]].steps.push(step),
              DirPosition::After(path) => {
                  if let Some(step_index) = self[target[0]].cases[target[1]]
                      .steps
                      .iter()
                      .position(|step| step.info.id == path[2])
                  {
                      if step_index >= self[target[0]].cases[target[1]].steps.len() {
                          self[target[0]].cases[target[1]].steps.push(step);
                      } else {
                          self[target[0]].cases[target[1]]
                              .steps
                              .insert(step_index, step);
                      }
                  }
              }
              DirPosition::Before(path) => {
                  if let Some(step_index) = self[target[0]].cases[target[1]]
                      .steps
                      .iter()
                      .position(|step| step.info.id == path[2])
                  {
                      self[target[0]].cases[target[1]]
                          .steps
                          .insert(step_index, step);
                  }
              }
          }
      }

      Ok(())
  }

  fn get_indices(&self, path: &Vec<Uuid>) -> Result<Vec<usize>> {
      if path.is_empty() {
          return Err(WorkspaceError::from(TestError::InvalidMoveSource));
      }
      let mut positions = vec![0; path.len()];
      let suite = self

          .iter()
          .position(|suite| suite.info.id == path[0])
          .ok_or(WorkspaceError::from(TestError::InvalidMoveSource))?;
      positions.push(suite);
      if path.len() > 1 {
          let case = self[suite]
              .cases
              .iter()
              .position(|case| case.info.id == path[1])
              .ok_or(WorkspaceError::from(TestError::InvalidMoveSource))?;
          positions.push(case);
          if path.len() > 2 {
              let step = self[suite].cases[case]
                  .steps
                  .iter()
                  .position(|step| step.info.id == path[2])
                  .ok_or(WorkspaceError::from(TestError::InvalidMoveSource))?;
              positions.push(step);
          }
      }
      Ok(positions)
  }*/
}

impl EventEmitter<TestsContainerEvent> for TestsContainer {}

#[cfg(test)]
mod tests {
  use crate::test::{TestInfo, TestInfoId};
  use gpui_kit::SharedString;
  use ki_project::FileTestInfo;
  use uuid::Uuid;

  #[test]
  fn test_info_id_id() {
    let id = Uuid::new_v4();
    // id
    let info_id = TestInfoId::Suite(id);
    assert_eq!(info_id.id(), id);
    let info_id = TestInfoId::CaseMulti(Uuid::new_v4(), id);
    assert_eq!(info_id.id(), id);
    let info_id = TestInfoId::CaseStep(Uuid::new_v4(), id);
    assert_eq!(info_id.id(), id);
    let info_id = TestInfoId::Step(Uuid::new_v4(), Uuid::new_v4(), id);
    assert_eq!(info_id.id(), id);
  }

  #[test]
  fn test_info_id_test_suite_id() {
    let id = Uuid::new_v4();
    // test_suite id
    let info_id = TestInfoId::Suite(id);
    assert_eq!(info_id.suite_id(), id);
    let info_id = TestInfoId::CaseMulti(id, Uuid::new_v4());
    assert_eq!(info_id.suite_id(), id);
    let info_id = TestInfoId::CaseStep(id, Uuid::new_v4());
    assert_eq!(info_id.suite_id(), id);
    let info_id = TestInfoId::Step(id, Uuid::new_v4(), Uuid::new_v4());
    assert_eq!(info_id.suite_id(), id);
  }

  #[test]
  fn test_info_id_test_case_id() {
    let id = Uuid::new_v4();
    // test_suite id
    let info_id = TestInfoId::Suite(id);
    assert_eq!(info_id.case_id(), None);
    let info_id = TestInfoId::CaseMulti(Uuid::new_v4(), id);
    assert_eq!(info_id.case_id(), Some(id));
    let info_id = TestInfoId::CaseStep(Uuid::new_v4(), id);
    assert_eq!(info_id.case_id(), Some(id));
    let info_id = TestInfoId::Step(Uuid::new_v4(), id, Uuid::new_v4());
    assert_eq!(info_id.case_id(), Some(id));
  }

  #[test]
  fn test_info_id_test_step_id() {
    let id = Uuid::new_v4();
    // test_suite id
    let info_id = TestInfoId::Suite(id);
    assert_eq!(info_id.step_id(), None);
    let info_id = TestInfoId::CaseMulti(Uuid::new_v4(), id);
    assert_eq!(info_id.step_id(), None);
    let info_id = TestInfoId::CaseStep(Uuid::new_v4(), id);
    assert_eq!(info_id.step_id(), None);
    let info_id = TestInfoId::Step(Uuid::new_v4(), Uuid::new_v4(), id);
    assert_eq!(info_id.step_id(), Some(id));
  }

  #[test]
  fn test_info_id_is_parent_suite() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let info_id = TestInfoId::Suite(suite_id);
    // Assert parent
    assert!(info_id.is_parent(&info_id));
    assert!(info_id.is_parent(&TestInfoId::CaseMulti(suite_id, case_id)));
    assert!(info_id.is_parent(&TestInfoId::CaseStep(suite_id, case_id)));
    assert!(info_id.is_parent(&TestInfoId::Step(suite_id, case_id, step_id)));
    // Assert not parent
    assert!(!info_id.is_parent(&TestInfoId::Suite(Uuid::new_v4())));
    assert!(!info_id.is_parent(&TestInfoId::CaseMulti(Uuid::new_v4(), case_id)));
    assert!(!info_id.is_parent(&TestInfoId::CaseStep(Uuid::new_v4(), case_id)));
    assert!(!info_id.is_parent(&TestInfoId::Step(Uuid::new_v4(), case_id, step_id)));
  }

  #[test]
  fn test_info_id_is_parent_case() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let info_id = TestInfoId::CaseMulti(suite_id, case_id);
    // Assert parent
    assert!(info_id.is_parent(&TestInfoId::CaseMulti(suite_id, case_id)));
    assert!(info_id.is_parent(&TestInfoId::Step(suite_id, case_id, step_id)));
    // Assert not parent
    assert!(!info_id.is_parent(&TestInfoId::Suite(suite_id)));
    assert!(!info_id.is_parent(&TestInfoId::CaseMulti(suite_id, Uuid::new_v4())));
    assert!(!info_id.is_parent(&TestInfoId::CaseMulti(Uuid::new_v4(), case_id)));
    assert!(!info_id.is_parent(&TestInfoId::Step(Uuid::new_v4(), case_id, step_id)));
  }

  #[test]
  fn test_info_id_is_parent_case_step() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let info_id = TestInfoId::CaseStep(suite_id, case_id);
    // Assert parent
    assert!(info_id.is_parent(&TestInfoId::CaseStep(suite_id, case_id)));
    // Assert not parent
    assert!(!info_id.is_parent(&TestInfoId::Suite(suite_id)));
    assert!(!info_id.is_parent(&TestInfoId::CaseMulti(suite_id, case_id)));
    assert!(!info_id.is_parent(&TestInfoId::Step(suite_id, case_id, step_id)));
  }

  #[test]
  fn test_info_id_is_parent_step() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let info_id = TestInfoId::Step(suite_id, case_id, step_id);
    // Assert parent
    assert!(info_id.is_parent(&TestInfoId::Step(suite_id, case_id, step_id)));
    // Assert not parent
    assert!(!info_id.is_parent(&TestInfoId::Suite(suite_id)));
    assert!(!info_id.is_parent(&TestInfoId::CaseMulti(suite_id, Uuid::new_v4())));
    assert!(!info_id.is_parent(&TestInfoId::Step(suite_id, case_id, Uuid::new_v4())));
    assert!(!info_id.is_parent(&TestInfoId::Step(suite_id, Uuid::new_v4(), step_id)));
  }

  #[test]
  fn test_info_id() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();

    let mut info = TestInfo {
      info_id: TestInfoId::Suite(suite_id),
      name: Default::default(),
      description: None,
      disabled: false,
    };
    assert_eq!(suite_id, info.id());
    info.info_id = TestInfoId::CaseMulti(suite_id, case_id);
    assert_eq!(case_id, info.id());
    info.info_id = TestInfoId::CaseStep(suite_id, case_id);
    assert_eq!(case_id, info.id());
    info.info_id = TestInfoId::Step(suite_id, case_id, step_id);
    assert_eq!(step_id, info.id());
  }

  #[test]
  fn test_info_new_suite() {
    let suite_id = Uuid::new_v4();
    let file_info = FileTestInfo {
      id: suite_id,
      name: suite_id.to_string(),
      description: Some(suite_id.to_string()),
      disabled: true,
    };

    let info = TestInfo::new_suite(file_info);
    match info.info_id() {
      TestInfoId::Suite(suite) if suite == suite_id => {}
      _ => panic!("TestInfo::new_suite failed with non-matching ids"),
    }
    assert_eq!(suite_id, info.id());
    assert_eq!(format!("{}", suite_id), info.name.to_string());
    assert_eq!(format!("{}", suite_id), info.description.map(|s| s.to_string()).unwrap());
    assert!(info.disabled);
  }

  #[test]
  fn test_info_new_case() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let file_info = FileTestInfo {
      id: case_id,
      name: case_id.to_string(),
      description: Some(case_id.to_string()),
      disabled: true,
    };

    let info = TestInfo::new_case(file_info, suite_id);
    match info.info_id() {
      TestInfoId::CaseMulti(suite, case) if suite == suite_id && case == case_id => {}
      _ => panic!("TestInfo::new_suite failed with non-matching ids"),
    }
    assert_eq!(case_id, info.id());
    assert_eq!(format!("{}", case_id), info.name.to_string());
    assert_eq!(format!("{}", case_id), info.description.map(|s| s.to_string()).unwrap());
    assert!(info.disabled);
    assert!(matches!(info.info_id, TestInfoId::CaseMulti(_, _)));
  }

  #[test]
  fn test_info_new_case_step() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let file_info = FileTestInfo {
      id: case_id,
      name: case_id.to_string(),
      description: Some(case_id.to_string()),
      disabled: true,
    };

    let info = TestInfo::new_case_step(file_info, suite_id);
    match info.info_id() {
      TestInfoId::CaseStep(suite, case) if suite == suite_id && case == case_id => {}
      _ => panic!("TestInfo::new_suite failed with non-matching ids"),
    }
    assert_eq!(case_id, info.id());
    assert_eq!(format!("{}", case_id), info.name.to_string());
    assert_eq!(format!("{}", case_id), info.description.map(|s| s.to_string()).unwrap());
    assert!(info.disabled);
    assert!(matches!(info.info_id, TestInfoId::CaseStep(_, _)));
  }

  #[test]
  fn test_info_new_step() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let file_info = FileTestInfo {
      id: step_id,
      name: step_id.to_string(),
      description: Some(step_id.to_string()),
      disabled: true,
    };

    let info = TestInfo::new_step(file_info, suite_id, case_id);
    match info.info_id() {
      TestInfoId::Step(suite, case, step) if suite == suite_id && case == case_id && step == step_id => {}
      _ => panic!("TestInfo::new_suite failed with non-matching ids"),
    }
    assert_eq!(step_id, info.id());
    assert_eq!(format!("{}", step_id), info.name.to_string());
    assert_eq!(format!("{}", step_id), info.description.map(|s| s.to_string()).unwrap());
    assert!(info.disabled);
    assert!(matches!(info.info_id, TestInfoId::Step(_, _, _)));
  }

  #[test]
  fn test_info_duplicate_suite() {
    let suite_id = Uuid::new_v4();
    let info = TestInfo {
      info_id: TestInfoId::Suite(suite_id),
      name: SharedString::new("Suite"),
      description: Some(SharedString::new("description")),
      disabled: true,
    };
    let new_name = SharedString::new("Suite duplicate");
    let duplicate = info.duplicate(new_name.clone());
    assert!(matches!(duplicate.info_id, TestInfoId::Suite(suite) if suite != suite_id));
    assert_eq!(duplicate.name, new_name);
    assert_eq!(duplicate.description, info.description);
    assert_eq!(duplicate.disabled, info.disabled);
  }

  #[test]
  fn test_info_duplicate_case() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let info = TestInfo {
      info_id: TestInfoId::CaseMulti(suite_id, case_id),
      name: SharedString::new("Case"),
      description: Some(SharedString::new("description")),
      disabled: true,
    };
    let new_name = SharedString::new("Case duplicate");
    let duplicate = info.duplicate(new_name.clone());
    assert!(matches!(duplicate.info_id, TestInfoId::CaseMulti(suite, case) if suite == suite_id && case != case_id));
    assert_eq!(duplicate.name, new_name);
    assert_eq!(duplicate.description, info.description);
    assert_eq!(duplicate.disabled, info.disabled);
  }

  #[test]
  fn test_info_duplicate_case_step() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let info = TestInfo {
      info_id: TestInfoId::CaseStep(suite_id, case_id),
      name: SharedString::new("Case"),
      description: Some(SharedString::new("description")),
      disabled: true,
    };
    let new_name = SharedString::new("Case duplicate");
    let duplicate = info.duplicate(new_name.clone());
    assert!(matches!(duplicate.info_id, TestInfoId::CaseStep(suite, case) if suite == suite_id && case != case_id));
    assert_eq!(duplicate.name, new_name);
    assert_eq!(duplicate.description, info.description);
    assert_eq!(duplicate.disabled, info.disabled);
  }

  #[test]
  fn test_info_duplicate_step() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let info = TestInfo {
      info_id: TestInfoId::Step(suite_id, case_id, step_id),
      name: SharedString::new("Step"),
      description: Some(SharedString::new("description")),
      disabled: true,
    };
    let new_name = SharedString::new("Step duplicate");
    let duplicate = info.duplicate(new_name.clone());
    assert!(
      matches!(duplicate.info_id, TestInfoId::Step(suite, case, step) if suite == suite_id && case == case_id && step != step_id)
    );
    assert_eq!(duplicate.name, new_name);
    assert_eq!(duplicate.description, info.description);
    assert_eq!(duplicate.disabled, info.disabled);
  }
}
