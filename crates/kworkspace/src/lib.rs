pub(crate) mod endpoint;
pub(crate) mod error;
pub(crate) mod project;
pub(crate) mod test;
pub(crate) mod variable;

pub use {
    error::{ProjectError, Result, TestError, WorkspaceError},
    project::{WorkspaceProject, WorkspaceProjectData},
    test::test_case::WorkspaceTestCase,
    test::test_step::WorkspaceTestStep,
    test::test_suite::WorkspaceTestSuite,
    test::{WorkspaceTestInfo, WorkspaceTestsContainer},
    variable::{WorkspaceProfile, WorkspaceVariable, WorkspaceVariableKind, WorkspaceVariables},
    workspace::Workspace,
};

mod workspace {
    use crate::error::ProjectError;
    use crate::project::WorkspaceProjectDataStatus;
    use crate::project::WorkspaceProjectDataStatus::{ExternallyModified, LoadError, Loaded};
    use crate::{Result, WorkspaceError, WorkspaceProject};
    use WorkspaceProjectDataStatus::{Moved, Unloaded};
    use gpui::{AppContext, Context, Entity};
    use serde::{Deserialize, Serialize};
    use settings::GlobalSettings;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    pub const WORKSPACES_FILENAME: &str = "workspaces.json";

    #[derive(Serialize, Deserialize, Default)]
    pub struct Workspace {
        active_project: Option<Uuid>,
        pub projects: Vec<WorkspaceProject>,
    }

    impl Workspace {
        pub fn init(cx: &mut Context<Self>) -> Self {
            let file_path = cx.read_global(|settings: &GlobalSettings, _| {
                settings.config_dir.join(WORKSPACES_FILENAME)
            });

            let mut workspace = if file_path.exists() {
                fs::read(&file_path)
                    .map_err(|e| WorkspaceError::Io(e))
                    .and_then(|file| {
                        serde_json::from_slice::<Workspace>(&file)
                            .map_err(|err| WorkspaceError::Read(err))
                    })
                    // If a project_error occurred while reading, create a new one
                    .unwrap_or_default()
            } else {
                Workspace::default()
            };

            workspace
                .projects
                .iter_mut()
                .for_each(|project| project.load(cx));

            workspace
        }

        pub fn save(&mut self, cx: &mut Context<Self>) -> Result<()> {
            let config_dir = cx.read_global(|settings: &GlobalSettings, _| {
                settings.config_dir.clone()
            });
            let file_path = config_dir.join(WORKSPACES_FILENAME);

            fs::create_dir_all(&config_dir)
                .map_err(|e| WorkspaceError::Io(e))
                .and_then(|_| fs::File::create(&file_path).map_err(|e| WorkspaceError::Io(e)))
                .and_then(|file| {
                    serde_json::to_writer_pretty(file, self)
                        .map_err(|err| WorkspaceError::Write(err))
                })?;

            self.save_active_project()
        }

        pub fn get_active_project(&self) -> Option<&WorkspaceProject> {
            if let Some(active_project) = self.active_project {
                self.projects
                    .iter()
                    .find(|p| p.id == active_project && p.is_loaded())
            } else {
                None
            }
        }

        pub fn get_active_project_id(&self) -> Option<Uuid> {
            self.active_project.clone()
        }

        pub fn get_active_project_mut(&mut self) -> Option<&mut WorkspaceProject> {
            if let Some(active_project) = self.active_project {
                self.projects
                    .iter_mut()
                    .find(|p| p.id == active_project && p.is_loaded())
            } else {
                None
            }
        }

        pub fn switch_project(&mut self, project_id: Uuid) -> Result<()> {
            self.save_active_project()?;

            let info = self
                .projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or(WorkspaceError::from(ProjectError::UnknownProject(
                    project_id,
                )))?;
            match info.get_status() {
                Unloaded => Err(WorkspaceError::General("Unknown".to_string())),
                Loaded(_) => {
                    self.active_project = Some(project_id);
                    Ok(())
                }
                Moved => Err(WorkspaceError::from(ProjectError::BadLocation(
                    info.path.clone(),
                ))),
                ExternallyModified => {
                    Err(WorkspaceError::Project(ProjectError::ExternallyModified))
                }
                LoadError(_) => Err(WorkspaceError::Project(ProjectError::Invalid)),
            }
        }

        pub fn remove_project(&mut self, project_id: Uuid) {
            let project_pos = self
                .projects
                .iter()
                .position(|p| p.id == project_id)
                .unwrap();
            let mut project = self.projects.remove(project_pos);
            let _ = project.save();

            // If the project was active, switch to the next project, if any
            if self.active_project == Some(project.id) {
                self.active_project = self.projects.iter().next().map_or(None, |p| Some(p.id));
            }
        }

        pub fn open_project(&mut self, path: PathBuf) -> Result<()> {
            if let Some(pos) = self.projects.iter().position(|p| p.path == path) {
                return self.switch_project(self.projects[pos].id);
            }
            let project = WorkspaceProject::open(path)?;
            self.active_project = Some(project.id);
            self.projects.push(project);

            Ok(())
        }

        pub fn create_project(&mut self, name: String, path: PathBuf) -> Result<()> {
            if let Some(pos) = self.projects.iter().position(|p| p.path == path) {
                return self.switch_project(self.projects[pos].id);
            }
            let project = WorkspaceProject::create(path, name)?;
            self.active_project = Some(project.id);
            self.projects.push(project);

            Ok(())
        }

        fn save_active_project(&mut self) -> Result<()> {
            if let Some(project) = self.get_active_project_mut() {
                if project.is_loaded() {
                    project.save()
                } else {
                    Err(WorkspaceError::Project(ProjectError::Invalid))
                }
            } else {
                Ok(())
            }
        }
    }
}
