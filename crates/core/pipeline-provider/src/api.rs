use std::env::VarError;

#[derive(Default)]
pub enum PipelineProvider {
    AppVeyor,
    Bitbucket,
    Buildkite,
    CircleCI,
    GithubActions,
    TravisCI,
    #[default]
    Unknown,
}

#[derive(Default)]
pub struct PipelineEnvironment {
    /// Base branch of the pull/merge request.
    pub base_branch: Option<String>,

    /// Branch that triggered the pipeline.
    pub branch: String,

    /// Unique ID of the current build/run.
    pub id: String,

    /// Name of the provider.
    pub name: PipelineProvider,

    /// ID of an associated pull/merge request.
    pub request_id: Option<String>,

    /// Link to the pull/merge request.
    pub request_url: Option<String>,

    /// Revision of the triggered pipeline.
    pub revision: String,

    /// Link to the pipeline.
    pub url: Option<String>,
}

pub fn handle_falsy_value(result: Result<String, VarError>) -> Option<String> {
    match result {
        Ok(var) => {
            if var == "false" || var == "" {
                None
            } else {
                Some(var)
            }
        }
        Err(_) => None,
    }
}
