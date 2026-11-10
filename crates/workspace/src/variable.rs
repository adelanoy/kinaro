use crate::FileProjectMetadata;
use crate::error::{ProjectError, ProjectResult};
use gpui_kit::component::select::SelectItem;
use gpui_kit::{Context, EventEmitter, SharedString};
use ki_project::{FileProfile, FileProjectVariables, FileVariable, FileVariableKind};
use ki_utils::next_available_name;
use ki_utils::undo::{Edit, History};
use log::warn;
use std::cmp::{Ordering, min};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum VariableKind {
  Text,
  PasswordClear,
  PasswordEncrypt,
}

impl From<FileVariableKind> for VariableKind {
  fn from(value: FileVariableKind) -> Self {
    match value {
      FileVariableKind::Text => VariableKind::Text,
      FileVariableKind::PasswordClear => VariableKind::PasswordClear,
      FileVariableKind::PasswordEncrypt => VariableKind::PasswordEncrypt,
    }
  }
}

impl From<VariableKind> for FileVariableKind {
  fn from(value: VariableKind) -> Self {
    match value {
      VariableKind::Text => FileVariableKind::Text,
      VariableKind::PasswordClear => FileVariableKind::PasswordClear,
      VariableKind::PasswordEncrypt => FileVariableKind::PasswordEncrypt,
    }
  }
}

impl Display for VariableKind {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      VariableKind::Text => f.write_str("Plain"),
      VariableKind::PasswordClear => f.write_str("Password (Clear)"),
      VariableKind::PasswordEncrypt => f.write_str("Password (Encrypted)"),
    }
  }
}

impl SelectItem for VariableKind {
  type Value = VariableKind;

  fn title(&self) -> SharedString {
    SharedString::new(format!("{self}"))
  }

  fn value(&self) -> &Self::Value {
    self
  }
}

///// PROJECT VARIABLES EVENTS /////
pub enum ProjectVariablesEvent {
  /// Emitted when the profile list has been modified (profile added, removed, modified...)
  ProfilesChanged,
  /// Emitted when the reference variables has been modified (added, removed, modified...)
  VariablesChanged,
  /// Emitted when the active profile has been changed, carrying the new active profile id (*None* when no profile is
  /// active). The active profile is not part of the project file: it is saved with the workspace
  ActiveProfile(Option<Uuid>),
}

#[derive(Default)]
pub struct ProjectVariables {
  pub references: Vec<VariableReference>,
  pub profiles: Vec<Profile>,
  history_references: History<ProjectVariables>,
  history_profiles: History<ProjectVariables>,
  active_profile: Option<Uuid>,
}

impl ProjectVariables {
  ///// PROFILES
  /// Returns the id of the currently active profile, *None* when no profile is active
  #[inline]
  pub fn active_profile(&self) -> Option<Uuid> {
    self.active_profile
  }

  /// Adds a profile with the given name
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`]
  pub fn add_profile(&mut self, row_ix: Option<usize>, cx: &mut Context<Self>) {
    let name = next_available_name("new profile", self.profiles.iter().map(|p| p.name.clone()));
    let new_profile = Profile {
      id: Uuid::new_v4(),
      name,
      description: Default::default(),
    };
    let ix = row_ix
      .map(|ix| min(ix + 1, self.profiles.len()))
      .unwrap_or(self.profiles.len());
    self.profiles.insert(ix, new_profile.clone());
    self.history_profiles.push(Box::new(UndoAddProfile {
      profile: new_profile,
      index: ix,
    }));
    cx.emit(ProjectVariablesEvent::ProfilesChanged);
  }

  /// Deletes a profile. If it was the active profile, no profile is active anymore
  /// # Result
  /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if the profile was deleted, followed by a
  /// [`ProjectVariablesEvent::ActiveProfile`] with *None* if it was the active profile
  pub fn delete_profile(&mut self, index: usize, cx: &mut Context<Self>) -> ProjectResult<()> {
    if index > self.profiles.len() - 1 {
      warn!(
        "Failed to delete profile at row: {} (max: {})",
        index,
        self.profiles.len() - 1
      );
      return Err(ProjectError::ProfileNotFound);
    }
    let profile = self.profiles.remove(index);
    cx.emit(ProjectVariablesEvent::ProfilesChanged);

    let was_active = Some(profile.id) == self.active_profile;
    if was_active {
      self.active_profile = None;
      cx.emit(ProjectVariablesEvent::ActiveProfile(None))
    }
    self.history_profiles.push(Box::new(UndoDeleteProfile {
      profile,
      index,
      was_active,
    }));
    Ok(())
  }

  /// Duplicates a profile, attributing a new id and copying all fields. Name is appended with *_X* where X is an integer.
  ///
  /// The copy is placed right after the original
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if the profile
  pub fn duplicate_profile(&mut self, index: usize, cx: &mut Context<Self>) {
    let Some(profile) = self.profiles.get(index) else {
      return;
    };
    let mut duplicated_profile = profile.clone();
    duplicated_profile.id = Uuid::new_v4();
    duplicated_profile.name = next_available_name(&profile.name, self.profiles.iter().map(|p| p.name.clone()));
    let ix = min(index + 1, self.profiles.len());
    self.profiles.insert(ix, duplicated_profile.clone());
    self.history_profiles.push(Box::new(UndoAddProfile {
      profile: duplicated_profile,
      index: ix,
    }));
    cx.emit(ProjectVariablesEvent::ProfilesChanged);
  }

  /// Moves a profile from one position to another
  ///
  /// # Result
  /// Returns a copy of the new profiles list
  ///
  /// Returns a [`ProjectError::ProfileNotFound`] if both indexes are identical, or if either one of them is out of bound
  pub fn move_profile(&mut self, from_id: Uuid, to_ix: usize, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(from_ix) = self.profiles.iter().position(|p| p.id == from_id) else {
      return Err(ProjectError::ProfileNotFound);
    };
    if from_ix == to_ix {
      return Ok(());
    }
    let max_ix = self.profiles.len() - 1;
    if from_ix > max_ix || to_ix > max_ix {
      return Err(ProjectError::OperationNotAllowed);
    }
    let profile_to_move = self.profiles.remove(from_ix);
    self.profiles.insert(to_ix, profile_to_move);
    self.history_profiles.push(Box::new(UndoMoveProfile { from_ix, to_ix }));
    cx.emit(ProjectVariablesEvent::ProfilesChanged);
    Ok(())
  }

  /// Merges a profile attributes
  ///
  /// # Result
  /// Returns a copy of the new profiles list if it was modified, else *None*
  ///
  /// Returns a [`ProjectError::ProfileNotFound`] if the profile could not be found by its id
  pub fn update_profile(&mut self, updated_profile: Profile, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(profile) = self.profiles.iter_mut().find(|p| p.id == updated_profile.id) else {
      warn!("Attempt to edit non existing profile with id: {}", updated_profile.id);
      return Err(ProjectError::ProfileNotFound);
    };

    if updated_profile != *profile {
      let before = std::mem::replace(profile, updated_profile.clone());
      self.history_profiles.push(Box::new(UndoUpdateProfile {
        before,
        after: updated_profile,
      }));
      cx.emit(ProjectVariablesEvent::ProfilesChanged);
    }
    Ok(())
  }

  /// Change the currently active profile, *None* meaning no active profile. A no-op when `profile_id` is already active
  /// or doesn't address an existing profile.
  ///
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ActiveProfile`] if the active profile was changed, nothing otherwise
  pub fn switch_profile(&mut self, profile_id: Option<Uuid>, cx: &mut Context<Self>) {
    if profile_id == self.active_profile {
      return;
    }
    match profile_id {
      None => {
        self.active_profile = None;
        cx.emit(ProjectVariablesEvent::ActiveProfile(None));
      }
      Some(profile_id) => {
        if self.profiles.iter().any(|p| p.id == profile_id) {
          self.active_profile = Some(profile_id);
          cx.emit(ProjectVariablesEvent::ActiveProfile(self.active_profile));
        }
      }
    }
  }

  ///// VARIABLES
  pub fn add_variable(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
    let name = next_available_name("new variable", self.references.iter().map(|p| p.name.clone()));
    let new_var = VariableReference {
      id: Uuid::new_v4(),
      name: SharedString::new(name),
      description: Default::default(),
      kind: VariableKind::Text,
      value: SharedString::new(""),
      overrides: HashMap::new(),
    };
    let ix = index
      .map(|ix| min(ix + 1, self.references.len()))
      .unwrap_or(self.references.len());
    self.references.insert(ix, new_var.clone());
    self.history_references.push(Box::new(UndoAddVariable {
      variable: new_var,
      index: ix,
    }));
    cx.emit(ProjectVariablesEvent::VariablesChanged);
  }

  /// Deletes a variable
  /// # Result
  /// Returns a [`ProjectError::VariableNotFound`] if the index is out of bound
  /// # Events
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if the profile was deleted
  pub fn delete_variable(&mut self, index: usize, cx: &mut Context<Self>) -> ProjectResult<()> {
    if index > self.references.len() - 1 {
      warn!(
        "Failed to delete variable at row: {} (len: {})",
        index,
        self.references.len() - 1
      );
      return Err(ProjectError::VariableNotFound);
    }
    let reference = self.references.remove(index);
    self.history_references.push(Box::new(UndoDeleteVariable {
      variable: reference,
      index,
    }));
    cx.emit(ProjectVariablesEvent::VariablesChanged);
    Ok(())
  }

  /// Duplicates a variable, attributing a new id and copying all fields. Name is appended with *_X* where X is an integer.
  ///
  /// The copy is placed right after the original
  /// # Events
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if the profile
  pub fn duplicate_variable(&mut self, index: usize, cx: &mut Context<Self>) {
    let Some(var) = self.references.get(index) else {
      return;
    };
    let mut duplicated_var = var.clone();
    duplicated_var.id = Uuid::new_v4();
    duplicated_var.name = next_available_name(&var.name, self.references.iter().map(|p| p.name.clone()));
    let ix = min(index + 1, self.references.len());
    self.references.insert(ix, duplicated_var.clone());
    self.history_references.push(Box::new(UndoAddVariable {
      variable: duplicated_var,
      index: ix,
    }));
    cx.emit(ProjectVariablesEvent::VariablesChanged);
  }

  /// Merges a variable attributes.
  ///
  /// If a profile id is provided, the profile's override value is updated, else the reference value is updated
  /// # Result
  ///
  /// Returns a [`ProjectError::VariableNotFound`] if the profile could not be found by its id
  pub fn update_variable(&mut self, updated_variable: VariableReference, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(var) = self.references.iter_mut().find(|p| p.id == updated_variable.id) else {
      warn!("Attempt to edit non existing variable with id: {}", updated_variable.id);
      return Err(ProjectError::VariableNotFound);
    };

    if updated_variable == *var {
      return Ok(());
    }
    let before = std::mem::replace(var, updated_variable.clone());
    self.history_references.push(Box::new(UndoUpdateVariable {
      before,
      after: updated_variable,
    }));

    cx.emit(ProjectVariablesEvent::VariablesChanged);
    Ok(())
  }

  /// Removes the override value of a variable for the given profile, the variable falling back to its reference value
  /// for this profile. A no-op if the variable has no override for this profile
  ///
  /// # Result
  /// Returns a [`ProjectError::VariableNotFound`] if the index is out of bound
  /// # Events
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if an override was removed
  pub fn revert_override(&mut self, index: usize, profile_id: Uuid, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(var) = self.references.get_mut(index) else {
      warn!("Failed to revert override of variable at row: {}", index);
      return Err(ProjectError::VariableNotFound);
    };
    let before = var.clone();
    if var.overrides.remove(&profile_id).is_some() {
      self.history_references.push(Box::new(UndoUpdateVariable {
        before,
        after: var.clone(),
      }));
      cx.emit(ProjectVariablesEvent::VariablesChanged);
    }
    Ok(())
  }

  pub fn move_variable(&mut self, from_id: Uuid, to_ix: usize, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(from_ix) = self.references.iter().position(|p| p.id == from_id) else {
      return Err(ProjectError::VariableNotFound);
    };
    if from_ix == to_ix {
      return Ok(());
    }
    let max_ix = self.references.len() - 1;
    if from_ix > max_ix || to_ix > max_ix {
      return Err(ProjectError::OperationNotAllowed);
    }
    let var_to_move = self.references.remove(from_ix);
    self.references.insert(to_ix, var_to_move);
    self.history_references.push(Box::new(UndoMoveVariable { from_ix, to_ix }));
    cx.emit(ProjectVariablesEvent::VariablesChanged);
    Ok(())
  }

  ///// CONVERSION
  pub(super) fn from_file(file_vars: FileProjectVariables, metadata: &FileProjectMetadata) -> Self {
    let profiles = file_vars.profiles.iter().map(Profile::from_file).collect::<Vec<Profile>>();
    let profile_ids: Vec<Uuid> = profiles.iter().map(|p| p.id).collect();
    let variables: Vec<VariableReference> = file_vars
      .variables
      .into_iter()
      .map(|file_var| VariableReference::from_file(file_var, &profile_ids))
      .collect();
    let active_profile = metadata.active_profile.filter(|id| profiles.iter().any(|p| p.id == *id));

    ProjectVariables {
      references: variables,
      profiles,
      history_references: Default::default(),
      history_profiles: Default::default(),
      active_profile,
    }
  }

  pub(super) fn to_file(&self) -> FileProjectVariables {
    let variables = self.references.iter().map(|p| p.to_file()).collect();
    let profiles = self.profiles.iter().map(|p| p.to_file()).collect();

    FileProjectVariables { variables, profiles }
  }

  ///// UNDO
  /// Reverts the last operation made on the profiles (add, duplicate, delete, move, update). A no-op if there is
  /// nothing to undo
  pub fn undo_profile(&mut self, cx: &mut Context<Self>) {
    if let Some(undo) = self.history_profiles.pop_undo() {
      undo.undo(self, cx);
      self.history_profiles.push_redo(undo);
    }
  }

  /// Reverts the last operation made on the variables (add, duplicate, delete, move, update, override revert). A no-op
  /// if there is nothing to undo
  pub fn undo_reference(&mut self, cx: &mut Context<Self>) {
    if let Some(undo) = self.history_references.pop_undo() {
      undo.undo(self, cx);
      self.history_references.push_redo(undo);
    }
  }

  ///// REDO
  /// Re-applies the last profile operation reverted by [`ProjectVariables::undo_profile`]. A no-op if there is nothing
  /// to redo, which is the case as soon as a new profile operation is performed after an undo
  pub fn redo_profile(&mut self, cx: &mut Context<Self>) {
    if let Some(redo) = self.history_profiles.pop_redo() {
      redo.redo(self, cx);
      self.history_profiles.push_undo(redo);
    }
  }

  /// Re-applies the last variable operation reverted by [`ProjectVariables::undo_reference`]. A no-op if there is
  /// nothing to redo, which is the case as soon as a new variable operation is performed after an undo
  /// # Events
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if an operation was re-applied
  pub fn redo_reference(&mut self, cx: &mut Context<Self>) {
    if let Some(redo) = self.history_references.pop_redo() {
      redo.redo(self, cx);
      self.history_references.push_undo(redo);
    }
  }
}

impl EventEmitter<ProjectVariablesEvent> for ProjectVariables {}

/// This struct contains the definition of a variable with all the overridden values per-profile
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct VariableReference {
  pub id: Uuid,
  pub name: SharedString,
  pub description: SharedString,
  pub kind: VariableKind,
  /// Reference value for this variable. to set a new value, [`VariableReference::set_value`] should be used to handle overrides
  pub value: SharedString,
  overrides: HashMap<Uuid, SharedString>,
}

impl VariableReference {
  fn from_file(file_vars: FileVariable, profiles: &[Uuid]) -> Self {
    Self {
      id: file_vars.id,
      name: SharedString::new(&file_vars.name),
      description: SharedString::new(&file_vars.description),
      kind: file_vars.kind.into(),
      value: SharedString::new(&file_vars.value),
      overrides: file_vars
        .overrides
        .into_iter()
        .filter(|(profile_id, _)| profiles.contains(profile_id))
        .map(|(profile_id, value)| (profile_id, SharedString::new(value)))
        .collect(),
    }
  }

  fn to_file(&self) -> FileVariable {
    FileVariable {
      id: self.id,
      name: self.name.to_string(),
      description: self.description.to_string(),
      kind: self.kind.into(),
      value: self.value.to_string(),
      overrides: self
        .overrides
        .iter()
        .map(|(profile_id, value)| (*profile_id, value.to_string()))
        .collect(),
    }
  }

  pub fn find_override(&self, profile_id: &Uuid) -> Option<SharedString> {
    self
      .overrides
      .iter()
      .find_map(|(id, value)| if id == profile_id { Some(value.clone()) } else { None })
  }

  /// Sets a new value depending on the profile.
  pub fn set_value(&mut self, value: SharedString, profile_id: Option<Uuid>) {
    match profile_id {
      Some(id) if self.value == value => {
        self.overrides.remove(&id);
      }
      Some(id) => {
        self.overrides.insert(id, value);
      }
      None => self.value = value,
    };
  }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Profile {
  pub id: Uuid,
  pub name: SharedString,
  pub description: SharedString,
}

impl Profile {
  fn from_file(file_profile: &FileProfile) -> Self {
    Self {
      id: file_profile.id,
      name: SharedString::new(&file_profile.name),
      description: SharedString::new(&file_profile.description),
    }
  }

  fn to_file(&self) -> FileProfile {
    FileProfile {
      id: self.id,
      name: self.name.to_string(),
      description: self.description.to_string(),
    }
  }
}

impl PartialOrd for Profile {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.name.partial_cmp(&other.name)
  }
}

impl SelectItem for Profile {
  type Value = Uuid;

  fn title(&self) -> SharedString {
    self.name.clone()
  }

  fn value(&self) -> &Self::Value {
    &self.id
  }
}

#[derive(PartialEq, Clone, Debug)]
pub struct DraggedVar(pub Uuid);

#[derive(PartialEq, Clone, Debug)]
pub struct DraggedProfile(pub Uuid);

///// UNDO / REDO
fn insert_profile(target: &mut ProjectVariables, index: usize, profile: &Profile, cx: &mut Context<ProjectVariables>) {
  let ix = min(index, target.profiles.len());
  target.profiles.insert(ix, profile.clone());
  cx.emit(ProjectVariablesEvent::ProfilesChanged);
  cx.notify();
}

fn remove_profile(target: &mut ProjectVariables, id: Uuid, cx: &mut Context<ProjectVariables>) {
  let Some(ix) = target.profiles.iter().position(|p| p.id == id) else {
    warn!("Undo/redo: unknown profile id: {}", id);
    return;
  };
  target.profiles.remove(ix);
  cx.emit(ProjectVariablesEvent::ProfilesChanged);
  if target.active_profile == Some(id) {
    target.active_profile = None;
    cx.emit(ProjectVariablesEvent::ActiveProfile(None));
  }
  cx.notify();
}

fn move_profile_to(target: &mut ProjectVariables, from_ix: usize, to_ix: usize, cx: &mut Context<ProjectVariables>) {
  let profile = target.profiles.remove(from_ix);
  let to_ix = min(to_ix, target.profiles.len());
  target.profiles.insert(to_ix, profile);
  cx.emit(ProjectVariablesEvent::ProfilesChanged);
  cx.notify();
}

fn replace_profile(target: &mut ProjectVariables, profile: &Profile, cx: &mut Context<ProjectVariables>) {
  let Some(existing) = target.profiles.iter_mut().find(|p| p.id == profile.id) else {
    warn!("Undo/redo: unknown profile id: {}", profile.id);
    return;
  };
  *existing = profile.clone();
  cx.emit(ProjectVariablesEvent::ProfilesChanged);
  cx.notify();
}

fn insert_variable(
  target: &mut ProjectVariables,
  index: usize,
  variable: &VariableReference,
  cx: &mut Context<ProjectVariables>,
) {
  let ix = min(index, target.references.len());
  target.references.insert(ix, variable.clone());
  cx.emit(ProjectVariablesEvent::VariablesChanged);
  cx.notify();
}

fn remove_variable(target: &mut ProjectVariables, id: Uuid, cx: &mut Context<ProjectVariables>) {
  let Some(ix) = target.references.iter().position(|v| v.id == id) else {
    warn!("Undo/redo: unknown variable id: {}", id);
    return;
  };
  target.references.remove(ix);
  cx.emit(ProjectVariablesEvent::VariablesChanged);
  cx.notify();
}

fn move_variable_to(target: &mut ProjectVariables, from_ix: usize, to_ix: usize, cx: &mut Context<ProjectVariables>) {
  let variable = target.references.remove(from_ix);
  let to_ix = min(to_ix, target.references.len());
  target.references.insert(to_ix, variable);
  cx.emit(ProjectVariablesEvent::VariablesChanged);
  cx.notify();
}

fn replace_variable(target: &mut ProjectVariables, variable: &VariableReference, cx: &mut Context<ProjectVariables>) {
  let Some(existing) = target.references.iter_mut().find(|v| v.id == variable.id) else {
    warn!("Undo/redo: unknown variable id: {}", variable.id);
    return;
  };
  *existing = variable.clone();
  cx.emit(ProjectVariablesEvent::VariablesChanged);
  cx.notify();
}

/// Insertion of a profile (added or duplicated)
struct UndoAddProfile {
  profile: Profile,
  index: usize,
}

impl Edit<ProjectVariables> for UndoAddProfile {
  fn undo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    remove_profile(target, self.profile.id, cx);
  }

  fn redo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    insert_profile(target, self.index, &self.profile, cx);
  }
}

/// Deletion of a profile, remembering whether it was the active one
struct UndoDeleteProfile {
  profile: Profile,
  index: usize,
  was_active: bool,
}

impl Edit<ProjectVariables> for UndoDeleteProfile {
  fn undo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    insert_profile(target, self.index, &self.profile, cx);
    if self.was_active {
      target.active_profile = Some(self.profile.id);
      cx.emit(ProjectVariablesEvent::ActiveProfile(target.active_profile));
    }
  }

  fn redo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    remove_profile(target, self.profile.id, cx);
  }
}

/// Move of a profile from one position to another
struct UndoMoveProfile {
  from_ix: usize,
  to_ix: usize,
}

impl Edit<ProjectVariables> for UndoMoveProfile {
  fn undo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    move_profile_to(target, self.to_ix, self.from_ix, cx);
  }

  fn redo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    move_profile_to(target, self.from_ix, self.to_ix, cx);
  }
}

/// Modification of a profile's attributes
struct UndoUpdateProfile {
  before: Profile,
  after: Profile,
}

impl Edit<ProjectVariables> for UndoUpdateProfile {
  fn undo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    replace_profile(target, &self.before, cx);
  }

  fn redo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    replace_profile(target, &self.after, cx);
  }
}

/// Insertion of a variable (added or duplicated)
struct UndoAddVariable {
  variable: VariableReference,
  index: usize,
}

impl Edit<ProjectVariables> for UndoAddVariable {
  fn undo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    remove_variable(target, self.variable.id, cx);
  }

  fn redo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    insert_variable(target, self.index, &self.variable, cx);
  }
}

/// Deletion of a variable
struct UndoDeleteVariable {
  variable: VariableReference,
  index: usize,
}

impl Edit<ProjectVariables> for UndoDeleteVariable {
  fn undo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    insert_variable(target, self.index, &self.variable, cx);
  }

  fn redo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    remove_variable(target, self.variable.id, cx);
  }
}

/// Move of a variable from one position to another
struct UndoMoveVariable {
  from_ix: usize,
  to_ix: usize,
}

impl Edit<ProjectVariables> for UndoMoveVariable {
  fn undo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    move_variable_to(target, self.to_ix, self.from_ix, cx);
  }

  fn redo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    move_variable_to(target, self.from_ix, self.to_ix, cx);
  }
}

/// Modification of a variable's attributes, including its reference value and its per-profile overrides
struct UndoUpdateVariable {
  before: VariableReference,
  after: VariableReference,
}

impl Edit<ProjectVariables> for UndoUpdateVariable {
  fn undo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    replace_variable(target, &self.before, cx);
  }

  fn redo(&self, target: &mut ProjectVariables, cx: &mut Context<ProjectVariables>) {
    replace_variable(target, &self.after, cx);
  }
}
