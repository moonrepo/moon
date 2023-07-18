use crate::tasks_expander_error::TasksExpanderError;
use crate::token_expander::TokenExpander;
use moon_common::color;
use moon_config::{patterns, InputPath};
use moon_project::Project;
use moon_task::Task;
use rustc_hash::FxHashMap;
use std::env;
use std::path::Path;
use tracing::{trace, warn};

fn substitute_env_var(value: &str, strict: bool) -> String {
    let pattern = if strict {
        &patterns::ENV_VAR_SUBSTITUTE
    } else {
        &patterns::ENV_VAR
    };

    pattern.replace_all(
        value,
        |caps: &patterns::Captures| {
            let name = caps.get(1).unwrap().as_str();
            let value = env::var(name).unwrap_or_default();

            if value.is_empty() {
                warn!(
                    "Task value `{}` contains the environment variable ${}, but this variable is either not set or is empty. Substituting with an empty string.",
                    value,
                    name
                );
            }

            value
        })
    .to_string()
}

pub struct TasksExpander<'proj> {
    pub project: &'proj mut Project,
    pub workspace_root: &'proj Path,
}

impl<'proj> TasksExpander<'proj> {
    pub fn expand(&mut self) {}

    pub fn expand_command(&mut self, task: &mut Task) -> miette::Result<()> {
        // Token variables
        let command =
            TokenExpander::for_command(self.project, task, self.workspace_root).expand_command()?;

        // Environment variables
        task.command = substitute_env_var(&command, false);

        Ok(())
    }

    pub fn expand_args(&mut self, task: &mut Task) -> miette::Result<()> {
        Ok(())
    }

    pub fn expand_deps(&mut self, task: &mut Task) -> miette::Result<()> {
        Ok(())
    }

    pub fn expand_env(&mut self, task: &mut Task) -> miette::Result<()> {
        // Substitute environment variables
        task.env.iter_mut().for_each(|(_, value)| {
            *value = substitute_env_var(value, true);
        });

        // Load variables from an .env file
        if let Some(env_file) = &task.options.env_file {
            let target = &task.target;
            let env_path = env_file
                .to_workspace_relative(self.project.source.as_str())
                .to_path(self.workspace_root);

            trace!(
                target = target.as_str(),
                env_file = ?env_path,
                "Loading env vars from dotenv",
            );

            // The `.env` file may not have been committed, so avoid crashing
            if env_path.exists() {
                let env_vars = dotenvy::from_path_iter(&env_path)
                    .map_err(|error| TasksExpanderError::InvalidEnvFile {
                        path: env_path.to_path_buf(),
                        error,
                    })?
                    .flatten()
                    .collect::<FxHashMap<_, _>>();

                // Don't override task-level variables
                for (key, val) in env_vars {
                    task.env.entry(key).or_insert(val);
                }
            } else {
                warn!(
                    target = target.as_str(),
                    env_file = ?env_path,
                    "The {} option is enabled but file doesn't exist, skipping as this may be intentional",
                    color::id("envFile"),
                );
            }
        }

        Ok(())
    }

    pub fn expand_inputs(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.inputs.is_empty() {
            return Ok(());
        }

        // Expand inputs to file system paths and environment variables
        for input in &task.inputs {
            if let InputPath::EnvVar(var) = input {
                task.input_vars.insert(var.to_owned());
            }
        }

        let (files, globs) =
            TokenExpander::for_inputs(self.project, task, self.workspace_root).expand_inputs()?;

        task.input_paths.extend(files);
        task.input_globs.extend(globs);

        Ok(())
    }

    pub fn expand_outputs(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.outputs.is_empty() {
            return Ok(());
        }

        // Expand outputs to file system paths
        let (files, globs) =
            TokenExpander::for_outputs(self.project, task, self.workspace_root).expand_outputs()?;

        // Outputs must *not* be considered an input,
        // so if there's an input that matches an output,
        // remove it! Is there a better way to do this?
        for file in files {
            if task.input_paths.contains(&file) {
                task.input_paths.remove(&file);
            }

            task.output_paths.insert(file);
        }

        for glob in globs {
            if task.input_globs.contains(&glob) {
                task.input_globs.remove(&glob);
            }

            task.output_globs.insert(glob);
        }

        Ok(())
    }
}
