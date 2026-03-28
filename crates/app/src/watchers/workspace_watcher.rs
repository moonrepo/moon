use crate::session::MoonSession;
use async_trait::async_trait;
use moon_common::path::WorkspaceRelativePath;
use moon_config::{ConfigFinder, WorkspaceProjects};
use moon_file_watcher::*;
use moon_workspace::{STATE_GRAPH_FILE_NAME, STATE_PROJECTS_FILE_NAME};
use proto_core::ProtoEnvironment;
use regex::Regex;
use starbase_utils::fs;
use starbase_utils::glob::GlobSet;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::trace;

pub struct WorkspaceWatcher {
    graph_handle: Option<JoinHandle<()>>,

    project_config_regex: Regex,
    tasks_config_regex: Regex,
    workspace_config_regex: Regex,
}

impl Default for WorkspaceWatcher {
    fn default() -> Self {
        let exts_group = format!("({})", ConfigFinder::default().extensions.join("|"));

        Self {
            graph_handle: None,
            project_config_regex: Regex::new(&format!(r"(^|/)moon\.{exts_group}$")).unwrap(),
            tasks_config_regex: Regex::new(&format!(r"^(\.moon|\.config/moon)/.*\.{exts_group}$"))
                .unwrap(),
            workspace_config_regex: Regex::new(&format!(
                r"^(\.moon|\.config/moon)/(?<name>\w+)\.{exts_group}$"
            ))
            .unwrap(),
        }
    }
}

#[async_trait]
impl FileWatcher<MoonSession> for WorkspaceWatcher {
    async fn on_file_event(
        &mut self,
        session: &mut MoonSession,
        event: &FileEvent,
    ) -> miette::Result<()> {
        if !event.is_mutated() {
            return Ok(());
        }

        // Handle root `.prototools` changes
        if event.path.as_str() == ".prototools" {
            self.reset_proto(session)?;

            return Ok(());
        }

        // Handle `.moon/*.config` changes
        if let Some(caps) = self.workspace_config_regex.captures(event.path.as_str()) {
            match caps.name("name").map(|cap| cap.as_str()) {
                Some("extensions") => self.reset_extensions(session)?,
                Some("toolchains") => self.reset_toolchains(session)?,
                Some("workspace") => self.reset_workspace(session).await?,
                _ => {}
            };

            return Ok(());
        }

        // Handle `.moon/tasks/**/*.config` changes
        if self.tasks_config_regex.is_match(event.path.as_str()) {
            self.reset_tasks(session).await?;

            return Ok(());
        }

        // Handle `moon.config` changes
        if self.project_config_regex.is_match(event.path.as_str()) {
            self.reset_projects(session).await?;

            return Ok(());
        }

        // Handle the creation/removal of project directories
        if event.is_mutated_directory() && self.is_a_project_root(session, &event.path)? {
            self.reset_projects(session).await?;

            return Ok(());
        }

        Ok(())
    }
}

impl WorkspaceWatcher {
    fn is_a_project_root(
        &self,
        session: &MoonSession,
        path: &WorkspaceRelativePath,
    ) -> miette::Result<bool> {
        let (sources, globs): (Vec<_>, Vec<_>) = match &session.workspace_config.projects {
            WorkspaceProjects::Sources(sources) => (sources.values().collect(), Vec::new()),
            WorkspaceProjects::Globs(globs) => (Vec::new(), globs.iter().collect()),
            WorkspaceProjects::Both(inner) => (
                inner.sources.values().collect(),
                inner.globs.iter().collect(),
            ),
        };

        for source in sources {
            if path == source {
                return Ok(true);
            }
        }

        if !globs.is_empty() {
            return Ok(GlobSet::new(globs)?.matches(path.as_str()));
        }

        Ok(false)
    }

    async fn rebuild_graphs(&mut self, session: &mut MoonSession) -> miette::Result<()> {
        session.reset_components();

        // Abort any existing graph building
        if let Some(handle) = self.graph_handle.take() {
            handle.abort();
        }

        // Ensure the cache/state files are cleared before rebuilding
        let cache_engine = session.get_cache_engine()?;

        fs::remove_file(cache_engine.state.resolve_path(STATE_GRAPH_FILE_NAME))?;
        fs::remove_file(cache_engine.state.resolve_path(STATE_PROJECTS_FILE_NAME))?;

        // Rebuild the graphs in a background thread
        self.graph_handle = Some(session.rebuild_graphs());

        Ok(())
    }

    fn reset_proto(&self, session: &mut MoonSession) -> miette::Result<()> {
        trace!("Updating proto environment");

        let mut env = ProtoEnvironment::new()?;
        env.working_dir = session.working_dir.clone();

        session.proto_env = Arc::new(env);
        session.reset_components();

        Ok(())
    }

    fn reset_extensions(&self, session: &mut MoonSession) -> miette::Result<()> {
        trace!("Updating extensions config");

        let extensions_config = session
            .config_loader
            .load_extensions_config(&session.workspace_root)?;
        let invalidate = session
            .extensions_config
            .should_invalidate(&extensions_config);

        session.extensions_config = Arc::new(extensions_config);

        // Invalidate the extensions registry if the extensions config changed
        if invalidate {
            session.reset_components();
            session.download_extensions();
        }

        Ok(())
    }

    async fn reset_projects(&mut self, session: &mut MoonSession) -> miette::Result<()> {
        // Always invalidate the workspace graph if a project config changes
        self.rebuild_graphs(session).await?;

        Ok(())
    }

    async fn reset_tasks(&mut self, session: &mut MoonSession) -> miette::Result<()> {
        trace!("Updating inherited tasks config");

        let tasks_config = session
            .config_loader
            .load_tasks_manager(&session.workspace_root)?;
        let invalidate = session.tasks_config.should_invalidate(&tasks_config);

        session.tasks_config = Arc::new(tasks_config);

        // Invalidate the workspace graphs if the tasks config changed,
        // so that task inheritance is properly reflected
        if invalidate {
            self.rebuild_graphs(session).await?;
        }

        Ok(())
    }

    fn reset_toolchains(&self, session: &mut MoonSession) -> miette::Result<()> {
        trace!("Updating toolchains config");

        let toolchains_config = session
            .config_loader
            .load_toolchains_config(&session.workspace_root, session.proto_env.load_config()?)?;
        let invalidate = session
            .toolchains_config
            .should_invalidate(&toolchains_config);

        session.toolchains_config = Arc::new(toolchains_config);

        // Invalidate the toolchain registry if the toolchains config changed
        if invalidate {
            session.reset_components();
            session.download_toolchains();
        }

        Ok(())
    }

    async fn reset_workspace(&mut self, session: &mut MoonSession) -> miette::Result<()> {
        trace!("Updating workspace config");

        let workspace_config = session
            .config_loader
            .load_workspace_config(&session.workspace_root)?;
        let mut rebuild = false;

        // Invalidate the VCS adapter if the VCS config changed
        if session
            .workspace_config
            .vcs
            .should_invalidate(&workspace_config.vcs)
        {
            session.reset_vcs();
        }

        // Invalidate the workspace graphs if the project configs changed
        if workspace_config.projects != session.workspace_config.projects
            || workspace_config.default_project != session.workspace_config.default_project
        {
            session.reset_components();
            rebuild = true;
        }

        // If the daemon has been turned off, attempt to stop it via the client
        if !workspace_config.daemon
            && let Ok(Some(mut client)) = session.connect_to_daemon().await
        {
            let _ = client.stop().await;
        }

        session.workspace_config = Arc::new(workspace_config);

        // Must run after the new config has been set!
        if rebuild {
            self.rebuild_graphs(session).await?;
        }

        Ok(())
    }
}
