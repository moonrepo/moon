use moon_common::Id;
use moon_config::patterns;
use moon_project::Project;
use rustc_hash::FxHashMap;
use std::env;
use std::path::Path;
use tracing::debug;

pub struct ExpanderContext<'graph, 'query> {
    /// Mapping of aliases to their project IDs.
    pub aliases: FxHashMap<&'graph str, &'graph Id>,

    /// The base unexpanded project.
    pub project: &'graph Project,

    /// Function to query for projects.
    pub query: Box<dyn Fn(String) -> miette::Result<Vec<&'query Project>> + 'graph>,

    /// Workspace root, of course.
    pub workspace_root: &'graph Path,
}

pub fn substitute_env_vars(mut env: FxHashMap<String, String>) -> FxHashMap<String, String> {
    let cloned_env = env.clone();

    for (key, value) in env.iter_mut() {
        *value = substitute_env_var(key, value, &cloned_env);
    }

    env
}

pub fn substitute_env_var(
    base_name: &str,
    value: &str,
    env_map: &FxHashMap<String, String>,
) -> String {
    if !value.contains('$') {
        return value.to_owned();
    }

    patterns::ENV_VAR_SUBSTITUTE.replace_all(
        value,
        |caps: &patterns::Captures| {
            // First with wrapping {} then without: ${FOO} -> $FOO
            let name = caps.get(1).or_else(|| caps.get(2)).unwrap().as_str();

            // If the variable is referencing itself, don't pull
            // from the local map, and instead only pull from the
            // system environment. Otherwise we hit recursion!
            let maybe_var = if !base_name.is_empty() && base_name == name {
                env::var(name).ok()
            } else {
                env_map.get(name).cloned().or_else(|| env::var(name).ok())
            };

            match maybe_var {
                Some(var) => var,
                None => {
                    debug!(
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
