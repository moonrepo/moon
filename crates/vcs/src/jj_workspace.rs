use crate::jj::{Jujutsu, JjWorkspace};
use crate::process_cache::ProcessCache;
use crate::vcs::Vcs;
use miette::IntoDiagnostic;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, instrument};

/// Extensions for Jujutsu workspace management.
pub trait JujutsuWorkspaceExt {
    /// Create a new workspace.
    async fn create_workspace(&self, name: &str, path: &Path) -> miette::Result<()>;

    /// List all workspaces in the repository.
    async fn list_workspaces(&self) -> miette::Result<Vec<JjWorkspace>>;

    /// Switch to a different workspace.
    async fn switch_workspace(&self, name: &str) -> miette::Result<()>;

    /// Update a stale workspace.
    async fn update_stale_workspace(&self, name: &str) -> miette::Result<()>;

    /// Remove a workspace from tracking.
    async fn forget_workspace(&self, name: &str) -> miette::Result<()>;

    /// Get the root path of a specific workspace.
    async fn get_workspace_root(&self, name: &str) -> miette::Result<PathBuf>;

    /// Run a command in a specific workspace.
    async fn run_in_workspace<I, S>(
        &self,
        workspace: &str,
        args: I,
    ) -> miette::Result<Arc<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>;
}

impl JujutsuWorkspaceExt for Jujutsu {
    #[instrument(skip(self))]
    async fn create_workspace(&self, name: &str, path: &Path) -> miette::Result<()> {
        debug!("Creating Jujutsu workspace: {}", name);

        self.process
            .run(
                ["workspace", "add", "--name", name, &path.to_string_lossy()],
                false,
            )
            .await?;

        Ok(())
    }

    async fn list_workspaces(&self) -> miette::Result<Vec<JjWorkspace>> {
        Jujutsu::list_workspaces(self).await
    }

    #[instrument(skip(self))]
    async fn switch_workspace(&self, name: &str) -> miette::Result<()> {
        debug!("Switching to Jujutsu workspace: {}", name);

        let workspace_root = self.get_workspace_root(name).await?;
        std::env::set_current_dir(&workspace_root).into_diagnostic()?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn update_stale_workspace(&self, name: &str) -> miette::Result<()> {
        debug!("Updating stale Jujutsu workspace: {}", name);

        let workspace_root = self.get_workspace_root(name).await?;
        
        // Create a new process cache for the specific workspace
        let workspace_process = ProcessCache::new("jj", &workspace_root);
        
        workspace_process
            .run(["workspace", "update-stale"], false)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn forget_workspace(&self, name: &str) -> miette::Result<()> {
        debug!("Forgetting Jujutsu workspace: {}", name);

        self.process
            .run(["workspace", "forget", name], false)
            .await?;

        Ok(())
    }

    async fn get_workspace_root(&self, name: &str) -> miette::Result<PathBuf> {
        let workspaces = self.list_workspaces().await?;
        
        for workspace in workspaces {
            if workspace.name == name {
                return Ok(workspace.path);
            }
        }

        Err(miette::miette!("Workspace '{}' not found", name))
    }

    async fn run_in_workspace<I, S>(
        &self,
        workspace: &str,
        args: I,
    ) -> miette::Result<Arc<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let workspace_root = self.get_workspace_root(workspace).await?;
        let workspace_process = ProcessCache::new("jj", &workspace_root);
        
        let args_vec: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
        workspace_process.run(args_vec, true).await
    }
}

/// Multi-workspace operations for Jujutsu.
pub struct JujutsuMultiWorkspace {
    /// The main Jujutsu instance.
    jj: Arc<Jujutsu>,
    
    /// All workspaces in the repository.
    workspaces: Vec<JjWorkspace>,
}

impl JujutsuMultiWorkspace {
    pub async fn new(jj: Arc<Jujutsu>) -> miette::Result<Self> {
        let workspaces = jj.list_workspaces().await?;
        
        Ok(Self { jj, workspaces })
    }

    /// Run a task in all workspaces concurrently.
    pub async fn run_in_all_workspaces<F, Fut, T>(
        &self,
        func: F,
    ) -> miette::Result<Vec<(String, T)>>
    where
        F: Fn(Arc<Jujutsu>, String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = miette::Result<T>> + Send,
        T: Send,
    {
        use futures::future::join_all;

        let futures: Vec<_> = self
            .workspaces
            .iter()
            .map(|ws| {
                let jj = Arc::clone(&self.jj);
                let workspace_name = ws.name.clone();
                let func = &func;
                
                async move {
                    let result = func(jj, workspace_name.clone()).await;
                    (workspace_name, result)
                }
            })
            .collect();

        let results = join_all(futures).await;
        
        let mut successful_results = Vec::new();
        for (name, result) in results {
            match result {
                Ok(value) => successful_results.push((name, value)),
                Err(e) => {
                    debug!("Error in workspace {}: {:?}", name, e);
                    return Err(e);
                }
            }
        }

        Ok(successful_results)
    }

    /// Get touched files across all workspaces.
    pub async fn get_all_touched_files(&self) -> miette::Result<Vec<(String, crate::TouchedFiles)>> {
        self.run_in_all_workspaces(|jj, workspace| async move {
            // Create a workspace-specific Jujutsu instance
            let workspace_root = jj.get_workspace_root(&workspace).await?;
            let workspace_jj = Jujutsu::load(
                &workspace_root,
                jj.default_branch.as_str(),
                &jj.remote_candidates,
            )?;
            
            workspace_jj.get_touched_files().await
        })
        .await
    }

    /// Check if any workspace has conflicts.
    pub async fn has_conflicts(&self) -> miette::Result<bool> {
        for workspace in &self.workspaces {
            let output = self.jj.run_in_workspace(
                &workspace.name,
                ["log", "--no-graph", "-r", "@", "-T", "conflict"],
            ).await?;
            
            if output.trim() == "true" {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get workspaces with conflicts.
    pub async fn get_conflicted_workspaces(&self) -> miette::Result<Vec<String>> {
        let mut conflicted = Vec::new();

        for workspace in &self.workspaces {
            let output = self.jj.run_in_workspace(
                &workspace.name,
                ["log", "--no-graph", "-r", "@", "-T", "conflict"],
            ).await?;
            
            if output.trim() == "true" {
                conflicted.push(workspace.name.clone());
            }
        }

        Ok(conflicted)
    }
}