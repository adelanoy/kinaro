use crate::FileProjectMetadata;
use crate::error::{ProjectError, ProjectResult};
use gpui_kit::component::select::SelectItem;
use gpui_kit::{Context, EventEmitter, SharedString};
use ki_project::{FileProfile, FileProjectVariables, FileVariable, FileVariableKind};
use ki_utils::id::KId;
use ki_utils::undo::{Edit, History};
use log::warn;
use std::cmp::{Ordering, min};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

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
  ActiveProfile(Option<KId>),
}

#[derive(Default)]
pub struct ProjectVariables {
  pub references: Vec<VariableReference>,
  pub profiles: Vec<Profile>,
  history_references: History<ProjectVariables>,
  history_profiles: History<ProjectVariables>,
  active_profile: Option<KId>,
}

impl ProjectVariables {
  ///// PROFILES
  /// Returns the id of the currently active profile, *None* when no profile is active
  #[inline]
  pub fn active_profile(&self) -> Option<KId> {
    self.active_profile
  }

  /// Adds a new empty profile named *new profile* (appended with *_X* if already taken), right after the row `row_ix`,
  /// or at the end of the list when *None*
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`]
  pub fn add_profile(&mut self, row_ix: Option<usize>, cx: &mut Context<Self>) {
    let name = ki_utils::next_available_name("new profile", self.profiles.iter().map(|p| p.name.clone()));
    let new_profile = Profile {
      id: KId::new(),
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

  /// Deletes a profile. If it was the active profile, no profile is active anymore.
  ///
  /// The variables overrides of this profile are kept in memory so that the deletion can be undone, but they are not
  /// part of the project file anymore
  /// # Result
  /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if the profile was deleted, followed by a
  /// [`ProjectVariablesEvent::ActiveProfile`] with *None* if it was the active profile
  pub fn delete_profile(&mut self, index: usize, cx: &mut Context<Self>) -> ProjectResult<()> {
    if self.profiles.is_empty() || index > self.profiles.len() - 1 {
      warn!("Failed to delete profile at row: {}", index);
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
  /// The variables overrides of the original profile are copied to the new profile.
  ///
  /// The copy is placed right after the original. A no-op if `index` is out of bound.
  ///
  /// Undoing the duplication only removes the profile: its overrides stay in memory, unreachable since the id is never
  /// reused, and are not saved
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if the profile was duplicated, followed by a
  /// [`ProjectVariablesEvent::VariablesChanged`] if at least one override was copied
  pub fn duplicate_profile(&mut self, index: usize, cx: &mut Context<Self>) {
    let Some(profile) = self.profiles.get(index) else {
      return;
    };
    let original_id = profile.id;
    let mut duplicated_profile = profile.clone();
    duplicated_profile.id = KId::new();
    duplicated_profile.name = ki_utils::next_available_name(&profile.name, self.profiles.iter().map(|p| p.name.clone()));
    let ix = min(index + 1, self.profiles.len());
    self.profiles.insert(ix, duplicated_profile.clone());

    let mut overrides_copied = false;
    for var in self.references.iter_mut() {
      if let Some(value) = var.overrides.get(&original_id).cloned() {
        var.overrides.insert(duplicated_profile.id, value);
        overrides_copied = true;
      }
    }

    self.history_profiles.push(Box::new(UndoAddProfile {
      profile: duplicated_profile,
      index: ix,
    }));
    cx.emit(ProjectVariablesEvent::ProfilesChanged);
    if overrides_copied {
      cx.emit(ProjectVariablesEvent::VariablesChanged);
    }
  }

  /// Moves a profile from one position to another. A no-op if both indexes are identical
  ///
  /// # Result
  /// Returns a [`ProjectError::OperationNotAllowed`] if either index is out of bound
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if the profile was moved
  pub fn move_profile(&mut self, from_ix: usize, to_ix: usize, cx: &mut Context<Self>) -> ProjectResult<()> {
    if from_ix == to_ix {
      return Ok(());
    }
    if from_ix >= self.profiles.len() || to_ix >= self.profiles.len() {
      return Err(ProjectError::OperationNotAllowed);
    }
    let profile_to_move = self.profiles.remove(from_ix);
    self.profiles.insert(to_ix, profile_to_move);
    self.history_profiles.push(Box::new(UndoMoveProfile { from_ix, to_ix }));
    cx.emit(ProjectVariablesEvent::ProfilesChanged);
    Ok(())
  }

  /// Replaces the profile at row `ix` with `updated_profile`. A no-op if nothing changed.
  ///
  /// The name is trimmed, and appended with *_X* (where X is an integer) if another profile already uses it
  ///
  /// # Result
  /// - Returns a [`ProjectError::InvalidProfileName`] if the trimmed name is empty
  /// - Returns a [`ProjectError::ProfileNotFound`] if `ix` is out of bound
  /// - Returns a [`ProjectError::OperationNotAllowed`] if the profile at row `ix` is not the one being updated (ids
  ///   differ)
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if the profile was modified
  pub fn update_profile(&mut self, mut updated_profile: Profile, ix: usize, cx: &mut Context<Self>) -> ProjectResult<()> {
    let sanitized_name = ki_utils::next_available_name(
      updated_profile.name.trim(),
      self
        .profiles
        .iter()
        .enumerate()
        .filter(|(i, _)| ix != *i)
        .map(|(_, p)| p.name.clone()),
    );
    if sanitized_name.is_empty() {
      return Err(ProjectError::InvalidProfileName(sanitized_name));
    }
    let Some(profile) = self.profiles.get_mut(ix) else {
      warn!("Attempt to edit non existing profile at index: {}", ix);
      return Err(ProjectError::ProfileNotFound);
    };
    if profile.id != updated_profile.id {
      warn!(
        "Attempt to edit a profile with wrong id - profile at index: {} with id: {}, provided id: {}",
        ix, profile.id, updated_profile.id
      );
      return Err(ProjectError::OperationNotAllowed);
    }

    if updated_profile != *profile {
      updated_profile.name = sanitized_name;
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
  pub fn switch_profile(&mut self, profile_id: Option<KId>, cx: &mut Context<Self>) {
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
  /// Adds a new empty text variable named *new variable* (appended with *_X* if already taken), right after the row
  /// `index`, or at the end of the list when *None*
  /// # Events
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`]
  pub fn add_variable(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
    let name = ki_utils::next_available_name("new variable", self.references.iter().map(|p| p.name.clone()));
    let new_var = VariableReference {
      id: KId::new(),
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
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if the variable was deleted
  pub fn delete_variable(&mut self, index: usize, cx: &mut Context<Self>) -> ProjectResult<()> {
    if self.references.is_empty() || index > self.references.len() - 1 {
      warn!("Failed to delete variable at row: {}", index,);
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

  /// Duplicates a variable, copying all fields including its overrides. Name is appended with *_X* where X is an
  /// integer.
  ///
  /// The copy is placed right after the original. A no-op if `index` is out of bound
  /// # Events
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if the variable was duplicated
  pub fn duplicate_variable(&mut self, index: usize, cx: &mut Context<Self>) {
    let Some(var) = self.references.get(index) else {
      return;
    };
    let mut duplicated_var = var.clone();
    duplicated_var.id = KId::new();
    duplicated_var.name = ki_utils::next_available_name(&var.name, self.references.iter().map(|p| p.name.clone()));
    let ix = min(index + 1, self.references.len());
    self.references.insert(ix, duplicated_var.clone());
    self.history_references.push(Box::new(UndoAddVariable {
      variable: duplicated_var,
      index: ix,
    }));
    cx.emit(ProjectVariablesEvent::VariablesChanged);
  }

  /// Replaces the variable at row `ix` with `updated_variable`, including its reference value and overrides. A no-op if
  /// nothing changed.
  ///
  /// As variables are referenced by name (`{{var.name}}`), the name is trimmed, and appended with *_X* (where X is an
  /// integer) if another variable already uses it
  ///
  /// # Result
  /// - Returns a [`ProjectError::InvalidVariableName`] if the trimmed name is empty or contains forbidden characters
  /// - Returns a [`ProjectError::VariableNotFound`] if `ix` is out of bound
  /// - Returns a [`ProjectError::OperationNotAllowed`] if the variable at row `ix` is not the one being updated
  /// # Events
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if the variable was modified
  pub fn update_variable(
    &mut self,
    mut updated_variable: VariableReference,
    ix: usize,
    cx: &mut Context<Self>,
  ) -> ProjectResult<()> {
    let sanitized_name = ki_utils::next_available_name(
      updated_variable.name.trim(),
      self
        .references
        .iter()
        .enumerate()
        .filter(|(i, _)| ix != *i)
        .map(|(_, p)| p.name.clone()),
    );
    if sanitized_name.is_empty() || !ki_utils::is_variable_name_valid(&sanitized_name) {
      return Err(ProjectError::InvalidVariableName(sanitized_name));
    }
    let Some(var) = self.references.get_mut(ix) else {
      warn!("Attempt to edit non existing variable at index: {}", ix);
      return Err(ProjectError::VariableNotFound);
    };
    if var.id != updated_variable.id {
      warn!(
        "Attempt to edit a variable with wrong id - var at index: {} with id: {}, provided id: {}",
        ix, var.id, updated_variable.id
      );
      return Err(ProjectError::OperationNotAllowed);
    }
    if updated_variable == *var {
      return Ok(());
    }
    // var names are referenced in steps, so they are trimmed and made unique
    updated_variable.name = sanitized_name;
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
  pub fn revert_override(&mut self, index: usize, profile_id: KId, cx: &mut Context<Self>) -> ProjectResult<()> {
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

  /// Moves a variable from one position to another. A no-op if both indexes are identical
  ///
  /// # Result
  /// Returns a [`ProjectError::OperationNotAllowed`] if either index is out of bound
  /// # Events
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if the variable was moved
  pub fn move_variable(&mut self, from_ix: usize, to_ix: usize, cx: &mut Context<Self>) -> ProjectResult<()> {
    if from_ix == to_ix {
      return Ok(());
    }
    if from_ix >= self.references.len() || to_ix >= self.references.len() {
      return Err(ProjectError::OperationNotAllowed);
    }
    let var_to_move = self.references.remove(from_ix);
    self.references.insert(to_ix, var_to_move);
    self.history_references.push(Box::new(UndoMoveVariable { from_ix, to_ix }));
    cx.emit(ProjectVariablesEvent::VariablesChanged);
    Ok(())
  }

  ///// CONVERSION
  /// Builds the project variables from their file representation. Overrides of unknown profiles are dropped, and the
  /// active profile is restored from `metadata` if it still exists
  pub(super) fn from_file(file_vars: FileProjectVariables, metadata: &FileProjectMetadata) -> Self {
    let profiles = file_vars.profiles.iter().map(Profile::from_file).collect::<Vec<Profile>>();
    let profile_ids: Vec<KId> = profiles.iter().map(|p| p.id).collect();
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

  /// Returns the file representation of the project variables. Overrides of deleted profiles are not included
  pub(super) fn to_file(&self) -> FileProjectVariables {
    let variables = self.references.iter().map(|p| p.to_file()).collect();
    let profiles = self.profiles.iter().map(|p| p.to_file()).collect();

    FileProjectVariables::new(variables, profiles)
  }

  ///// UNDO
  /// Reverts the last operation made on the profiles (add, duplicate, delete, move, update). A no-op if there is
  /// nothing to undo
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if an operation was reverted, followed by a
  /// [`ProjectVariablesEvent::ActiveProfile`] if the active profile was affected
  pub fn undo_profile(&mut self, cx: &mut Context<Self>) {
    if let Some(undo) = self.history_profiles.pop_undo() {
      undo.undo(self, cx);
      self.history_profiles.push_redo(undo);
    }
  }

  /// Reverts the last operation made on the variables (add, duplicate, delete, move, update, override revert). A no-op
  /// if there is nothing to undo
  /// # Events
  /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if an operation was reverted
  pub fn undo_reference(&mut self, cx: &mut Context<Self>) {
    if let Some(undo) = self.history_references.pop_undo() {
      undo.undo(self, cx);
      self.history_references.push_redo(undo);
    }
  }

  ///// REDO
  /// Re-applies the last profile operation reverted by [`ProjectVariables::undo_profile`]. A no-op if there is nothing
  /// to redo, which is the case as soon as a new profile operation is performed after an undo
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if an operation was re-applied, followed by a
  /// [`ProjectVariablesEvent::ActiveProfile`] if the active profile was affected
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
  /// Memory-only ID, regenerated on each load, to identify a variable across renames (undo/redo, stale edits)
  id: KId,
  pub name: SharedString,
  pub description: SharedString,
  pub kind: VariableKind,
  /// Reference value for this variable. to set a new value, [`VariableReference::set_value`] should be used to handle overrides
  pub value: SharedString,
  overrides: HashMap<KId, SharedString>,
}

impl VariableReference {
  fn from_file(file_vars: FileVariable, profiles: &[KId]) -> Self {
    Self {
      id: KId::new(),
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

  /// Returns the value this variable takes in the profile `profile_id`, *None* if the profile doesn't override it
  pub fn find_override(&self, profile_id: &KId) -> Option<SharedString> {
    self
      .overrides
      .iter()
      .find_map(|(id, value)| if id == profile_id { Some(value.clone()) } else { None })
  }

  /// Sets the reference value when `profile_id` is *None*, else the override of this profile. Setting an override to
  /// the reference value removes the override
  pub fn set_value(&mut self, value: SharedString, profile_id: Option<KId>) {
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
  pub id: KId,
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
  type Value = KId;

  fn title(&self) -> SharedString {
    self.name.clone()
  }

  fn value(&self) -> &Self::Value {
    &self.id
  }
}

#[derive(PartialEq, Clone, Debug)]
pub struct DraggedVar(pub usize);

#[derive(PartialEq, Clone, Debug)]
pub struct DraggedProfile(pub usize);

///// UNDO / REDO
fn insert_profile(target: &mut ProjectVariables, index: usize, profile: &Profile, cx: &mut Context<ProjectVariables>) {
  let ix = min(index, target.profiles.len());
  target.profiles.insert(ix, profile.clone());
  cx.emit(ProjectVariablesEvent::ProfilesChanged);
  cx.notify();
}

fn remove_profile(target: &mut ProjectVariables, id: KId, cx: &mut Context<ProjectVariables>) {
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

fn remove_variable(target: &mut ProjectVariables, id: KId, cx: &mut Context<ProjectVariables>) {
  let Some(ix) = target.references.iter().position(|v| v.id == id) else {
    warn!("Undo/redo: unknown variable");
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
    warn!("Undo/redo: unknown variable: {}", variable.name);
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

#[cfg(test)]
mod tests {
  use super::*;
  use gpui_kit::{AppContext, Entity, TestAppContext};
  use std::cell::RefCell;
  use std::rc::Rc;

  /// Id of the *dev* profile
  fn dev() -> KId {
    "11111111".parse().unwrap()
  }

  /// Id of the *prod* profile
  fn prod() -> KId {
    "22222222".parse().unwrap()
  }

  /// Loads 2 profiles (*dev*, *prod*) and 2 variables (*host* overridden by *dev*, *port*), with `active` as the active
  /// profile
  fn load(active: Option<KId>, cx: &mut TestAppContext) -> Entity<ProjectVariables> {
    let profile = |id, name: &str| FileProfile {
      id,
      name: name.to_string(),
      description: String::new(),
    };
    let variable = |name: &str, overrides: &[(KId, &str)]| FileVariable {
      name: name.to_string(),
      description: String::new(),
      kind: FileVariableKind::Text,
      value: format!("{name}_value"),
      overrides: overrides.iter().map(|(id, v)| (*id, v.to_string())).collect(),
    };
    let file = FileProjectVariables::new(
      vec![variable("host", &[(dev(), "localhost")]), variable("port", &[])],
      vec![profile(dev(), "dev"), profile(prod(), "prod")],
    );
    let metadata = FileProjectMetadata {
      active_profile: active,
      ..Default::default()
    };
    cx.new(|_| ProjectVariables::from_file(file, &metadata))
  }

  /// Records the kind of every event emitted by `vars` from now on
  fn record_events(vars: &Entity<ProjectVariables>, cx: &mut TestAppContext) -> Rc<RefCell<Vec<&'static str>>> {
    let events = Rc::new(RefCell::new(vec![]));
    cx.update(|cx| {
      let events = events.clone();
      cx.subscribe(vars, move |_, event: &ProjectVariablesEvent, _| {
        events.borrow_mut().push(match event {
          ProjectVariablesEvent::ProfilesChanged => "profiles",
          ProjectVariablesEvent::VariablesChanged => "variables",
          ProjectVariablesEvent::ActiveProfile(_) => "active",
        })
      })
      .detach();
    });
    events
  }

  fn var(vars: &Entity<ProjectVariables>, ix: usize, cx: &TestAppContext) -> VariableReference {
    vars.read_with(cx, |vars, _| vars.references[ix].clone())
  }

  fn profile(vars: &Entity<ProjectVariables>, ix: usize, cx: &TestAppContext) -> Profile {
    vars.read_with(cx, |vars, _| vars.profiles[ix].clone())
  }

  fn var_names(vars: &Entity<ProjectVariables>, cx: &TestAppContext) -> Vec<SharedString> {
    vars.read_with(cx, |vars, _| vars.references.iter().map(|v| v.name.clone()).collect())
  }

  fn profile_names(vars: &Entity<ProjectVariables>, cx: &TestAppContext) -> Vec<SharedString> {
    vars.read_with(cx, |vars, _| vars.profiles.iter().map(|p| p.name.clone()).collect())
  }

  fn update_var(
    vars: &Entity<ProjectVariables>,
    ix: usize,
    f: impl FnOnce(&mut VariableReference),
    cx: &mut TestAppContext,
  ) -> ProjectResult<()> {
    let mut edited = var(vars, ix, cx);
    f(&mut edited);
    vars.update(cx, |vars, cx| vars.update_variable(edited, ix, cx))
  }

  fn update_profile(
    vars: &Entity<ProjectVariables>,
    ix: usize,
    f: impl FnOnce(&mut Profile),
    cx: &mut TestAppContext,
  ) -> ProjectResult<()> {
    let mut edited = profile(vars, ix, cx);
    f(&mut edited);
    vars.update(cx, |vars, cx| vars.update_profile(edited, ix, cx))
  }

  ///// LOADING / SAVING
  #[gpui_kit::test]
  fn from_file_restores_the_active_profile_if_it_exists(cx: &mut TestAppContext) {
    let vars = load(Some(prod()), cx);
    assert_eq!(vars.read_with(cx, |vars, _| vars.active_profile()), Some(prod()));
    let vars = load(Some("99999999".parse::<KId>().unwrap()), cx);
    assert_eq!(vars.read_with(cx, |vars, _| vars.active_profile()), None);
  }

  #[gpui_kit::test]
  fn to_file_drops_overrides_of_deleted_profiles(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    vars.update(cx, |vars, cx| vars.delete_profile(0, cx)).unwrap();
    // Still in memory, for undo
    assert_eq!(var(&vars, 0, cx).find_override(&dev()), Some("localhost".into()));
    let file = vars.read_with(cx, |vars, _| vars.to_file());
    assert!(file.variables.iter().all(|v| v.overrides.is_empty()), "{:?}", file.variables);
  }

  #[gpui_kit::test]
  fn variables_ids_are_unique(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    vars.update(cx, |vars, cx| vars.add_variable(None, cx));
    vars.update(cx, |vars, cx| vars.duplicate_variable(0, cx));
    let mut ids = vars.read_with(cx, |vars, _| vars.references.iter().map(|v| v.id).collect::<Vec<_>>());
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 4);
  }

  ///// VARIABLES
  #[gpui_kit::test]
  fn update_variable_keeps_the_name_when_editing_other_fields(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    let events = record_events(&vars, cx);
    update_var(&vars, 0, |v| v.description = "The host".into(), cx).unwrap();
    assert_eq!(var(&vars, 0, cx).name, "host");
    assert_eq!(var(&vars, 0, cx).description, "The host");
    assert_eq!(*events.borrow(), ["variables"]);
  }

  #[gpui_kit::test]
  fn update_variable_trims_the_name(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    update_var(&vars, 0, |v| v.name = "  hostname ".into(), cx).unwrap();
    assert_eq!(var_names(&vars, cx), ["hostname", "port"]);
  }

  #[gpui_kit::test]
  fn update_variable_makes_a_taken_name_unique(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    update_var(&vars, 0, |v| v.name = "port".into(), cx).unwrap();
    assert_eq!(var_names(&vars, cx), ["port_1", "port"]);
  }

  #[gpui_kit::test]
  fn update_variable_rejects_invalid_names(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    let events = record_events(&vars, cx);
    for name in ["", "   ", "my-var", "{{var.host}}"] {
      let result = update_var(&vars, 0, |v| v.name = name.into(), cx);
      assert!(
        matches!(result, Err(ProjectError::InvalidVariableName(_))),
        "{name:?}: {result:?}"
      );
    }
    assert_eq!(var_names(&vars, cx), ["host", "port"]);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn update_variable_rejects_a_stale_row(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    // The variable at row 0 is submitted for row 1, e.g. after a move during the edition
    let mut edited = var(&vars, 0, cx);
    edited.value = "new".into();
    let result = vars.update(cx, |vars, cx| vars.update_variable(edited, 1, cx));
    assert_eq!(result, Err(ProjectError::OperationNotAllowed));
    assert_eq!(var(&vars, 1, cx).value, "port_value");
  }

  #[gpui_kit::test]
  fn update_variable_rejects_an_unknown_row(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    let edited = var(&vars, 0, cx);
    let result = vars.update(cx, |vars, cx| vars.update_variable(edited, 5, cx));
    assert_eq!(result, Err(ProjectError::VariableNotFound));
  }

  #[gpui_kit::test]
  fn update_variable_without_change_is_a_no_op(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    let events = record_events(&vars, cx);
    update_var(&vars, 0, |_| {}, cx).unwrap();
    assert!(events.borrow().is_empty());
    // Nothing was pushed to the history
    vars.update(cx, |vars, cx| vars.undo_reference(cx));
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn variable_rename_can_be_undone_and_redone(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    update_var(
      &vars,
      0,
      |v| {
        v.name = "hostname".into();
        v.value = "example.com".into();
      },
      cx,
    )
    .unwrap();
    vars.update(cx, |vars, cx| vars.undo_reference(cx));
    assert_eq!(var(&vars, 0, cx).name, "host");
    assert_eq!(var(&vars, 0, cx).value, "host_value");
    vars.update(cx, |vars, cx| vars.redo_reference(cx));
    assert_eq!(var(&vars, 0, cx).name, "hostname");
    assert_eq!(var(&vars, 0, cx).value, "example.com");
  }

  #[gpui_kit::test]
  fn added_then_renamed_variable_is_removed_by_undo(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    vars.update(cx, |vars, cx| vars.add_variable(None, cx));
    update_var(&vars, 2, |v| v.name = "token".into(), cx).unwrap();
    vars.update(cx, |vars, cx| vars.undo_reference(cx));
    assert_eq!(var_names(&vars, cx), ["host", "port", "new variable"]);
    vars.update(cx, |vars, cx| vars.undo_reference(cx));
    assert_eq!(var_names(&vars, cx), ["host", "port"]);
  }

  #[gpui_kit::test]
  fn duplicated_variable_is_independent_from_the_original(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    vars.update(cx, |vars, cx| vars.duplicate_variable(0, cx));
    assert_eq!(var_names(&vars, cx), ["host", "host_1", "port"]);
    let (original, copy) = (var(&vars, 0, cx), var(&vars, 1, cx));
    assert_ne!(original.id, copy.id);
    assert_eq!(copy.find_override(&dev()), Some("localhost".into()));

    update_var(&vars, 1, |v| v.value = "copy".into(), cx).unwrap();
    vars.update(cx, |vars, cx| vars.undo_reference(cx));
    assert_eq!(var(&vars, 1, cx).value, "host_value");
    vars.update(cx, |vars, cx| vars.redo_reference(cx));
    assert_eq!(var(&vars, 0, cx), original);
    assert_eq!(var(&vars, 1, cx).value, "copy");
  }

  #[gpui_kit::test]
  fn variable_operations_on_an_empty_list_fail_without_panicking(cx: &mut TestAppContext) {
    let vars = cx.new(|_| ProjectVariables::default());
    assert_eq!(
      vars.update(cx, |vars, cx| vars.delete_variable(0, cx)),
      Err(ProjectError::VariableNotFound)
    );
    assert_eq!(
      vars.update(cx, |vars, cx| vars.move_variable(0, 1, cx)),
      Err(ProjectError::OperationNotAllowed)
    );
    assert_eq!(
      vars.update(cx, |vars, cx| vars.revert_override(0, dev(), cx)),
      Err(ProjectError::VariableNotFound)
    );
    vars.update(cx, |vars, cx| vars.duplicate_variable(0, cx));
    assert!(var_names(&vars, cx).is_empty());
  }

  ///// PROFILES
  #[gpui_kit::test]
  fn update_profile_keeps_the_name_when_editing_other_fields(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    let events = record_events(&vars, cx);
    update_profile(&vars, 0, |p| p.description = "Local".into(), cx).unwrap();
    assert_eq!(profile(&vars, 0, cx).name, "dev");
    assert_eq!(profile(&vars, 0, cx).description, "Local");
    assert_eq!(*events.borrow(), ["profiles"]);
  }

  #[gpui_kit::test]
  fn update_profile_renames_trims_and_makes_unique(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    update_profile(&vars, 0, |p| p.name = " local ".into(), cx).unwrap();
    assert_eq!(profile_names(&vars, cx), ["local", "prod"]);
    update_profile(&vars, 1, |p| p.name = "local".into(), cx).unwrap();
    assert_eq!(profile_names(&vars, cx), ["local", "local_1"]);
    // Overrides follow the profile id, not its name
    assert_eq!(var(&vars, 0, cx).find_override(&dev()), Some("localhost".into()));
  }

  #[gpui_kit::test]
  fn update_profile_rejects_blank_names(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    for name in ["", "   "] {
      let result = update_profile(&vars, 0, |p| p.name = name.into(), cx);
      assert!(
        matches!(result, Err(ProjectError::InvalidProfileName(_))),
        "{name:?}: {result:?}"
      );
    }
    assert_eq!(profile_names(&vars, cx), ["dev", "prod"]);
  }

  #[gpui_kit::test]
  fn update_profile_rejects_a_stale_or_unknown_row(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    let edited = profile(&vars, 0, cx);
    let result = vars.update(cx, |vars, cx| vars.update_profile(edited.clone(), 1, cx));
    assert_eq!(result, Err(ProjectError::OperationNotAllowed));
    let result = vars.update(cx, |vars, cx| vars.update_profile(edited, 5, cx));
    assert_eq!(result, Err(ProjectError::ProfileNotFound));
    assert_eq!(profile_names(&vars, cx), ["dev", "prod"]);
  }

  #[gpui_kit::test]
  fn profile_rename_can_be_undone_and_redone(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    update_profile(&vars, 0, |p| p.name = "local".into(), cx).unwrap();
    vars.update(cx, |vars, cx| vars.undo_profile(cx));
    assert_eq!(profile_names(&vars, cx), ["dev", "prod"]);
    vars.update(cx, |vars, cx| vars.redo_profile(cx));
    assert_eq!(profile_names(&vars, cx), ["local", "prod"]);
  }

  #[gpui_kit::test]
  fn duplicated_profile_gets_a_new_id(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    vars.update(cx, |vars, cx| vars.duplicate_profile(0, cx));
    assert_eq!(profile_names(&vars, cx), ["dev", "dev_1", "prod"]);
    let copy = profile(&vars, 1, cx);
    assert_ne!(copy.id, dev());
    // Undoing the duplication removes the copy, not the original
    vars.update(cx, |vars, cx| vars.undo_profile(cx));
    assert_eq!(profile_names(&vars, cx), ["dev", "prod"]);
    assert_eq!(profile(&vars, 0, cx).id, dev());
  }

  #[gpui_kit::test]
  fn duplicated_profile_copies_the_overrides(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    let events = record_events(&vars, cx);
    vars.update(cx, |vars, cx| vars.duplicate_profile(0, cx));
    assert_eq!(*events.borrow(), ["profiles", "variables"]);
    let copy = profile(&vars, 1, cx).id;
    assert_eq!(var(&vars, 0, cx).find_override(&copy), Some("localhost".into()));
    assert_eq!(var(&vars, 1, cx).find_override(&copy), None);

    // The copy is independent from the original
    update_var(&vars, 0, |v| v.set_value("copy.local".into(), Some(copy)), cx).unwrap();
    assert_eq!(var(&vars, 0, cx).find_override(&dev()), Some("localhost".into()));
    assert_eq!(var(&vars, 0, cx).find_override(&copy), Some("copy.local".into()));

    // Saved with the project
    let file = vars.read_with(cx, |vars, _| vars.to_file());
    assert_eq!(file.variables[0].overrides.get(&copy).map(String::as_str), Some("copy.local"));
  }

  #[gpui_kit::test]
  fn undone_profile_duplication_is_not_saved_and_redo_restores_overrides(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    vars.update(cx, |vars, cx| vars.duplicate_profile(0, cx));
    let copy = profile(&vars, 1, cx).id;
    vars.update(cx, |vars, cx| vars.undo_profile(cx));
    let file = vars.read_with(cx, |vars, _| vars.to_file());
    assert!(!file.variables[0].overrides.contains_key(&copy));

    vars.update(cx, |vars, cx| vars.redo_profile(cx));
    assert_eq!(profile(&vars, 1, cx).id, copy);
    assert_eq!(var(&vars, 0, cx).find_override(&copy), Some("localhost".into()));
  }

  #[gpui_kit::test]
  fn duplicating_a_profile_without_overrides_only_changes_profiles(cx: &mut TestAppContext) {
    let vars = load(None, cx);
    let events = record_events(&vars, cx);
    vars.update(cx, |vars, cx| vars.duplicate_profile(1, cx));
    assert_eq!(profile_names(&vars, cx), ["dev", "prod", "prod_1"]);
    assert_eq!(*events.borrow(), ["profiles"]);
  }

  #[gpui_kit::test]
  fn deleting_the_active_profile_deactivates_it_until_undone(cx: &mut TestAppContext) {
    let vars = load(Some(dev()), cx);
    let events = record_events(&vars, cx);
    vars.update(cx, |vars, cx| vars.delete_profile(0, cx)).unwrap();
    assert_eq!(vars.read_with(cx, |vars, _| vars.active_profile()), None);
    assert_eq!(*events.borrow(), ["profiles", "active"]);

    vars.update(cx, |vars, cx| vars.undo_profile(cx));
    assert_eq!(profile_names(&vars, cx), ["dev", "prod"]);
    assert_eq!(vars.read_with(cx, |vars, _| vars.active_profile()), Some(dev()));
    let file = vars.read_with(cx, |vars, _| vars.to_file());
    assert_eq!(file.variables[0].overrides.get(&dev()).map(String::as_str), Some("localhost"));
  }

  #[gpui_kit::test]
  fn profile_operations_on_an_empty_list_fail_without_panicking(cx: &mut TestAppContext) {
    let vars = cx.new(|_| ProjectVariables::default());
    assert_eq!(
      vars.update(cx, |vars, cx| vars.delete_profile(0, cx)),
      Err(ProjectError::ProfileNotFound)
    );
    assert_eq!(
      vars.update(cx, |vars, cx| vars.move_profile(0, 1, cx)),
      Err(ProjectError::OperationNotAllowed)
    );
    vars.update(cx, |vars, cx| vars.duplicate_profile(0, cx));
    assert!(profile_names(&vars, cx).is_empty());
  }
}
