mod api;
mod buildkite;
mod circleci;
mod github;
mod travisci;

use api::PipelineEnvironment;
use std::env;

pub fn get_pipeline_environment() -> PipelineEnvironment {
    if env::var("BUILDKITE").is_ok() {
        return buildkite::create_environment();
    }

    if env::var("CIRCLECI").is_ok() {
        return circleci::create_environment();
    }

    if env::var("GITHUB_ACTIONS").is_ok() {
        return github::create_environment();
    }

    if env::var("TRAVIS").is_ok() {
        return travisci::create_environment();
    }

    PipelineEnvironment::default()
}
