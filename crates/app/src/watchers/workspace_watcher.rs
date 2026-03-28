use std::sync::Arc;

use crate::session::MoonSession;
use async_trait::async_trait;
use moon_config::ConfigFinder;
use moon_file_watcher::*;
use proto_core::ProtoEnvironment;
use regex::Regex;
use tokio::sync::Mutex;

pub struct WorkspaceWatcher {
    graph_mutex: Arc<Mutex<()>>,
    project_config_regex: Regex,
    tasks_config_regex: Regex,
    workspace_config_regex: Regex,
}

impl Default for WorkspaceWatcher {
    fn default() -> Self {
        let exts_group = format!("({})", ConfigFinder::default().extensions.join("|"));

        Self {
            graph_mutex: Arc::new(Mutex::new(())),
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
        &self,
        session: &mut MoonSession,
        event: &FileEvent,
    ) -> miette::Result<()> {
        println!("WorkspaceWatcher: File event: {:?}", event);

        // Handle `.prototools` changes
        if event.path.as_str() == ".prototools" {
            self.reset_proto(session)?;

            return Ok(());
        }

        // Handle `.moon/*.config` changes
        if let Some(caps) = self.workspace_config_regex.captures(event.path.as_str()) {
            match caps.name("name").map(|cap| cap.as_str()) {
                Some("extensions") => self.reset_extensions(session)?,
                Some("toolchains") => self.reset_toolchains(session)?,
                Some("workspace") => self.reset_workspace(session)?,
                _ => {}
            };

            return Ok(());
        }

        // Handle `.moon/tasks/**/*.config` changes
        if self.tasks_config_regex.is_match(event.path.as_str()) {
            self.reset_tasks(session)?;

            return Ok(());
        }

        // Handle `moon.config` changes
        if self.project_config_regex.is_match(event.path.as_str()) {
            self.reset_projects(session)?;

            return Ok(());
        }

        Ok(())
    }
}

impl WorkspaceWatcher {
    fn reset_proto(&self, session: &mut MoonSession) -> miette::Result<()> {
        let mut env = ProtoEnvironment::new()?;
        env.working_dir = session.working_dir.clone();

        session.set_proto_env(env);

        Ok(())
    }

    fn reset_extensions(&self, session: &mut MoonSession) -> miette::Result<()> {
        session.set_extensions_config(
            session
                .config_loader
                .load_extensions_config(&session.workspace_root)?,
        );

        Ok(())
    }

    fn reset_projects(&self, session: &mut MoonSession) -> miette::Result<()> {
        session.reset_components();
        session.regenerate_graphs(Arc::clone(&self.graph_mutex));

        Ok(())
    }

    fn reset_tasks(&self, session: &mut MoonSession) -> miette::Result<()> {
        session.set_tasks_config(
            session
                .config_loader
                .load_tasks_manager(&session.workspace_root)?,
        );

        Ok(())
    }

    fn reset_toolchains(&self, session: &mut MoonSession) -> miette::Result<()> {
        session.set_toolchains_config(
            session.config_loader.load_toolchains_config(
                &session.workspace_root,
                session.proto_env.load_config()?,
            )?,
        );

        Ok(())
    }

    fn reset_workspace(&self, session: &mut MoonSession) -> miette::Result<()> {
        session.set_workspace_config(
            session
                .config_loader
                .load_workspace_config(&session.workspace_root)?,
        );
        session.regenerate_graphs(Arc::clone(&self.graph_mutex));

        Ok(())
    }
}
