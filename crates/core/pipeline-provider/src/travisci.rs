use crate::api::{opt_var, var, PipelineEnvironment, PipelineProvider};

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: opt_var("TRAVIS_PULL_REQUEST_BRANCH"),
        branch: var("TRAVIS_BRANCH"),
        id: var("TRAVIS_BUILD_ID"),
        provider: PipelineProvider::TravisCI,
        request_id: opt_var("TRAVIS_PULL_REQUEST"),
        request_url: None,
        revision: var("TRAVIS_COMMIT"),
        url: opt_var("TRAVIS_BUILD_WEB_URL"),
    }
}
