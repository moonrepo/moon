use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SigninBody {
    pub organization_key: String,
    pub repository: String,
    pub repository_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SigninResponse {
    pub organization_id: i32,
    pub repository_id: i32,
    pub token: String,
}
