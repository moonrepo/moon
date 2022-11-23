use crate::api::{opt_var, var, PipelineEnvironment, PipelineProvider};

pub fn create_environment() -> PipelineEnvironment {
    let trigger = opt_var("CODEBUILD_WEBHOOK_TRIGGER");

    PipelineEnvironment {
        base_branch: opt_var("CODEBUILD_WEBHOOK_BASE_REF"),
        branch: opt_var("CODEBUILD_WEBHOOK_HEAD_REF")
            .or_else(|| match &trigger {
                Some(value) => value
                    .strip_prefix("branch/")
                    .map(|branch| branch.to_owned()),
                None => None,
            })
            .unwrap_or_default(),
        id: var("CODEBUILD_BUILD_ID"),
        provider: PipelineProvider::AwsCodebuild,
        request_id: match &trigger {
            Some(value) => value.strip_prefix("pr/").map(|pr| pr.to_owned()),
            None => None,
        },
        request_url: None,
        revision: var("CODEBUILD_RESOLVED_SOURCE_VERSION"),
        url: opt_var("CODEBUILD_PUBLIC_BUILD_URL"),
    }
}
