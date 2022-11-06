use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    let base_branch;
    let branch;

    if let Ok(pr_branch) = env::var("APPVEYOR_PULL_REQUEST_HEAD_REPO_BRANCH") {
        base_branch = handle_falsy_value(env::var("APPVEYOR_REPO_BRANCH"));
        branch = pr_branch;
    } else {
        base_branch = None;
        branch = env::var("APPVEYOR_REPO_BRANCH").unwrap_or_default();
    }

    PipelineEnvironment {
        base_branch,
        branch,
        id: env::var("APPVEYOR_BUILD_ID").unwrap_or_default(),
        name: PipelineProvider::AppVeyor,
        request_id: handle_falsy_value(env::var("APPVEYOR_PULL_REQUEST_NUMBER")),
        request_url: None,
        revision: env::var("APPVEYOR_PULL_REQUEST_HEAD_COMMIT").unwrap_or_default(),
        url: None,
    }
}
