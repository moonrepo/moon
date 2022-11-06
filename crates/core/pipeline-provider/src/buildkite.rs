use crate::api::{opt_var, var, PipelineEnvironment, PipelineProvider};

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: opt_var("BUILDKITE_PULL_REQUEST_BASE_BRANCH"),
        branch: var("BUILDKITE_BRANCH"),
        id: var("BUILDKITE_BUILD_ID"),
        provider: PipelineProvider::Buildkite,
        request_id: opt_var("BUILDKITE_PULL_REQUEST"),
        request_url: None,
        revision: var("BUILDKITE_COMMIT"),
        url: opt_var("BUILDKITE_BUILD_URL"),
    }
}
