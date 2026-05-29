use project_file::{Endpoint, HttpMethod, RestParameter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEndpoint {
    pub id: Uuid,
    pub url: String,
    pub method: HttpMethod,
    pub query_parameters: Vec<RestParameter>,
    pub path_parameters: Vec<RestParameter>,
}

impl WorkspaceEndpoint {
    pub fn from_file(file_endpoints: &Vec<Endpoint>) -> Vec<WorkspaceEndpoint> {
        file_endpoints
            .iter()
            .map(|endpoint| WorkspaceEndpoint {
                id: endpoint.id,
                url: endpoint.url.clone(),
                method: endpoint.method.clone(),
                query_parameters: endpoint.query_parameters.clone(),
                path_parameters: endpoint.path_parameters.clone(),
            })
            .collect()
    }

    pub fn get_file(&self) -> Endpoint {
        Endpoint {
            id: self.id,
            url: self.url.clone(),
            method: self.method.clone(),
            query_parameters: vec![],
            path_parameters: vec![],
        }
    }
}
