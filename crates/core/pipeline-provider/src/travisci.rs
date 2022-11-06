use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: handle_falsy_value(env::var("TRAVIS_PULL_REQUEST_BRANCH")),
        branch: env::var("TRAVIS_BRANCH").unwrap_or_default(),
        id: env::var("TRAVIS_BUILD_ID").unwrap_or_default(),
        name: PipelineProvider::TravisCI,
        request_id: handle_falsy_value(env::var("TRAVIS_PULL_REQUEST")),
        request_url: None,
        revision: env::var("TRAVIS_COMMIT").unwrap_or_default(),
        url: handle_falsy_value(env::var("TRAVIS_BUILD_WEB_URL")),
    }
}
