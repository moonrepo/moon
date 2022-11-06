use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: handle_falsy_value(env::var("CF_PULL_REQUEST_TARGET")),
        branch: env::var("CF_BRANCH").unwrap_or_default(),
        id: env::var("CF_BUILD_ID").unwrap_or_default(),
        name: PipelineProvider::Codefresh,
        request_id: handle_falsy_value(
            env::var("CF_PULL_REQUEST_NUMBER").or_else(|| env::var("CF_PULL_REQUEST_ID")),
        ),
        request_url: None,
        revision: env::var("CF_REVISION").unwrap_or_default(),
        url: handle_falsy_value(env::var("CF_BUILD_URL")),
    }
}
