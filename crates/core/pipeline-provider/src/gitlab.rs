use crate::api::{opt_var, var, PipelineEnvironment, PipelineProvider};

pub fn create_environment() -> PipelineEnvironment {
    let base_branch = opt_var("CI_MERGE_REQUEST_TARGET_BRANCH_NAME")
        .or_else(|| opt_var("CI_EXTERNAL_PULL_REQUEST_TARGET_BRANCH_NAME"));
    let branch = opt_var("CI_MERGE_REQUEST_SOURCE_BRANCH_NAME")
        .or_else(|| opt_var("CI_EXTERNAL_PULL_REQUEST_SOURCE_BRANCH_NAME"))
        .or_else(|| opt_var("CI_COMMIT_BRANCH"))
        .unwrap_or_default();

    PipelineEnvironment {
        base_branch,
        branch,
        id: var("CI_PIPELINE_ID"),
        provider: PipelineProvider::Gitlab,
        request_id: opt_var("CI_MERGE_REQUEST_ID"),
        request_url: None,
        revision: var("CI_COMMIT_SHA"),
        url: opt_var("CI_PIPELINE_URL"),
    }
}
