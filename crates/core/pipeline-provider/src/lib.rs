mod api;
mod appveyor;
mod bitbucket;
mod buildkite;
mod circleci;
mod codefresh;
mod codeship;
mod drone;
mod github;
mod gitlab;
mod google_cloud_build;
mod semaphore;
mod travisci;

pub use api::{PipelineEnvironment, PipelineOutput, PipelineProvider};
use std::env;

pub fn detect_pipeline_provider() -> PipelineProvider {
    if env::var("APPVEYOR").is_ok() {
        return PipelineProvider::AppVeyor;
    }

    if env::var("BITBUCKET_WORKSPACE").is_ok() {
        return PipelineProvider::Bitbucket;
    }

    if env::var("BUILDKITE").is_ok() {
        return PipelineProvider::Buildkite;
    }

    if env::var("CIRCLECI").is_ok() {
        return PipelineProvider::CircleCI;
    }

    if env::var("CF_ACCOUNT").is_ok() {
        return PipelineProvider::Codefresh;
    }

    if let Ok(var) = env::var("CI_NAME") {
        if var == "codeship" {
            return PipelineProvider::Codeship;
        }
    }

    if env::var("DRONE").is_ok() {
        return PipelineProvider::Drone;
    }

    if env::var("GITHUB_ACTIONS").is_ok() {
        return PipelineProvider::GithubActions;
    }

    if env::var("GITLAB_CI").is_ok() {
        return PipelineProvider::Gitlab;
    }

    if env::var("GOOGLE_CLOUD_BUILD").is_ok() || env::var("BUILD_OUTPUT").is_ok() {
        return PipelineProvider::GoogleCloudBuild;
    }

    if env::var("SEMAPHORE").is_ok() {
        return PipelineProvider::Semaphore;
    }

    if env::var("TRAVIS").is_ok() {
        return PipelineProvider::TravisCI;
    }

    PipelineProvider::Unknown
}

pub fn get_pipeline_environment() -> Option<PipelineEnvironment> {
    if env::var("CI").is_err() {
        return None;
    }

    let environment = match detect_pipeline_provider() {
        PipelineProvider::AppVeyor => appveyor::create_environment(),
        PipelineProvider::Bitbucket => bitbucket::create_environment(),
        PipelineProvider::Buildkite => buildkite::create_environment(),
        PipelineProvider::CircleCI => circleci::create_environment(),
        PipelineProvider::Codefresh => codefresh::create_environment(),
        PipelineProvider::Codeship => codeship::create_environment(),
        PipelineProvider::Drone => drone::create_environment(),
        PipelineProvider::GithubActions => github::create_environment(),
        PipelineProvider::Gitlab => gitlab::create_environment(),
        PipelineProvider::GoogleCloudBuild => google_cloud_build::create_environment(),
        PipelineProvider::Semaphore => semaphore::create_environment(),
        PipelineProvider::TravisCI => travisci::create_environment(),
        PipelineProvider::Unknown => {
            return None;
        }
    };

    Some(environment)
}

pub fn get_pipeline_output() -> PipelineOutput {
    match detect_pipeline_provider() {
        PipelineProvider::Buildkite => buildkite::BUILDKITE,
        PipelineProvider::GithubActions => github::GITHUB,
        _ => PipelineOutput::default(),
    }
}
