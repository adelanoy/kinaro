use crate::workspace::TestSuite;
use crate::workspace::error::ProjectError::TestNotFound;
use crate::workspace::project::Result;
use gpui_kit::{Context, EventEmitter, SharedString};
use ki_project::{FileTestInfo, FileTestsContainer};
use log::warn;
use uuid::Uuid;

pub mod test_case;
pub mod test_step;
pub mod test_suite;

#[derive(Debug, Clone, Copy)]
pub enum TestNodeKind {
  Suite,
  Case,
  Step,
  AnonymousStep,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestInfo {
  pub id: Uuid,
  pub name: SharedString,
  pub description: Option<SharedString>,
  pub disabled: bool,
}

impl TestInfo {
  pub fn from_file(file_test_info: FileTestInfo) -> Self {
    Self {
      id: file_test_info.id,
      name: file_test_info.name.into(),
      description: file_test_info.description.map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  pub fn get_file(&self) -> FileTestInfo {
    FileTestInfo {
      id: self.id,
      name: self.name.to_string(),
      description: self.description.clone().map(|s| s.to_string()),
      disabled: self.disabled,
    }
  }
}

pub enum TestsContainerEvent {
  Modified,
}

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct TestsContainer {
  pub suites: Vec<TestSuite>,
}

impl TestsContainer {
  pub fn from_file(file_container: FileTestsContainer) -> Self {
    Self {
      suites: TestSuite::from_file(file_container.suites),
    }
  }

  pub fn to_file(&self) -> FileTestsContainer {
    FileTestsContainer {
      suites: self.suites.iter().map(|suite| suite.get_file()).collect(),
    }
  }

  pub fn switch_active_status(&mut self, path: &[Uuid], cx: &mut Context<Self>) {
    if path.is_empty() {
      return;
    }
    let Some(info) = self.info_mut_from_path(path) else {
      warn!("switch_active_status: unknow path: {:?}", path);
      return;
    };
    info.disabled = !info.disabled;
    cx.emit(TestsContainerEvent::Modified);
  }

  pub fn rename_at(
    &mut self,
    path: &[Uuid],
    name: SharedString,
    cx: &mut Context<Self>,
  ) -> Result<()> {
    if path.is_empty() {
      return Ok(());
    }
    let Some(test_info) = self.info_mut_from_path(path) else {
      warn!("rename_at: unknow path: {:?}", path);
      return Err(TestNotFound);
    };

    if test_info.name != name {
      test_info.name = name;
      cx.emit(TestsContainerEvent::Modified);
    }

    Ok(())
  }

  #[allow(unused)]
  pub fn info_from_path(&self, path: &[Uuid]) -> Option<&TestInfo> {
    let suite = self.suites.iter().find(|suite| suite.info.id == path[0])?;
    if path.len() == 1 {
      Some(&suite.info)
    } else {
      suite.info_from_path(&path[1..path.len()])
    }
  }

  pub fn info_mut_from_path(&mut self, path: &[Uuid]) -> Option<&mut TestInfo> {
    let suite = self
      .suites
      .iter_mut()
      .find(|suite| suite.info.id == path[0])?;
    if path.len() == 1 {
      Some(&mut suite.info)
    } else {
      suite.info_mut_from_path(&path[1..path.len()])
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
