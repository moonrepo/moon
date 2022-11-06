use crate::api::{handle_falsy_value, PipelineEnvironment, PipelineProvider};
use std::env;

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: handle_falsy_value(env::var("DRONE_TARGET_BRANCH")),
        branch: env::var("DRONE_SOURCE_BRANCH")
            .or_else(|| env::var("DRONE_BRANCH"))
            .unwrap_or_default(),
        id: env::var("DRONE_BUILD_NUMBER").unwrap_or_default(),
        name: PipelineProvider::Drone,
        request_id: handle_falsy_value(env::var("DRONE_PULL_REQUEST")),
        request_url: None,
        revision: env::var("DRONE_COMMIT").unwrap_or_default(),
        url: handle_falsy_value(env::var("DRONE_BUILD_LINK")),
    }
}
