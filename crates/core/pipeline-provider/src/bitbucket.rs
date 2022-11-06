use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: handle_falsy_value(env::var("BITBUCKET_PR_DESTINATION_BRANCH")),
        branch: env::var("BITBUCKET_BRANCH").unwrap_or_default(),
        id: env::var("BITBUCKET_PIPELINE_UUID").unwrap_or_default(),
        name: PipelineProvider::Bitbucket,
        request_id: handle_falsy_value(env::var("BITBUCKET_PR_ID")),
        request_url: None,
        revision: env::var("BITBUCKET_COMMIT").unwrap_or_default(),
        url: None,
    }
}
