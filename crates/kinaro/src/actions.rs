use gpui::{actions, Action, App, KeyBinding};
pub const PROJECT_TREE_CONTEXT_KEY: &str = "ProjectTree";

// SHARED
actions!(
    [
        Delete,
        Duplicate,
        Escape,
        MoveUp,
        MoveDown,
        MoveLeft,
        MoveRight,
    ]
);

// Profiles
#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = profile, no_json)]
pub struct DeleteProfileAction(pub usize);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = profile, no_json)]
pub struct DuplicateProfileAction(pub usize);

// WORKSPACE
actions!([CreateProject, OpenProject]);

// PROJECT TREE
actions!([AddTestSuite, AddTestCase, AddTestStep, RemoveNode, SwitchNodeActiveStatus]);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        // SHARED
        KeyBinding::new("escape", Escape, None),
        KeyBinding::new("up", MoveUp, None),
        KeyBinding::new("down", MoveDown, None),
        KeyBinding::new("left", MoveLeft, None),
        KeyBinding::new("right", MoveRight, None),
        // WORKSPACE
        KeyBinding::new("ctrl-n", CreateProject, None),
        KeyBinding::new("ctrl-o", OpenProject, None),
        KeyBinding::new(
            "ctrl-shift-d",
            SwitchNodeActiveStatus,
            Some(PROJECT_TREE_CONTEXT_KEY),
        )
    ]);
}
