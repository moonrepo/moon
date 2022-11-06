use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: handle_falsy_value(env::var("BUILDKITE_PULL_REQUEST_BASE_BRANCH")),
        branch: env::var("BUILDKITE_BRANCH").unwrap_or_default(),
        id: env::var("BUILDKITE_BUILD_ID").unwrap_or_default(),
        name: PipelineProvider::Buildkite,
        request_id: handle_falsy_value(env::var("BUILDKITE_PULL_REQUEST")),
        request_url: handle_falsy_value(env::var("BUILDKITE_PULL_REQUEST_REPO")),
        revision: env::var("BUILDKITE_COMMIT").unwrap_or_default(),
        url: env::var("BUILDKITE_BUILD_URL").unwrap_or_default(),
    }
}
