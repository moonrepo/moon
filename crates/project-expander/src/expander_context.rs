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

    /// Check `runInCI` relationships are legitimate.
    pub check_ci_relationships: bool,

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
            let Some(name) = caps.name("name1")
                .or_else(|| caps.name("name2"))
                .map(|cap| cap.as_str())
                else {
                return String::new();
            };

            let flag = caps.name("flag1").or_else(|| caps.name("flag2")).map(|cap| cap.as_str());

            // If the variable is referencing itself, don't pull
            // from the local map, and instead only pull from the
            // system environment. Otherwise we hit recursion!
            let get_replacement_value = || {
                if !base_name.is_empty() && base_name == name {
                    env::var(name).ok()
                } else {
                    env_map.get(name).cloned().or_else(|| env::var(name).ok())
                }
            };

            match flag {
                // Don't substitute
                Some("!") => {
                    format!("${name}")
                },
                // Substitute with empty string when missing
                Some("?") =>{
                    debug!(
                        "Task value `{}` contains the environment variable ${}, but this variable is not set. Replacing with an empty value.",
                        value,
                        name
                    );

                    get_replacement_value().unwrap_or_default()
                },
                // Substitute with self when missing
                _ => {
                    debug!(
                        "Task value `{}` contains the environment variable ${}, but this variable is not set. Not substituting and keeping as-is. Append with ? or ! to change outcome.",
                        value,
                        name
                    );

                    get_replacement_value()
                        .unwrap_or_else(|| caps.get(0).unwrap().as_str().to_owned())
                }
            }
        })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_flags_when_missing() {
        let envs = FxHashMap::default();

        assert_eq!(substitute_env_var("", "$KEY", &envs), "$KEY");
        assert_eq!(substitute_env_var("", "${KEY}", &envs), "${KEY}");

        assert_eq!(substitute_env_var("", "$KEY!", &envs), "$KEY");
        assert_eq!(substitute_env_var("", "${KEY!}", &envs), "$KEY");

        assert_eq!(substitute_env_var("", "$KEY?", &envs), "");
        assert_eq!(substitute_env_var("", "${KEY?}", &envs), "");
    }

    #[test]
    fn handles_flags_when_not_missing() {
        let mut envs = FxHashMap::default();
        envs.insert("KEY".to_owned(), "value".to_owned());

        assert_eq!(substitute_env_var("", "$KEY", &envs), "value");
        assert_eq!(substitute_env_var("", "${KEY}", &envs), "value");

        assert_eq!(substitute_env_var("", "$KEY!", &envs), "$KEY");
        assert_eq!(substitute_env_var("", "${KEY!}", &envs), "$KEY");

        assert_eq!(substitute_env_var("", "$KEY?", &envs), "value");
        assert_eq!(substitute_env_var("", "${KEY?}", &envs), "value");
    }
}
