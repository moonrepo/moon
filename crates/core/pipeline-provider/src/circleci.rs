use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: None,
        branch: env::var("CIRCLE_BRANCH").unwrap_or_default(),
        id: env::var("CIRCLE_WORKFLOW_ID").unwrap_or_default(),
        name: PipelineProvider::CircleCI,
        request_id: handle_falsy_value(env::var("CIRCLE_PR_NUMBER")),
        request_url: handle_falsy_value(env::var("CIRCLE_PULL_REQUEST")),
        revision: env::var("CIRCLE_SHA1").unwrap_or_default(),
        url: env::var("CIRCLE_BUILD_URL").unwrap_or_default(),
    }
}
