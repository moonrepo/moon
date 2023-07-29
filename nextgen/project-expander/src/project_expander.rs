use crate::expander_context::ExpanderContext;
use crate::tasks_expander::TasksExpander;
use moon_config::DependencyConfig;
use moon_project::Project;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::mem;

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

        self.expand_deps(&mut project)?;
        self.expand_tasks(&mut project)?;

        Ok(project)
    }

    pub fn expand_deps(&mut self, project: &mut Project) -> miette::Result<()> {
        let mut depends_on = FxHashMap::default();

        for (dep_id, dep_config) in mem::take(&mut project.dependencies) {
            let new_dep_id = self
                .context
                .aliases
                .get(dep_id.as_str())
                .map(|id| (*id).to_owned())
                .unwrap_or(dep_id);

            depends_on.insert(
                new_dep_id.clone(),
                DependencyConfig {
                    id: new_dep_id,
                    ..dep_config
                },
            );
        }

        project.dependencies = depends_on;

        Ok(())
    }

    pub fn expand_tasks(&mut self, project: &mut Project) -> miette::Result<()> {
        let mut tasks = BTreeMap::new();
        let mut expander = TasksExpander::new(&self.context);

        for (task_id, mut task) in mem::take(&mut project.tasks) {
            // Resolve in this order!
            expander.expand_env(&mut task)?;
            expander.expand_deps(&mut task)?;
            expander.expand_inputs(&mut task)?;
            expander.expand_outputs(&mut task)?;
            expander.expand_args(&mut task)?;
            expander.expand_command(&mut task)?;

            tasks.insert(task_id, task);
        }

        project.tasks = tasks;

        Ok(())
    }
}
