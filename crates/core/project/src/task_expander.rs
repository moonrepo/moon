use crate::project::Project;
use moon_task::{ResolverData, Task, TaskError};

pub struct TaskExpander<'data> {
    data: &'data ResolverData,
}

impl<'data> TaskExpander<'data> {
    pub fn new(data: &'data ResolverData) -> Self {
        TaskExpander { data }
    }

    /// Expand environment variables by loading a `.env` file if configured.
    pub fn expand_env(&mut self, task: &mut Task) -> Result<(), TaskError> {
        if let Some(env_file) = &task.options.env_file {
            let env_path = self.data.project_root.join(env_file);
            let error_handler =
                |e: dotenvy::Error| TaskError::InvalidEnvFile(env_path.clone(), e.to_string());

            // The `.env` file may not have been committed, so avoid crashing in CI
            if is_ci() && !env_path.exists() {
                debug!(
                    target: task.get_log_target(),
                    "The `envFile` option is enabled but no `.env` file exists in CI, skipping as this may be intentional",
                );

                return Ok(());
            }

            for entry in dotenvy::from_path_iter(&env_path).map_err(error_handler)? {
                let (key, value) = entry.map_err(error_handler)?;

                // Vars defined in `env` take precedence over those in the env file
                self.env.entry(key).or_insert(value);
            }
        }

        Ok(())
    }
}
