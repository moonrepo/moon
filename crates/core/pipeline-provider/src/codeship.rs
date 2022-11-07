use crate::api::{opt_var, var, PipelineEnvironment, PipelineProvider};

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: None,
        branch: var("CI_BRANCH"),
        id: var("CI_BUILD_ID"),
        provider: PipelineProvider::Codeship,
        request_id: opt_var("CI_PR_NUMBER"),
        request_url: opt_var("CI_PULL_REQUEST"),
        revision: var("CI_COMMIT_ID"),
        url: None,
    }
}
