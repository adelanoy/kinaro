use gpui_kit::base::IndexPath;
use gpui_kit::component::select::SelectState;
use gpui_kit::{App, AppContext, Entity, Window};
use ki_workspace::variable::{Profile, ProjectVariables};

pub mod tree;

/// Creates a Profile `SelectState` initialized with the list of profiles in the project and with the `active_profile` selected
pub fn profile_select(vars: &Entity<ProjectVariables>, window: &mut Window, cx: &mut App) -> Entity<SelectState<Vec<Profile>>> {
  let (index, profiles) = vars.read_with(cx, |vars, _| {
    let active_profile = vars.active_profile();
    (
      vars.profiles.iter().position(|p| Some(p.id) == active_profile).unwrap_or(0),
      vars.profiles.clone(),
    )
  });
  cx.new(|cx| SelectState::new(profiles, Some(IndexPath::new(index)), window, cx))
}
