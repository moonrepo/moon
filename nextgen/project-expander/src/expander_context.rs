use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::patterns;
use moon_project::Project;
use moon_task::Target;
use rustc_hash::FxHashMap;
use std::env;
use std::path::Path;
use tracing::warn;

/// Boundaries between projects to validate.
#[derive(Default)]
pub struct ExpansionBoundaries {
    pub output_files: FxHashMap<WorkspaceRelativePathBuf, Target>,
    pub output_globs: FxHashMap<WorkspaceRelativePathBuf, Target>,
}

pub struct ExpanderContext<'graph, 'query> {
    /// Mapping of aliases to their project IDs.
    pub aliases: FxHashMap<&'graph str, &'graph Id>,

    /// Whether to check project boundaries.
    pub check_boundaries: bool,

    /// The base unexpanded project.
    pub project: &'graph Project,

    /// Function to query for projects.
    pub query: Box<dyn Fn(String) -> miette::Result<Vec<&'query Project>> + 'graph>,

    /// Workspace root, of course.
    pub workspace_root: &'graph Path,
}

pub fn substitute_env_var(value: &str, env_map: &FxHashMap<String, String>) -> String {
    patterns::ENV_VAR_SUBSTITUTE.replace_all(
        value,
        |caps: &patterns::Captures| {
            // First with wrapping {}, then without
            let name = caps.get(1).or_else(|| caps.get(2)).unwrap().as_str();

            match env_map.get(name).map(|v| v.to_owned()).or_else(|| env::var(name).ok()) {
                Some(var) => var,
                None => {
                     warn!(
                        "Task value `{}` contains the environment variable ${}, but this variable is not set. Not substituting and keeping as-is.",
                        value,
                        name
                    );

                    caps.get(0).unwrap().as_str().to_owned()
                }
            }
        })
    .to_string()
}
