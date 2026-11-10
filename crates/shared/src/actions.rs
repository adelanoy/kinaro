use gpui_kit::base::actions::Cancel;
use gpui_kit::{Action, App, KeyBinding, NoAction, actions};
use ki_utils::http::HttpMethod;

pub const PROJECT_TREE_CONTEXT_KEY: &str = "ProjectTree";
pub const TABLE_CONTEXT_KEY: &str = "Table";

// SHARED
actions!([
  CopyId, CopyPath, Delete, Down, Duplicate, Edit, Enter, Left, MoveUp, MoveDown, MoveLeft, MoveRight, New, Redo, Right, Undo, Up
]);

// WORKSPACE
actions!([CreateProject, OpenProject]);

// PROJECT TREE
actions!([AddTestSuite, AddTestCase, AddDelayStep, SwitchNodeActiveStatus]);

/// Adds a new Rest step sending a `method` request.
#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = project_tree, no_json)]
pub struct AddRestStep {
  pub method: HttpMethod,
}

pub fn init(cx: &mut App) {
  cx.bind_keys([
    // SHARED
    KeyBinding::new("escape", Cancel, None),
    KeyBinding::new("ctrl-alt-i", CopyId, None),
    KeyBinding::new("ctrl-alt-p", CopyPath, None),
    KeyBinding::new("delete", Delete, None),
    KeyBinding::new("ctrl-d", Duplicate, None),
    KeyBinding::new("f2", Edit, None),
    KeyBinding::new("enter", Enter, None),
    KeyBinding::new("ctrl-up", MoveUp, None),
    KeyBinding::new("ctrl-down", MoveDown, None),
    KeyBinding::new("ctrl-left", MoveLeft, None),
    KeyBinding::new("ctrl-right", MoveRight, None),
    KeyBinding::new("ctrl-shift-n", New, None),
    KeyBinding::new("down", Down, None),
    KeyBinding::new("left", Left, None),
    KeyBinding::new("ctrl-shift-z", Redo, None),
    KeyBinding::new("right", Right, None),
    KeyBinding::new("up", Up, None),
    KeyBinding::new("ctrl-z", Undo, None),
    // WORKSPACE
    KeyBinding::new("ctrl-n", CreateProject, None),
    KeyBinding::new("ctrl-o", OpenProject, None),
    // PROJECT TREE
    KeyBinding::new("ctrl-shift-d", SwitchNodeActiveStatus, Some(PROJECT_TREE_CONTEXT_KEY)),
    // INPUT
    // gpui-base binds space to `Confirm` in the "Popover" context, which also matches inputs nested
    // in a popover content and closes it. Inputs being deeper, this lets the space be typed instead.
    KeyBinding::new("space", NoAction, Some("Input")),
  ]);
}
