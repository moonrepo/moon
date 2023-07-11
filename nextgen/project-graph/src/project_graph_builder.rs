use moon_config::ToolchainConfig;
use moon_project_builder::ProjectBuilderContext;
use std::path::Path;

pub struct ProjectGraphBuilder<'app> {
    context: ProjectBuilderContext<'app>,
}

impl<'app> ProjectGraphBuilder<'app> {
    pub async fn load(&mut self, alias_or_id: &str) -> miette::Result<&Self> {
        // self.internal_load(alias_or_id)?;

        Ok(self)
    }

    pub async fn load_all(&mut self) -> miette::Result<&Self> {
        Ok(self)
    }
}
