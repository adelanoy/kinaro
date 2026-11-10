use gpui_kit::Context;
use std::collections::VecDeque;

/// A reversible operation applied to a `T`
pub trait Edit<T>: 'static {
  /// Reverts the operation on `target`
  fn undo(&self, target: &mut T, cx: &mut Context<T>);
  /// Re-applies the operation on `target`, after it was reverted
  fn redo(&self, target: &mut T, cx: &mut Context<T>);
}

/// Undo/redo stacks of [`Edit`]s.
///
/// The history does not apply edits itself, as it is usually a field of the very `T` the edits are applied to. The
/// owner pops an edit, applies it, then pushes it on the opposite stack:
/// - undo: [`History::pop_undo`], [`Edit::undo`], [`History::push_redo`]
/// - redo: [`History::pop_redo`], [`Edit::redo`], [`History::push_undo`]
///
/// The undo stack is capped: when full, the oldest edit is dropped.
pub struct History<T> {
  undo: VecDeque<Box<dyn Edit<T>>>,
  redo: Vec<Box<dyn Edit<T>>>,
  limit: usize,
}

impl<T> History<T> {
  /// Records a newly performed edit. The redo stack is cleared, as the edits it holds no longer apply
  pub fn push(&mut self, edit: Box<dyn Edit<T>>) {
    self.redo.clear();
    self.push_undo(edit);
  }

  /// Takes the most recent edit to undo, if any
  pub fn pop_undo(&mut self) -> Option<Box<dyn Edit<T>>> {
    self.undo.pop_back()
  }

  /// Puts back an edit that was just redone, keeping the redo stack intact
  pub fn push_undo(&mut self, edit: Box<dyn Edit<T>>) {
    if self.limit == 0 {
      return;
    }
    if self.undo.len() >= self.limit {
      self.undo.pop_front();
    }
    self.undo.push_back(edit);
  }

  /// Takes the most recently undone edit, if any
  pub fn pop_redo(&mut self) -> Option<Box<dyn Edit<T>>> {
    self.redo.pop()
  }

  /// Stores an edit that was just undone, so that it can be redone
  pub fn push_redo(&mut self, edit: Box<dyn Edit<T>>) {
    self.redo.push(edit)
  }

  /// Whether there is an edit to undo
  pub fn can_undo(&self) -> bool {
    !self.undo.is_empty()
  }

  /// Whether there is an edit to redo
  pub fn can_redo(&self) -> bool {
    !self.redo.is_empty()
  }
}

impl<T> Default for History<T> {
  fn default() -> Self {
    Self {
      undo: Default::default(),
      redo: vec![],
      limit: 20,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct Target;

  /// An edit that does nothing. It holds a field so that each boxed edit gets its own allocation, which is used to
  /// identify it when checking the stacks ordering
  struct Noop(#[allow(dead_code)] u32);

  impl Edit<Target> for Noop {
    fn undo(&self, _target: &mut Target, _cx: &mut Context<Target>) {}
    fn redo(&self, _target: &mut Target, _cx: &mut Context<Target>) {}
  }

  /// Pushes a new edit and returns its identity
  fn push(history: &mut History<Target>, id: u32) -> usize {
    let edit: Box<dyn Edit<Target>> = Box::new(Noop(id));
    let addr = addr_of(edit.as_ref());
    history.push(edit);
    addr
  }

  fn addr_of(edit: &dyn Edit<Target>) -> usize {
    edit as *const dyn Edit<Target> as *const () as usize
  }

  /// Simulates an undo (pop from undo, push to redo) and returns the moved edit's identity
  fn undo(history: &mut History<Target>) -> Option<usize> {
    let edit = history.pop_undo()?;
    let addr = addr_of(edit.as_ref());
    history.push_redo(edit);
    Some(addr)
  }

  /// Simulates a redo (pop from redo, push to undo) and returns the moved edit's identity
  fn redo(history: &mut History<Target>) -> Option<usize> {
    let edit = history.pop_redo()?;
    let addr = addr_of(edit.as_ref());
    history.push_undo(edit);
    Some(addr)
  }

  #[test]
  fn undo_then_redo_in_reverse_order() {
    let mut history = History::default();
    let e1 = push(&mut history, 1);
    let e2 = push(&mut history, 2);

    assert_eq!(undo(&mut history), Some(e2));
    assert_eq!(undo(&mut history), Some(e1));
    assert_eq!(undo(&mut history), None);

    assert_eq!(redo(&mut history), Some(e1));
    assert_eq!(redo(&mut history), Some(e2));
    assert_eq!(redo(&mut history), None);

    assert_eq!(undo(&mut history), Some(e2));
  }

  #[test]
  fn new_edit_clears_redo() {
    let mut history = History::default();
    let e1 = push(&mut history, 1);
    push(&mut history, 2);
    undo(&mut history);
    assert!(history.can_redo());

    let e3 = push(&mut history, 3);
    assert!(!history.can_redo());
    assert_eq!(undo(&mut history), Some(e3));
    assert_eq!(undo(&mut history), Some(e1));
  }

  #[test]
  fn redo_keeps_remaining_redo_edits() {
    let mut history = History::default();
    push(&mut history, 1);
    let e2 = push(&mut history, 2);
    undo(&mut history);
    undo(&mut history);

    redo(&mut history);
    assert!(history.can_redo());
    assert_eq!(redo(&mut history), Some(e2));
  }

  #[test]
  fn undo_stack_is_capped() {
    let mut history = History {
      undo: Default::default(),
      redo: vec![],
      limit: 2,
    };
    push(&mut history, 1);
    let e2 = push(&mut history, 2);
    let e3 = push(&mut history, 3);

    assert_eq!(undo(&mut history), Some(e3));
    assert_eq!(undo(&mut history), Some(e2));
    assert_eq!(undo(&mut history), None);
  }
}
