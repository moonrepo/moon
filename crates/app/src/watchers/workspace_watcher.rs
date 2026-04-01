use crate::session::MoonSession;
use async_trait::async_trait;
use moon_common::path::WorkspaceRelativePath;
use moon_config::WorkspaceProjects;
use moon_daemon::AtomicDaemonState;
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
    session: MoonSession,

    project_config_regex: Regex,
    tasks_config_regex: Regex,
    workspace_config_regex: Regex,
}

impl WorkspaceWatcher {
    pub fn new(session: MoonSession) -> Self {
        let exts_group = format!("({})", session.config_loader.extensions.join("|"));

        Self {
            graph_handle: None,
            session,
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
impl FileWatcher<AtomicDaemonState> for WorkspaceWatcher {
    async fn on_file_event(
        &mut self,
        state: AtomicDaemonState,
        event: &FileEvent,
    ) -> miette::Result<()> {
        if !event.is_mutated() {
            return Ok(());
        }

        // Handle root `.prototools` changes
        if event.path.as_str() == ".prototools" {
            self.reset_proto(&state).await?;

            return Ok(());
        }

        // Handle `.moon/*.config` changes
        if let Some(caps) = self.workspace_config_regex.captures(event.path.as_str()) {
            match caps.name("name").map(|cap| cap.as_str()) {
                Some("extensions") => self.reset_extensions(&state).await?,
                Some("toolchains") => self.reset_toolchains(&state).await?,
                Some("workspace") => self.reset_workspace(&state).await?,
                _ => {}
            };

            return Ok(());
        }

        // Handle `.moon/tasks/**/*.config` changes
        if self.tasks_config_regex.is_match(event.path.as_str()) {
            self.reset_tasks(&state).await?;

            return Ok(());
        }

        // Handle `moon.config` changes
        if self.project_config_regex.is_match(event.path.as_str()) {
            self.reset_projects(&state).await?;

            return Ok(());
        }

        // Handle the creation/removal of project directories
        if event.is_mutated_directory() && self.is_a_project_root(&event.path)? {
            self.reset_projects(&state).await?;

            return Ok(());
        }

        Ok(())
    }
}

impl WorkspaceWatcher {
    fn is_a_project_root(&self, path: &WorkspaceRelativePath) -> miette::Result<bool> {
        let (sources, globs): (Vec<_>, Vec<_>) = match &self.session.workspace_config.projects {
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

    async fn rebuild_graphs(&mut self, state: &AtomicDaemonState) -> miette::Result<()> {
        // Abort any existing graph building
        if let Some(handle) = self.graph_handle.take() {
            handle.abort();
        }

        // Ensure the cache/state files are cleared before rebuilding
        let cache_engine = self.session.get_cache_engine()?;

        fs::remove_file(cache_engine.state.resolve_path(STATE_GRAPH_FILE_NAME))?;
        fs::remove_file(cache_engine.state.resolve_path(STATE_PROJECTS_FILE_NAME))?;

        // Rebuild the graphs in a background thread
        self.graph_handle = Some(self.session.rebuild_graphs(Arc::clone(state)));

        Ok(())
    }

    async fn reset_proto(&mut self, state: &AtomicDaemonState) -> miette::Result<()> {
        trace!("Updating proto environment");

        let mut env = ProtoEnvironment::new()?;
        env.working_dir = self.session.working_dir.clone();

        self.session.proto_env = Arc::new(env);
        self.session.reset_components();

        state.write().await.app_context = self.session.get_app_context().await?;

        Ok(())
    }

    async fn reset_extensions(&mut self, state: &AtomicDaemonState) -> miette::Result<()> {
        trace!("Updating extensions config");

        let extensions_config = self
            .session
            .config_loader
            .load_extensions_config(&self.session.workspace_root)?;
        let invalidate = self
            .session
            .extensions_config
            .should_invalidate(&extensions_config);

        self.session.extensions_config = Arc::new(extensions_config);

        // Invalidate the extensions registry if the extensions config changed
        if invalidate {
            self.session.reset_components();
            self.session.download_extensions();
        }

        state.write().await.app_context = self.session.get_app_context().await?;

        Ok(())
    }

    async fn reset_projects(&mut self, state: &AtomicDaemonState) -> miette::Result<()> {
        // Always invalidate the workspace graph if a project config changes
        self.session.reset_components();
        self.rebuild_graphs(state).await?;

        Ok(())
    }

    async fn reset_tasks(&mut self, state: &AtomicDaemonState) -> miette::Result<()> {
        trace!("Updating inherited tasks config");

        let tasks_config = self
            .session
            .config_loader
            .load_tasks_manager(&self.session.workspace_root)?;
        let invalidate = self.session.tasks_config.should_invalidate(&tasks_config);

        self.session.tasks_config = Arc::new(tasks_config);

        // Invalidate the workspace graphs if the tasks config changed,
        // so that task inheritance is properly reflected
        if invalidate {
            self.session.reset_components();
            self.rebuild_graphs(state).await?;
        }

        state.write().await.app_context = self.session.get_app_context().await?;

        Ok(())
    }

    async fn reset_toolchains(&mut self, state: &AtomicDaemonState) -> miette::Result<()> {
        trace!("Updating toolchains config");

        let toolchains_config = self.session.config_loader.load_toolchains_config(
            &self.session.workspace_root,
            self.session.proto_env.load_config()?,
        )?;
        let invalidate = self
            .session
            .toolchains_config
            .should_invalidate(&toolchains_config);

        self.session.toolchains_config = Arc::new(toolchains_config);

        // Invalidate the toolchain registry if the toolchains config changed
        if invalidate {
            self.session.reset_components();
            self.session.download_toolchains();
        }

        state.write().await.app_context = self.session.get_app_context().await?;

        Ok(())
    }

    async fn reset_workspace(&mut self, state: &AtomicDaemonState) -> miette::Result<()> {
        trace!("Updating workspace config");

        let workspace_config = self
            .session
            .config_loader
            .load_workspace_config(&self.session.workspace_root)?;
        let mut rebuild = false;

        // Invalidate the VCS adapter if the VCS config changed
        if self
            .session
            .workspace_config
            .vcs
            .should_invalidate(&workspace_config.vcs)
        {
            self.session.reset_vcs();
        }

        // Invalidate the workspace graphs if the project configs changed
        if workspace_config.projects != self.session.workspace_config.projects
            || workspace_config.default_project != self.session.workspace_config.default_project
        {
            self.session.reset_components();
            rebuild = true;
        }

        // If the daemon has been turned off, attempt to stop it via the client
        if !workspace_config.daemon
            && let Ok(Some(mut client)) = self.session.connect_to_daemon().await
        {
            let _ = client.stop().await;
        }

        self.session.workspace_config = Arc::new(workspace_config);

        // Must run after the new config has been set!
        if rebuild {
            self.rebuild_graphs(state).await?;
        }

        state.write().await.app_context = self.session.get_app_context().await?;

        Ok(())
    }
}
