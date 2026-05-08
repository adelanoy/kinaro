use gpui::Action;
use uuid::Uuid;

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = workspace, no_json)]
pub struct DeleteProject(pub Uuid);
