use gpui::{EventEmitter, SharedString};
use ki_project::{FileTestInfo, FileTestsContainer};
use uuid::Uuid;

pub mod test_case;
pub mod test_step;
pub mod test_suite;

pub use test_suite::TestSuite;
pub use test_case::TestCase;
pub use test_step::TestStep;

#[derive(Debug, Clone)]
pub enum TestNodeKind {
    Suite,
    Case,
    Step
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestInfo {
    pub id: Uuid,
    pub name: SharedString,
    pub description: Option<String>,
    pub active: bool,
}

impl TestInfo {
    pub fn from_file(file_test_info: FileTestInfo) -> Self {
        Self {
            id: file_test_info.id,
            name: file_test_info.name.into(),
            description: file_test_info.description.clone(),
            active: file_test_info.active,
        }
    }

    pub fn get_file(&self) -> FileTestInfo {
        FileTestInfo {
            id: self.id,
            name: self.name.to_string(),
            description: self.description.clone(),
            active: self.active,
        }
    }
}

pub enum TestsContainerEvent {
    TestSuiteAdded(Vec<Uuid>),
    TestSuiteRemoved(Vec<Uuid>),
    TestSuiteMoved((Vec<Uuid>, Vec<Uuid>)),
    TestCaseAdded(Vec<Uuid>),
    TestCaseRemoved(Vec<Uuid>),
    TestCaseMoved((Vec<Uuid>, Vec<Uuid>)),
    TestStepAdded(Vec<Uuid>),
    TestStepRemoved(Vec<Uuid>),
    TestStepMoved((Vec<Uuid>, Vec<Uuid>)),
}

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct TestsContainer {
    pub test_suites: Vec<TestSuite>,
}

impl TestsContainer {

    pub fn from_file(file_container: FileTestsContainer) -> Self {
        Self {
            test_suites: TestSuite::from_file(file_container.suites),
        }
    }

    pub fn to_file(&self) -> FileTestsContainer {
        FileTestsContainer {
            suites: self
                .test_suites
                .iter()
                .map(|suite| suite.get_file())
                .collect(),
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
