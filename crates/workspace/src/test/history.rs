//! Undo/redo of the edits made to the test tree.
//!
//! Edits hold the entities of the nodes they apply to: a removed node is kept alive by its edit, so undoing the
//! removal puts back the very same node, with its ids and data. Since the history is linear, a node is always found
//! where the edit left it when the edit is reverted or re-applied.

use crate::test::test_suite::TestSuiteChild;
use crate::test::{TestMetadata, TestMetadataOwner, TestsContainer, TestsContainerEvent};
use crate::{TestCase, TestStep, TestSuite};
use gpui_kit::{App, Context, Entity, WeakEntity};
use ki_utils::TestPath;
use ki_utils::undo::{Edit, UndoEntity};

/// A node of the test tree, whatever its kind.
#[derive(Clone, Debug)]
pub(crate) enum TestNode {
  Suite(Entity<TestSuite>),
  Case(Entity<TestCase>),
  Step(Entity<TestStep>),
}

impl TestNode {
  /// Returns the path of the node followed by the paths of its descendants, in display order.
  pub(crate) fn all_paths(&self, cx: &App) -> Vec<TestPath> {
    match self {
      TestNode::Suite(suite) => suite.read(cx).all_paths(cx),
      TestNode::Case(case) => case.read(cx).all_paths(cx),
      TestNode::Step(step) => vec![step.read(cx).path()],
    }
  }

  /// Applies to the node the metadata change from `from` to `to` (only the values differing between them, see
  /// [`TestMetadata`]), notifying the node if it changed. The owning container isn't notified. Returns whether the
  /// metadata changed.
  pub(crate) fn apply_meta_change(&self, from: &TestMetadata, to: &TestMetadata, cx: &mut App) -> bool {
    fn apply<T: 'static>(meta: &mut TestMetadata, from: &TestMetadata, to: &TestMetadata, cx: &mut Context<T>) -> bool {
      let changed = meta.apply_change(from, to);
      if changed {
        cx.notify();
      }
      changed
    }
    match self {
      TestNode::Suite(suite) => suite.update(cx, |suite, cx| apply(&mut suite.meta, from, to, cx)),
      TestNode::Case(case) => case.update(cx, |case, cx| apply(&mut case.meta, from, to, cx)),
      TestNode::Step(step) => step.update(cx, |step, cx| apply(&mut step.meta, from, to, cx)),
    }
  }

  /// Returns a copy of the metadata of the node.
  pub(crate) fn meta(&self, cx: &App) -> TestMetadata {
    match self {
      TestNode::Suite(suite) => suite.read(cx).meta.clone(),
      TestNode::Case(case) => case.read(cx).meta.clone(),
      TestNode::Step(step) => step.read(cx).meta.clone(),
    }
  }

  /// Returns the current path of the node.
  pub(crate) fn path(&self, cx: &App) -> TestPath {
    match self {
      TestNode::Suite(suite) => suite.read(cx).path(),
      TestNode::Case(case) => case.read(cx).path(),
      TestNode::Step(step) => step.read(cx).path(),
    }
  }
}

impl From<TestSuiteChild> for TestNode {
  fn from(child: TestSuiteChild) -> Self {
    match child {
      TestSuiteChild::Case(case) => TestNode::Case(case),
      TestSuiteChild::Step(step) => TestNode::Step(step),
    }
  }
}

/// Where a node is placed in the test tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TestLocation {
  /// The node holding it: `None` for a suite, the suite for a case or a suite step, the case for a step of a case
  pub(crate) parent: Option<TestPath>,
  /// Its index among the children of `parent`
  pub(crate) index: usize,
}

impl TestLocation {
  /// The location of the node at `path`, placed at `index` among its siblings.
  pub(crate) fn new(path: &TestPath, index: usize) -> Self {
    Self {
      parent: path.parent(),
      index,
    }
  }
}

/// A node was inserted in the tree (added, duplicated or pasted)
pub(crate) struct AddTest {
  pub(crate) node: TestNode,
  pub(crate) location: TestLocation,
}

impl Edit<TestsContainer> for AddTest {
  fn undo(&self, target: &mut TestsContainer, cx: &mut Context<TestsContainer>) {
    let path = self.node.path(cx);
    target.detach_node(&path, cx);
  }

  fn redo(&self, target: &mut TestsContainer, cx: &mut Context<TestsContainer>) {
    target.attach_node(self.node.clone(), self.location, cx);
  }
}

/// A node was removed from the tree, along with its descendants
pub(crate) struct RemoveTest {
  pub(crate) node: TestNode,
  pub(crate) location: TestLocation,
}

impl Edit<TestsContainer> for RemoveTest {
  fn undo(&self, target: &mut TestsContainer, cx: &mut Context<TestsContainer>) {
    target.attach_node(self.node.clone(), self.location, cx);
  }

  fn redo(&self, target: &mut TestsContainer, cx: &mut Context<TestsContainer>) {
    let path = self.node.path(cx);
    target.detach_node(&path, cx);
  }
}

/// A node was moved, either within its parent (offset) or to another parent (drag and drop)
pub(crate) struct MoveTest {
  pub(crate) node: TestNode,
  pub(crate) from: TestLocation,
  pub(crate) to: TestLocation,
}

impl Edit<TestsContainer> for MoveTest {
  fn undo(&self, target: &mut TestsContainer, cx: &mut Context<TestsContainer>) {
    target.relocate_node(&self.node, self.from, cx);
  }

  fn redo(&self, target: &mut TestsContainer, cx: &mut Context<TestsContainer>) {
    target.relocate_node(&self.node, self.to, cx);
  }
}

/// The metadata of a node was modified (renamed, described, enabled/disabled).
///
/// It is recorded either in the container's history, or in the node's own history (see [`MetaHistoryOwner`]). Only the
/// values differing between `before` and `after` are reverted or re-applied, so that the changes recorded in the other
/// history are kept.
pub(crate) struct UpdateMeta {
  pub(crate) node: TestNode,
  pub(crate) before: TestMetadata,
  pub(crate) after: TestMetadata,
}

impl Edit<TestsContainer> for UpdateMeta {
  fn undo(&self, _target: &mut TestsContainer, cx: &mut Context<TestsContainer>) {
    if self.node.apply_meta_change(&self.after, &self.before, cx) {
      cx.emit(TestsContainerEvent::TestsModified);
    }
  }

  fn redo(&self, _target: &mut TestsContainer, cx: &mut Context<TestsContainer>) {
    if self.node.apply_meta_change(&self.before, &self.after, cx) {
      cx.emit(TestsContainerEvent::TestsModified);
    }
  }
}

impl<T: MetaHistoryOwner> Edit<T> for UpdateMeta {
  fn undo(&self, target: &mut T, cx: &mut Context<T>) {
    target.apply_meta_change(&self.after, &self.before, cx);
  }

  fn redo(&self, target: &mut T, cx: &mut Context<T>) {
    target.apply_meta_change(&self.before, &self.after, cx);
  }
}

/// A node of the test tree recording the changes made to its metadata in its own history.
pub(crate) trait MetaHistoryOwner: TestMetadataOwner + UndoEntity {
  /// Returns the container holding the node.
  fn container(&self) -> WeakEntity<TestsContainer>;
  /// Returns the metadata of the node.
  fn meta(&self) -> &TestMetadata;
  /// Returns the metadata of the node, for edition.
  fn meta_mut(&mut self) -> &mut TestMetadata;
  /// Returns the node, as addressed in edits.
  fn node(cx: &Context<Self>) -> TestNode;

  /// Applies the metadata change from `from` to `to` (only the values differing between them, see [`UpdateMeta`]).
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container if the metadata changed.
  fn apply_meta_change(&mut self, from: &TestMetadata, to: &TestMetadata, cx: &mut Context<Self>) {
    let changed = self.meta_mut().apply_change(from, to);
    if changed {
      cx.notify();
      _ = self
        .container()
        .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    }
  }

  /// Records in the node's own history the change made to its metadata since `before`. A no-op if it didn't change.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container if the metadata changed.
  fn record_meta_change(&mut self, before: TestMetadata, cx: &mut Context<Self>) {
    let after = self.meta().clone();
    if before == after {
      return;
    }
    let node = Self::node(cx);
    self.history().push(Box::new(UpdateMeta { node, before, after }));
    _ = self
      .container()
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
  }
}
