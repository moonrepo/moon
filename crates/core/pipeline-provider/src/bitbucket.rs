use crate::api::{opt_var, var, PipelineEnvironment, PipelineProvider};

pub fn create_environment() -> PipelineEnvironment {
    PipelineEnvironment {
        base_branch: opt_var("BITBUCKET_PR_DESTINATION_BRANCH"),
        branch: var("BITBUCKET_BRANCH"),
        id: var("BITBUCKET_PIPELINE_UUID"),
        provider: PipelineProvider::Bitbucket,
        request_id: opt_var("BITBUCKET_PR_ID"),
        request_url: None,
        revision: var("BITBUCKET_COMMIT"),
        url: None,
    }
}
