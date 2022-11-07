use crate::api::{opt_var, var, PipelineEnvironment, PipelineProvider};

pub fn create_environment() -> PipelineEnvironment {
    let base_branch;
    let branch;

    if let Some(pr_branch) = opt_var("SEMAPHORE_GIT_PR_BRANCH") {
        base_branch = opt_var("SEMAPHORE_GIT_BRANCH");
        branch = pr_branch;
    } else {
        base_branch = None;
        branch = var("SEMAPHORE_GIT_BRANCH");
    }

    PipelineEnvironment {
        base_branch,
        branch,
        id: var("SEMAPHORE_WORKFLOW_ID"),
        provider: PipelineProvider::Semaphore,
        request_id: opt_var("SEMAPHORE_GIT_PR_NUMBER"),
        request_url: None,
        revision: opt_var("SEMAPHORE_GIT_PR_SHA")
            .or_else(|| opt_var("SEMAPHORE_GIT_SHA"))
            .unwrap_or_default(),
        url: None,
    }
}
