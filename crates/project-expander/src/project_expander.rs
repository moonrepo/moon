use crate::expander_context::ExpanderContext;
use crate::tasks_expander::TasksExpander;
use moon_common::color;
use moon_config::DependencyConfig;
use moon_project::Project;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::mem;
use tracing::debug;

pub struct ProjectExpander<'graph, 'query> {
    context: ExpanderContext<'graph, 'query>,
}

impl<'graph, 'query> ProjectExpander<'graph, 'query> {
    pub fn new(context: ExpanderContext<'graph, 'query>) -> Self {
        Self { context }
    }

    pub fn expand(&mut self) -> miette::Result<Project> {
        // Clone before expanding!
        let mut project = self.context.project.to_owned();

        debug!(
            id = project.id.as_str(),
            "Expanding project {}",
            color::id(&project.id)
        );

        self.expand_deps(&mut project)?;
        self.expand_tasks(&mut project)?;

        Ok(project)
    }

    pub fn expand_deps(&mut self, project: &mut Project) -> miette::Result<()> {
        let mut depends_on = FxHashMap::default();

        for dep_config in mem::take(&mut project.dependencies) {
            let new_dep_id = self
                .context
                .aliases
                .get(dep_config.id.as_str())
                .map(|id| (*id).to_owned())
                .unwrap_or(dep_config.id);

            // Use a map so that aliases and IDs get flattened
            depends_on.insert(
                new_dep_id.clone(),
                DependencyConfig {
                    id: new_dep_id,
                    ..dep_config
                },
            );
        }

        project.dependencies = depends_on.into_values().collect();

        Ok(())
    }

    pub fn expand_tasks(&mut self, project: &mut Project) -> miette::Result<()> {
        let mut tasks = BTreeMap::new();
        let mut expander = TasksExpander::new(&self.context);

        for (task_id, mut task) in mem::take(&mut project.tasks) {
            debug!(
                target = task.target.as_str(),
                "Expanding task {}",
                color::label(&task.target)
            );

            // Resolve in this order!
            expander.expand_env(&mut task)?;
            expander.expand_deps(&mut task)?;
            expander.expand_inputs(&mut task)?;
            expander.expand_outputs(&mut task)?;
            expander.expand_args(&mut task)?;
            expander.expand_command(&mut task)?;

            task.metadata.expanded = true;

            tasks.insert(task_id, task);
        }

        project.tasks = tasks;

        Ok(())
    }
}
