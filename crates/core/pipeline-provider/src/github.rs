use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: handle_falsy_value(env::var("GITHUB_BASE_REF")),
        branch: env::var("GITHUB_HEAD_REF").unwrap_or_default(),
        id: env::var("GITHUB_RUN_ID").unwrap_or_default(),
        name: PipelineProvider::GithubActions,
        request_id: None,
        request_url: None,
        revision: env::var("GITHUB_SHA").unwrap_or_default(),
        url: None,
    }
}
