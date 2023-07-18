use crate::token_expander::TokenExpander;
use moon_config::InputPath;
use moon_project::Project;
use moon_task::Task;
use std::path::Path;

pub struct TasksExpander<'proj> {
    pub project: &'proj mut Project,
    pub workspace_root: &'proj Path,
}

impl<'proj> TasksExpander<'proj> {
    pub fn expand(&mut self) {}

    pub fn expand_command(&mut self, task: &mut Task) -> miette::Result<()> {
        task.command =
            TokenExpander::for_command(self.project, task, self.workspace_root).expand_command()?;

        // TODO env var expansion

        Ok(())
    }

    pub fn expand_args(&mut self, task: &mut Task) -> miette::Result<()> {
        Ok(())
    }

    pub fn expand_deps(&mut self, task: &mut Task) -> miette::Result<()> {
        Ok(())
    }

    pub fn expand_env(&mut self, task: &mut Task) -> miette::Result<()> {
        Ok(())
    }

    pub fn expand_inputs(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.inputs.is_empty() {
            return Ok(());
        }

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
