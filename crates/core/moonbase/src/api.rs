use moon_utils::time::chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
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

// ARTIFACTS

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: i64,
    pub repository_id: i32,
    pub hash: String,
    pub size: i32,
    pub target: String,
    pub path: String,
    pub created_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactResponse {
    pub artifact: Artifact,
}
