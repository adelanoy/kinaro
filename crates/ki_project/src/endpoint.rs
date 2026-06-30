use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileEndpoint {
    pub id: Uuid,
    pub url: String,
    pub method: HttpMethod,
    pub query_parameters: Vec<FileRestParameter>,
    pub path_parameters: Vec<FileRestParameter>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FileRestParameter {
    pub param: String,
    pub value: String,
}