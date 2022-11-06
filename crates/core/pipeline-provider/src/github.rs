use crate::api::{opt_var, var, PipelineEnvironment, PipelineProvider};

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: opt_var("GITHUB_BASE_REF"),
        branch: var("GITHUB_HEAD_REF"),
        id: var("GITHUB_RUN_ID"),
        provider: PipelineProvider::GithubActions,
        request_id: None,
        request_url: None,
        revision: var("GITHUB_SHA"),
        url: None,
    }
}
