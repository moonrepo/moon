use crate::expander_context::ProjectExpanderContext;
use moon_common::color;
use moon_config::DependencyConfig;
use moon_project::Project;
use rustc_hash::FxHashMap;
use std::mem;
use tracing::{debug, instrument};

pub struct ProjectExpander<'graph> {
    context: ProjectExpanderContext<'graph>,
}

impl<'graph> ProjectExpander<'graph> {
    pub fn new(context: ProjectExpanderContext<'graph>) -> Self {
        Self { context }
    }

    #[instrument(name = "expand_project", skip_all)]
    pub fn expand(mut self, project: &Project) -> miette::Result<Project> {
        let mut project = project.to_owned();

        debug!(
            project_id = project.id.as_str(),
            "Expanding project {}",
            color::id(&project.id)
        );

        self.expand_deps(&mut project)?;

        Ok(project)
    }

    #[instrument(skip_all)]
    fn expand_deps(&mut self, project: &mut Project) -> miette::Result<()> {
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
}
