use crate::api::{opt_var, var, PipelineEnvironment, PipelineProvider};

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: opt_var("CF_PULL_REQUEST_TARGET").or_else(|| opt_var("CF_BASE_BRANCH")),
        branch: var("CF_BRANCH"),
        id: var("CF_BUILD_ID"),
        provider: PipelineProvider::Codefresh,
        request_id: opt_var("CF_PULL_REQUEST_NUMBER").or_else(|| opt_var("CF_PULL_REQUEST_ID")),
        request_url: None,
        revision: var("CF_REVISION"),
        url: opt_var("CF_BUILD_URL"),
    }
}
