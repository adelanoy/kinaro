use gpui_kit::{App, KeyBinding, actions};
pub const PROJECT_TREE_CONTEXT_KEY: &str = "ProjectTree";

// SHARED
actions!([
    Delete, Duplicate, Enter, Escape, MoveUp, MoveDown, MoveLeft, MoveRight, Rename,
]);

// WORKSPACE
actions!([CreateProject, OpenProject]);

// PROJECT TREE
actions!([
    AddTestSuite,
    AddTestCase,
    AddTestStep,
    SwitchNodeActiveStatus
]);

pub fn init(cx: &mut App) {
  cx.bind_keys([
    // SHARED
    KeyBinding::new("delete", Delete, None),
    KeyBinding::new("ctrl-d", Duplicate, None),
    KeyBinding::new("escape", Escape, None),
    KeyBinding::new("enter", Enter, None),
    KeyBinding::new("up", MoveUp, None),
    KeyBinding::new("down", MoveDown, None),
    KeyBinding::new("left", MoveLeft, None),
    KeyBinding::new("right", MoveRight, None),
    KeyBinding::new("f2", Rename, None),
    // WORKSPACE
    KeyBinding::new("ctrl-n", CreateProject, None),
    KeyBinding::new("ctrl-o", OpenProject, None),
    // PROJECT TREE
    KeyBinding::new(
      "ctrl-shift-d",
      SwitchNodeActiveStatus,
      Some(PROJECT_TREE_CONTEXT_KEY),
    ),
  ]);
}
