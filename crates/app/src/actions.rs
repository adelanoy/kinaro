use gpui_kit::{App, KeyBinding, actions};
pub const PROJECT_TREE_CONTEXT_KEY: &str = "ProjectTree";

// SHARED
actions!([
  Delete, Down, Duplicate, Enter, Escape, Left, MoveUp, MoveDown, MoveLeft, MoveRight, Rename, Right, Up
]);

// WORKSPACE
actions!([CreateProject, OpenProject]);

// PROJECT TREE
actions!([AddTestSuite, AddTestCase, AddTestStep, SwitchNodeActiveStatus]);

pub fn init(cx: &mut App) {
  cx.bind_keys([
    // SHARED
    KeyBinding::new("delete", Delete, None),
    KeyBinding::new("ctrl-d", Duplicate, None),
    KeyBinding::new("escape", Escape, None),
    KeyBinding::new("enter", Enter, None),
    KeyBinding::new("ctrl-up", MoveUp, None),
    KeyBinding::new("ctrl-down", MoveDown, None),
    KeyBinding::new("ctrl-left", MoveLeft, None),
    KeyBinding::new("ctrl-right", MoveRight, None),
    KeyBinding::new("up", Up, None),
    KeyBinding::new("down", Down, None),
    KeyBinding::new("left", Left, None),
    KeyBinding::new("right", Right, None),
    KeyBinding::new("f2", Rename, None),
    // WORKSPACE
    KeyBinding::new("ctrl-n", CreateProject, None),
    KeyBinding::new("ctrl-o", OpenProject, None),
    // PROJECT TREE
    KeyBinding::new("ctrl-shift-d", SwitchNodeActiveStatus, Some(PROJECT_TREE_CONTEXT_KEY)),
  ]);
}
