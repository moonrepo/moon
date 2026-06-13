use crate::VcsPlugin;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_common::path::{WorkspaceRelativePath, WorkspaceRelativePathBuf, locate_config_dir};
use moon_pdk_api::{
    GetVcsImpactsInput, GetVcsImpactsOutput, MoonContext, SetupVcsHooksInput,
    TeardownVcsHooksInput, VcsChangeLayer, VcsImpactCompleteness, VcsImpactIntent, VcsObservation,
    VcsPathEffect,
};
use moon_vcs::{ChangedFiles, ChangedStatus, Vcs, VcsHookEnvironment, WorkspaceFiles};
use semver::Version;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

#[derive(Debug)]
pub(crate) struct VcsPluginAdapter {
    baseline_label: String,
    context: MoonContext,
    files: WorkspaceFiles,
    observation: VcsObservation,
    plugin: Arc<VcsPlugin>,
}

impl VcsPluginAdapter {
    pub fn new(
        baseline_label: String,
        context: MoonContext,
        files: WorkspaceFiles,
        observation: VcsObservation,
        plugin: Arc<VcsPlugin>,
    ) -> Self {
        Self {
            baseline_label,
            context,
            files,
            observation,
            plugin,
        }
    }

    async fn impacts(&self, intent: VcsImpactIntent) -> miette::Result<ChangedFiles> {
        let intent = match intent {
            VcsImpactIntent::Submission {
                base,
                head,
                include_workspace,
            } => VcsImpactIntent::Submission {
                base: base.map(|value| self.pin_known_state(value)),
                head: head.map(|value| self.pin_known_state(value)),
                include_workspace,
            },
            intent => intent,
        };
        let output = self
            .plugin
            .get_impacts(GetVcsImpactsInput {
                context: self.context.clone(),
                observation_id: self.observation.id.clone(),
                intent,
            })
            .await?;

        ensure_impacts_available(&output)?;

        Ok(changed_files_from_impacts(output))
    }

    fn baseline(&self) -> Option<&moon_pdk_api::VcsStateIdentity> {
        self.observation.baseline.as_ref()
    }

    fn pin_known_state(&self, value: String) -> String {
        if value == self.observation.current.id
            || self.observation.current.label.as_deref() == Some(&value)
        {
            return self.observation.current.id.clone();
        }

        if let Some(baseline) = self.baseline()
            && (value == baseline.id || baseline.label.as_deref() == Some(&value))
        {
            return baseline.id.clone();
        }

        value
    }
}

fn ensure_impacts_available(output: &GetVcsImpactsOutput) -> miette::Result<()> {
    match output.completeness {
        VcsImpactCompleteness::Exact => Ok(()),
        VcsImpactCompleteness::Conservative => {
            warn!(
                diagnostics = ?output.diagnostics,
                "Source-control provider returned conservative impacts"
            );

            Ok(())
        }
        VcsImpactCompleteness::Unavailable => {
            let diagnostics = output.diagnostics.join("; ");

            Err(if diagnostics.is_empty() {
                miette::miette!("source-control provider could not determine impacted paths")
            } else {
                miette::miette!(
                    "source-control provider could not determine impacted paths: {diagnostics}"
                )
            })
        }
    }
}

fn add_effect(
    changed: &mut ChangedFiles,
    path: String,
    primary: ChangedStatus,
    layers: &[VcsChangeLayer],
) {
    let statuses = changed.files.entry(path.into()).or_default();

    if !statuses.contains(&primary) {
        statuses.push(primary);
    }

    for layer in layers {
        let status = match layer {
            VcsChangeLayer::Recorded | VcsChangeLayer::Staged => ChangedStatus::Staged,
            VcsChangeLayer::Workspace => ChangedStatus::Unstaged,
            VcsChangeLayer::Untracked => ChangedStatus::Untracked,
        };

        if !statuses.contains(&status) {
            statuses.push(status);
        }
    }
}

fn validate_hook_environment(
    workspace_root: &Path,
    hooks_dir: PathBuf,
    working_dir: PathBuf,
) -> miette::Result<VcsHookEnvironment> {
    let workspace_root = workspace_root.canonicalize().into_diagnostic()?;
    let working_dir = working_dir.canonicalize().into_diagnostic()?;

    if !working_dir.starts_with(&workspace_root) {
        return Err(miette::miette!(
            "source-control provider returned a hook working directory outside the workspace"
        ));
    }

    let mut existing = hooks_dir.as_path();
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            miette::miette!("source-control provider returned an invalid hooks directory")
        })?;
    }

    if !existing
        .canonicalize()
        .into_diagnostic()?
        .starts_with(&workspace_root)
    {
        return Err(miette::miette!(
            "source-control provider returned a hooks directory outside the workspace"
        ));
    }

    Ok(VcsHookEnvironment {
        hooks_dir,
        working_dir,
    })
}

fn changed_files_from_impacts(output: GetVcsImpactsOutput) -> ChangedFiles {
    let mut changed = ChangedFiles::default();

    for VcsPathEffect {
        before,
        after,
        layers,
    } in output.effects
    {
        match (before, after) {
            (None, Some(path)) => add_effect(&mut changed, path, ChangedStatus::Added, &layers),
            (Some(path), None) => add_effect(&mut changed, path, ChangedStatus::Deleted, &layers),
            (Some(before), Some(after)) if before != after => {
                add_effect(&mut changed, before, ChangedStatus::Deleted, &layers);
                add_effect(&mut changed, after, ChangedStatus::Added, &layers);
            }
            (Some(path), Some(_)) => {
                add_effect(&mut changed, path, ChangedStatus::Modified, &layers)
            }
            (None, None) => {}
        }
    }

    changed
}

#[async_trait]
impl Vcs for VcsPluginAdapter {
    async fn get_local_branch(&self) -> miette::Result<Arc<String>> {
        Ok(Arc::new(
            self.observation
                .current
                .label
                .clone()
                .unwrap_or_else(|| self.observation.current.id.clone()),
        ))
    }

    async fn get_local_branch_revision(&self) -> miette::Result<Arc<String>> {
        Ok(Arc::new(self.observation.current.id.clone()))
    }

    async fn get_default_branch(&self) -> miette::Result<Arc<String>> {
        Ok(Arc::new(
            self.baseline()
                .and_then(|baseline| {
                    baseline
                        .label
                        .clone()
                        .or_else(|| (!baseline.id.is_empty()).then(|| baseline.id.clone()))
                })
                .unwrap_or_else(|| self.baseline_label.clone()),
        ))
    }

    async fn get_default_branch_revision(&self) -> miette::Result<Arc<String>> {
        self.baseline()
            .filter(|baseline| !baseline.id.is_empty())
            .map(|baseline| Arc::new(baseline.id.clone()))
            .ok_or_else(|| {
                miette::miette!(
                    "source-control provider could not resolve baseline `{}`",
                    self.baseline_label
                )
            })
    }

    async fn get_file_hashes(
        &self,
        files: &[WorkspaceRelativePathBuf],
        allow_ignored: bool,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        self.files.hash_files(files, allow_ignored).await
    }

    async fn get_file_tree(
        &self,
        dir: &WorkspaceRelativePath,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        self.files.list(dir)
    }

    async fn get_repository_root(&self) -> miette::Result<PathBuf> {
        Ok(self
            .plugin
            .from_virtual_path(&self.observation.repository_root))
    }

    async fn get_repository_slug(&self) -> miette::Result<Arc<String>> {
        self.observation
            .repository_slug
            .clone()
            .map(Arc::new)
            .ok_or_else(|| miette::miette!("source-control provider reported no repository slug"))
    }

    async fn get_changed_files(&self) -> miette::Result<ChangedFiles> {
        self.impacts(VcsImpactIntent::Workspace).await
    }

    async fn get_changed_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> miette::Result<ChangedFiles> {
        let head = if self.is_default_branch(revision) {
            self.observation.current.id.clone()
        } else {
            revision.to_owned()
        };

        self.impacts(VcsImpactIntent::Submission {
            base: None,
            head: (!head.is_empty()).then_some(head),
            include_workspace: false,
        })
        .await
    }

    async fn get_changed_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> miette::Result<ChangedFiles> {
        self.impacts(VcsImpactIntent::Submission {
            base: (!base_revision.is_empty()).then(|| base_revision.to_owned()),
            head: (!revision.is_empty()).then(|| revision.to_owned()),
            include_workspace: revision.is_empty(),
        })
        .await
    }

    async fn get_version(&self) -> miette::Result<Version> {
        let version = self
            .observation
            .client_version
            .as_deref()
            .unwrap_or(&self.plugin.metadata.plugin_version);
        let version = version
            .split_whitespace()
            .find(|part| {
                part.chars()
                    .next()
                    .is_some_and(|char| char.is_ascii_digit())
            })
            .unwrap_or(version);

        Version::parse(version).into_diagnostic()
    }

    async fn get_working_root(&self) -> miette::Result<PathBuf> {
        Ok(self
            .plugin
            .from_virtual_path(&self.observation.working_root))
    }

    fn is_default_branch(&self, branch: &str) -> bool {
        branch == self.baseline_label
            || self.baseline().is_some_and(|baseline| {
                baseline.label.as_deref() == Some(branch)
                    || self.observation.current.label.as_deref() == Some(branch)
                        && self.observation.current.id == baseline.id
            })
    }

    fn is_enabled(&self) -> bool {
        self.observation.enabled
    }

    fn is_ignored(&self, file: &Path) -> bool {
        self.files.is_ignored(file)
    }

    async fn is_shallow_checkout(&self) -> miette::Result<bool> {
        Ok(matches!(
            self.observation.history,
            moon_pdk_api::VcsHistoryCompleteness::Incomplete
        ))
    }

    async fn setup_hooks(&self) -> miette::Result<Option<VcsHookEnvironment>> {
        if !self.plugin.metadata.supports_hooks {
            return Ok(None);
        }

        let workspace_root = self.plugin.from_virtual_path(&self.context.workspace_root);
        let hooks_dir = locate_config_dir(&workspace_root).join("hooks");
        let relative_hooks_dir = hooks_dir
            .strip_prefix(&workspace_root)
            .into_diagnostic()?
            .to_string_lossy()
            .replace('\\', "/");
        let output = self
            .plugin
            .setup_hooks(SetupVcsHooksInput {
                context: self.context.clone(),
                observation_id: self.observation.id.clone(),
                hooks_dir: relative_hooks_dir,
            })
            .await?;

        Ok(match output.working_dir {
            Some(working_dir) => Some(validate_hook_environment(
                &workspace_root,
                hooks_dir,
                self.plugin.from_virtual_path(working_dir),
            )?),
            None => None,
        })
    }

    async fn teardown_hooks(&self) -> miette::Result<()> {
        if self.plugin.metadata.supports_hooks {
            self.plugin
                .teardown_hooks(TeardownVcsHooksInput {
                    context: self.context.clone(),
                    observation_id: self.observation.id.clone(),
                })
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_pdk_api::{GetVcsImpactsOutput, VcsImpactCompleteness};
    use starbase_sandbox::create_empty_sandbox;

    #[test]
    fn converts_renames_and_provider_layers() {
        let changed = changed_files_from_impacts(GetVcsImpactsOutput {
            effects: vec![VcsPathEffect {
                before: Some("old.txt".into()),
                after: Some("new.txt".into()),
                layers: vec![VcsChangeLayer::Staged],
            }],
            completeness: VcsImpactCompleteness::Exact,
            diagnostics: vec![],
        });

        assert_eq!(
            changed
                .files
                .get(&WorkspaceRelativePathBuf::from("old.txt")),
            Some(&vec![ChangedStatus::Deleted, ChangedStatus::Staged])
        );
        assert_eq!(
            changed
                .files
                .get(&WorkspaceRelativePathBuf::from("new.txt")),
            Some(&vec![ChangedStatus::Added, ChangedStatus::Staged])
        );
    }

    #[test]
    fn rejects_unavailable_impacts() {
        let output = GetVcsImpactsOutput {
            completeness: VcsImpactCompleteness::Unavailable,
            diagnostics: vec!["history is unavailable".into()],
            ..Default::default()
        };

        assert!(ensure_impacts_available(&output).is_err());
    }

    #[test]
    fn rejects_hook_paths_outside_the_workspace() {
        let workspace = create_empty_sandbox();
        let external = create_empty_sandbox();

        assert!(
            validate_hook_environment(
                workspace.path(),
                external.path().join("hooks"),
                workspace.path().to_owned(),
            )
            .is_err()
        );
        assert!(
            validate_hook_environment(
                workspace.path(),
                workspace.path().join(".moon/hooks"),
                external.path().to_owned(),
            )
            .is_err()
        );
    }
}
