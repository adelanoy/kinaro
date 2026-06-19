use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub id: Uuid,
    pub url: String,
    pub method: HttpMethod,
    pub query_parameters: Vec<RestParameter>,
    pub path_parameters: Vec<RestParameter>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RestParameter {
    pub param: String,
    pub value: String,
}