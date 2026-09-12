use ki_project::{FileEndpoint, FileRestParameter, HttpMethod};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceEndpoint {
  pub id: Uuid,
  pub url: String,
  pub method: HttpMethod,
  pub query_parameters: Vec<FileRestParameter>,
  pub path_parameters: Vec<FileRestParameter>,
}

impl WorkspaceEndpoint {
  pub fn from_file(file_endpoints: &[FileEndpoint]) -> Vec<WorkspaceEndpoint> {
    file_endpoints
      .iter()
      .map(|endpoint| WorkspaceEndpoint {
        id: endpoint.id,
        url: endpoint.url.clone(),
        method: endpoint.method,
        query_parameters: endpoint.query_parameters.clone(),
        path_parameters: endpoint.path_parameters.clone(),
      })
      .collect()
  }

  pub fn to_file(&self) -> FileEndpoint {
    FileEndpoint {
      id: self.id,
      url: self.url.clone(),
      method: self.method,
      query_parameters: vec![],
      path_parameters: vec![],
    }
  }
}
