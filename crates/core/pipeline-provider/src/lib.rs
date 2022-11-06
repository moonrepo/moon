mod api;
mod appveyor;
mod bitbucket;
mod buildkite;
mod circleci;
mod github;
mod travisci;

use api::PipelineEnvironment;
use std::env;

pub fn get_pipeline_environment() -> Option<PipelineEnvironment> {
    if env::var("APPVEYOR").is_ok() {
        return Some(appveyor::create_environment());
    }

    if env::var("BITBUCKET_WORKSPACE").is_ok() {
        return Some(bitbucket::create_environment());
    }

    if env::var("BUILDKITE").is_ok() {
        return Some(buildkite::create_environment());
    }

    if env::var("CIRCLECI").is_ok() {
        return Some(circleci::create_environment());
    }

    if env::var("GITHUB_ACTIONS").is_ok() {
        return Some(github::create_environment());
    }

    if env::var("TRAVIS").is_ok() {
        return Some(travisci::create_environment());
    }

    None
}
