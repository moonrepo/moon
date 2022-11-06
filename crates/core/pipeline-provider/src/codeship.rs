use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: None,
        branch: env::var("CI_BRANCH").unwrap_or_default(),
        id: env::var("CI_BUILD_ID").unwrap_or_default(),
        name: PipelineProvider::Codeship,
        request_id: handle_falsy_value(env::var("CI_PR_NUMBER")),
        request_url: handle_falsy_value(env::var("CI_PULL_REQUEST")),
        revision: env::var("CI_COMMIT_ID").unwrap_or_default(),
        url: None,
    }
}
