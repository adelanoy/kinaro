//! Fixtures shared by the unit tests of the test tree.

use crate::FileProjectMetadata;
use crate::test::{TestMetadataOwner, TestsContainer, TestsContainerEvent};
use crate::{TestCase, TestStep, TestSuite};
use gpui_kit::{AppContext, ClipboardString, Entity, TestAppContext};
use ki_project::{
  FileDelayStep, FileStepData, FileSuiteChild, FileTestCase, FileTestMetadata, FileTestStep, FileTestSuite, FileTestsContainer,
};
use ki_utils::TestPath;
use ki_utils::time::TimeUnit;
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;

/// Builds the on-disk metadata of a node, with no description and enabled.
pub(crate) fn file_meta(id: Uuid, name: &str) -> FileTestMetadata {
  FileTestMetadata {
    id,
    name: name.to_string(),
    description: vec![],
    disabled: false,
  }
}

/// Builds the on-disk representation of a delay step of `ms` milliseconds.
pub(crate) fn file_delay_step(id: Uuid, name: &str, ms: u32) -> FileTestStep {
  FileTestStep {
    info: file_meta(id, name),
    data: FileStepData::Delay(FileDelayStep {
      duration: ms,
      unit: TimeUnit::Millisecond,
    }),
  }
}

/// Builds the on-disk representation of a case, as a child of a suite.
pub(crate) fn file_case(id: Uuid, name: &str, steps: Vec<FileTestStep>) -> FileSuiteChild {
  FileSuiteChild::Case(FileTestCase {
    info: file_meta(id, name),
    steps,
  })
}

/// Builds the on-disk representation of a delay step of `ms` milliseconds, as a child of a suite.
pub(crate) fn file_suite_step(id: Uuid, name: &str, ms: u32) -> FileSuiteChild {
  FileSuiteChild::Step(file_delay_step(id, name, ms))
}

/// Builds the on-disk representation of a suite.
pub(crate) fn file_suite(id: Uuid, name: &str, children: Vec<FileSuiteChild>) -> FileTestSuite {
  FileTestSuite {
    info: file_meta(id, name),
    children,
  }
}

/// Builds a clipboard entry holding `text`, tagged with `metadata`.
pub(crate) fn clipboard(text: String, metadata: &str) -> ClipboardString {
  ClipboardString {
    text,
    metadata: Some(metadata.to_string()),
  }
}

/// Paths of the reference test tree:
/// ```text
/// Suite A
/// ├── Case A
/// │   ├── Step A1   (delay 100)
/// │   └── Step A2   (delay 200)
/// └── Suite step A  (delay 300)
/// Suite B
/// ├── Case B
/// │   └── Step B1   (delay 400)
/// └── Suite step B  (delay 500)
/// ```
#[derive(Clone, Copy, Debug)]
pub(crate) struct Paths {
  pub suite_a: TestPath,
  pub case_a: TestPath,
  pub step_a1: TestPath,
  pub step_a2: TestPath,
  pub suite_step_a: TestPath,
  pub suite_b: TestPath,
  pub case_b: TestPath,
  pub step_b1: TestPath,
  pub suite_step_b: TestPath,
}

impl Paths {
  /// Generates the paths of a new reference tree.
  pub(crate) fn new() -> Self {
    let (sa, ca, ssa) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let (sb, cb, ssb) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    Self {
      suite_a: TestPath::Suite(sa),
      case_a: TestPath::Case(sa, ca),
      step_a1: TestPath::Step(sa, ca, Uuid::new_v4()),
      step_a2: TestPath::Step(sa, ca, Uuid::new_v4()),
      suite_step_a: TestPath::SuiteStep(sa, ssa),
      suite_b: TestPath::Suite(sb),
      case_b: TestPath::Case(sb, cb),
      step_b1: TestPath::Step(sb, cb, Uuid::new_v4()),
      suite_step_b: TestPath::SuiteStep(sb, ssb),
    }
  }

  /// Returns the on-disk representation of the reference tree.
  pub(crate) fn file(&self) -> FileTestsContainer {
    FileTestsContainer {
      suites: vec![
        file_suite(
          self.suite_a.id(),
          "Suite A",
          vec![
            file_case(
              self.case_a.id(),
              "Case A",
              vec![
                file_delay_step(self.step_a1.id(), "Step A1", 100),
                file_delay_step(self.step_a2.id(), "Step A2", 200),
              ],
            ),
            file_suite_step(self.suite_step_a.id(), "Suite step A", 300),
          ],
        ),
        file_suite(
          self.suite_b.id(),
          "Suite B",
          vec![
            file_case(
              self.case_b.id(),
              "Case B",
              vec![file_delay_step(self.step_b1.id(), "Step B1", 400)],
            ),
            file_suite_step(self.suite_step_b.id(), "Suite step B", 500),
          ],
        ),
      ],
    }
  }
}

/// Loads `file` in a new container, with the workspace configuration `config`.
pub(crate) fn load(file: FileTestsContainer, config: FileProjectMetadata, cx: &mut TestAppContext) -> Entity<TestsContainer> {
  cx.new(|cx| TestsContainer::from_file(file, &config, cx))
}

/// Loads a new reference tree, with no workspace configuration.
pub(crate) fn fixture(cx: &mut TestAppContext) -> (Paths, Entity<TestsContainer>) {
  let paths = Paths::new();
  let tests = load(paths.file(), FileProjectMetadata::default(), cx);
  (paths, tests)
}

/// Records every event emitted by `tests` from now on.
pub(crate) fn record_events(tests: &Entity<TestsContainer>, cx: &mut TestAppContext) -> Rc<RefCell<Vec<TestsContainerEvent>>> {
  let events = Rc::new(RefCell::new(vec![]));
  cx.update(|cx| {
    let events = events.clone();
    cx.subscribe(tests, move |_, event: &TestsContainerEvent, _| {
      events.borrow_mut().push(event.clone())
    })
    .detach();
  });
  events
}

/// Returns the suite `path` points to or goes through.
pub(crate) fn suite(tests: &Entity<TestsContainer>, path: TestPath, cx: &TestAppContext) -> Entity<TestSuite> {
  cx.read(|cx| tests.read(cx).suite_from_path(&path, cx))
    .unwrap_or_else(|| panic!("no suite at {path}"))
}

/// Returns the case `path` points to or goes through.
pub(crate) fn case(tests: &Entity<TestsContainer>, path: TestPath, cx: &TestAppContext) -> Entity<TestCase> {
  cx.read(|cx| tests.read(cx).case_from_path(&path, cx))
    .unwrap_or_else(|| panic!("no case at {path}"))
}

/// Returns the step at `path`.
pub(crate) fn step(tests: &Entity<TestsContainer>, path: TestPath, cx: &TestAppContext) -> Entity<TestStep> {
  cx.read(|cx| tests.read(cx).step_from_path(&path, cx))
    .unwrap_or_else(|| panic!("no step at {path}"))
}

/// Returns the paths of every node of the tree, in display order.
pub(crate) fn all_paths(tests: &Entity<TestsContainer>, cx: &TestAppContext) -> Vec<TestPath> {
  cx.read(|cx| {
    tests
      .read(cx)
      .suites
      .iter()
      .flat_map(|suite| suite.read(cx).all_paths(cx))
      .collect()
  })
}

/// Returns the path of the node `path` points to followed by the paths of its descendants, in
/// display order.
pub(crate) fn subtree(tests: &Entity<TestsContainer>, path: TestPath, cx: &TestAppContext) -> Vec<TestPath> {
  match path {
    TestPath::Suite(_) => {
      let suite = suite(tests, path, cx);
      cx.read(|cx| suite.read(cx).all_paths(cx))
    }
    TestPath::Case(_, _) => {
      let case = case(tests, path, cx);
      cx.read(|cx| case.read(cx).all_paths(cx))
    }
    TestPath::SuiteStep(_, _) | TestPath::Step(_, _, _) => vec![step(tests, path, cx).read_with(cx, |step, _| step.path())],
  }
}

/// Returns the names of the suites, in display order.
pub(crate) fn suite_names(tests: &Entity<TestsContainer>, cx: &TestAppContext) -> Vec<String> {
  cx.read(|cx| {
    tests
      .read(cx)
      .suites
      .iter()
      .map(|suite| suite.read(cx).name().to_string())
      .collect()
  })
}

/// Returns the names of the children (cases and steps) of the suite `path` goes through, in display
/// order.
pub(crate) fn child_names(tests: &Entity<TestsContainer>, path: TestPath, cx: &TestAppContext) -> Vec<String> {
  let suite = suite(tests, path, cx);
  cx.read(|cx| {
    suite
      .read(cx)
      .children
      .iter()
      .map(|child| child.name(cx).to_string())
      .collect()
  })
}

/// Returns the names of the steps of the case `path` goes through, in display order.
pub(crate) fn step_names(tests: &Entity<TestsContainer>, path: TestPath, cx: &TestAppContext) -> Vec<String> {
  let case = case(tests, path, cx);
  cx.read(|cx| {
    case
      .read(cx)
      .steps()
      .iter()
      .map(|step| step.read(cx).name().to_string())
      .collect()
  })
}

/// Returns the name of the node at `path`.
pub(crate) fn name_at(tests: &Entity<TestsContainer>, path: TestPath, cx: &TestAppContext) -> String {
  cx.read(|cx| {
    let tests = tests.read(cx);
    let name = match path {
      TestPath::Suite(_) => tests.suite_from_path(&path, cx).map(|suite| suite.read(cx).name()),
      TestPath::Case(_, _) => tests.case_from_path(&path, cx).map(|case| case.read(cx).name()),
      TestPath::SuiteStep(_, _) | TestPath::Step(_, _, _) => tests.step_from_path(&path, cx).map(|step| step.read(cx).name()),
    };
    name.unwrap_or_else(|| panic!("no node at {path}")).to_string()
  })
}
