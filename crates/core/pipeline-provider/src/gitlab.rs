use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    let base_branch = env::var("CI_MERGE_REQUEST_TARGET_BRANCH_NAME")
        .or_else(|| env::var("CI_EXTERNAL_PULL_REQUEST_TARGET_BRANCH_NAME"));
    let branch = env::var("CI_MERGE_REQUEST_SOURCE_BRANCH_NAME")
        .or_else(|| env::var("CI_EXTERNAL_PULL_REQUEST_SOURCE_BRANCH_NAME"))
        .or_else(|| env::var("CI_COMMIT_BRANCH"))
        .unwrap_or_default();

    PipelineEnvironment {
        base_branch: handle_falsy_value(base_branch),
        branch,
        id: env::var("CI_PIPELINE_ID").unwrap_or_default(),
        name: PipelineProvider::Gitlab,
        request_id: handle_falsy_value(env::var("CI_MERGE_REQUEST_ID")),
        request_url: None,
        revision: env::var("CI_COMMIT_SHA").unwrap_or_default(),
        url: handle_falsy_value(env::var("CI_PIPELINE_URL")),
    }
}
