use crate::{VcsPlugin, adapter::VcsPluginAdapter};
use miette::IntoDiagnostic;
use moon_pdk_api::{DetectVcsInput, MoonContext, ObserveVcsInput, VcsConsistency};
use moon_plugin::{MoonHostData, PluginLocator, PluginRegistry, PluginType};
use moon_vcs::{BoxedVcs, WorkspaceFiles, git::Git};
use serde::{Deserialize, Serialize};
use starbase_utils::hash;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;
use warpgate::{DataLocator, Id};

const USER_CONFIG_FILE: &str = "vcs.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VcsPluginConfig {
    pub enabled: bool,
    pub plugin: PluginLocator,
    pub sha256: String,
}

pub fn get_user_vcs_config_path(host_data: &MoonHostData) -> PathBuf {
    host_data.moon_env.store_root.join(USER_CONFIG_FILE)
}

pub fn load_user_vcs_config(path: &Path) -> miette::Result<Option<VcsPluginConfig>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path).into_diagnostic()?;
    let mut config = serde_json::from_str::<VcsPluginConfig>(&content).into_diagnostic()?;
    if config.enabled {
        validate_config(&mut config)?;
    }

    Ok(Some(config))
}

pub async fn load_vcs_adapter(
    host_data: MoonHostData,
    working_dir: &Path,
    workspace_root: &Path,
    baseline: &str,
    remote_candidates: &[String],
) -> miette::Result<BoxedVcs> {
    let config_path = get_user_vcs_config_path(&host_data);
    let config = load_user_vcs_config(&config_path)?;

    if let Some(config) = config.filter(|config| config.enabled) {
        let plugin = load_verified_vcs_plugin(
            host_data.clone(),
            Id::raw("user-vcs"),
            config.plugin,
            &config.sha256,
        )
        .await?;

        if let Some(adapter) = activate_provider(
            plugin,
            working_dir,
            workspace_root,
            baseline,
            remote_candidates,
            true,
        )
        .await?
        {
            return Ok(adapter);
        }
    }

    if !Git::is_repository(workspace_root) {
        return Ok(Box::new(Git::load(
            workspace_root,
            baseline,
            remote_candidates,
        )?));
    }

    let plugin = load_bundled_git_plugin(host_data).await?;

    activate_provider(
        plugin,
        working_dir,
        workspace_root,
        baseline,
        remote_candidates,
        false,
    )
    .await?
    .ok_or_else(|| miette::miette!("bundled Git provider returned no adapter"))
}

async fn activate_provider(
    plugin: Arc<VcsPlugin>,
    working_dir: &Path,
    workspace_root: &Path,
    baseline: &str,
    remote_candidates: &[String],
    require_active: bool,
) -> miette::Result<Option<BoxedVcs>> {
    let context = MoonContext {
        working_dir: plugin.to_virtual_path(working_dir),
        workspace_root: plugin.to_virtual_path(workspace_root),
    };
    let detection = if require_active {
        let detection = plugin
            .detect(DetectVcsInput {
                context: context.clone(),
            })
            .await?;

        if !detection.active {
            debug!(
                reason = detection.reason,
                "Source-control provider is not active"
            );

            return Ok(None);
        }

        Some(detection)
    } else {
        None
    };

    let observation = plugin
        .observe(ObserveVcsInput {
            baseline: Some(baseline.to_owned()),
            remote_candidates: remote_candidates.to_owned(),
            context: context.clone(),
            consistency: VcsConsistency::FreshObservation,
        })
        .await?;

    debug!(
        plugin = plugin.metadata.name,
        reason = detection.map(|output| output.reason),
        "Activated source-control provider"
    );

    Ok(Some(Box::new(VcsPluginAdapter::new(
        baseline.to_owned(),
        context,
        WorkspaceFiles::new(workspace_root)?,
        observation,
        plugin,
    ))))
}

pub async fn load_verified_vcs_plugin(
    host_data: MoonHostData,
    id: Id,
    locator: PluginLocator,
    expected_sha256: &str,
) -> miette::Result<Arc<VcsPlugin>> {
    let registry = PluginRegistry::new(PluginType::Vcs, host_data)?;
    let expected_sha256 = expected_sha256.to_owned();

    registry
        .load_verified_with_config(
            id,
            locator,
            move |wasm_file, bytes| verify_sha256(wasm_file, bytes, &expected_sha256),
            configure_vcs_manifest,
        )
        .await
}

async fn load_bundled_git_plugin(host_data: MoonHostData) -> miette::Result<Arc<VcsPlugin>> {
    let mut moon_env = (*host_data.moon_env).clone();
    let cache_root = std::env::temp_dir().join("moon-vcs-plugins");
    moon_env.plugins_dir = cache_root.join("plugins");
    moon_env.temp_dir = cache_root.join("temp");
    let host_data = MoonHostData {
        moon_env: Arc::new(moon_env),
        ..host_data
    };
    let registry = PluginRegistry::new(PluginType::Vcs, host_data)?;
    let locator = PluginLocator::Data(Box::new(DataLocator {
        data: "data://vcs_git".into(),
        bytes: Some(include_bytes!("../res/vcs_git.wasm").to_vec()),
    }));

    registry
        .load_with_config(Id::raw("git"), locator, configure_vcs_manifest)
        .await
}

fn configure_vcs_manifest(manifest: &mut warpgate::PluginManifest) -> miette::Result<()> {
    manifest.allowed_hosts = Some(vec![]);
    manifest.allowed_paths = Some(Default::default());
    manifest.timeout_ms = Some(120_000);

    Ok(())
}

fn verify_sha256(wasm_file: &Path, bytes: &[u8], expected: &str) -> miette::Result<()> {
    let actual = hash::sha256::from_bytes(bytes);

    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(miette::miette!(
            "VCS plugin integrity check failed for {}: expected SHA-256 {expected}, received {actual}",
            wasm_file.display()
        ))
    }
}

fn validate_config(config: &mut VcsPluginConfig) -> miette::Result<()> {
    if config.sha256.len() != 64
        || !config
            .sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(miette::miette!(
            "trusted plugin SHA-256 must contain exactly 64 hexadecimal characters"
        ));
    }

    match &mut config.plugin {
        PluginLocator::File(file) => {
            let path = file.get_unresolved_path();

            if !path.is_absolute() {
                return Err(miette::miette!(
                    "user VCS plugin file locators must use an absolute path"
                ));
            }

            file.path = Some(path);
        }
        PluginLocator::Url(url) if !url.url.starts_with("https://") => {
            return Err(miette::miette!("user VCS plugin URLs must use HTTPS"));
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_common::path::WorkspaceRelativePathBuf;
    use moon_plugin::{MoonEnvironment, ProtoEnvironment};
    use starbase_sandbox::create_empty_sandbox;
    use std::process::Command;
    use std::sync::Arc;
    use warpgate::FileLocator;

    fn create_host_data(sandbox: &Path) -> MoonHostData {
        MoonHostData {
            moon_env: Arc::new(MoonEnvironment::new_testing(sandbox)),
            proto_env: Arc::new(ProtoEnvironment::new_testing(sandbox).unwrap()),
            ..Default::default()
        }
    }

    fn create_nested_host_data(sandbox: &Path, workspace_root: &Path) -> MoonHostData {
        let mut moon_env = MoonEnvironment::new_testing(sandbox);
        moon_env.working_dir = workspace_root.to_owned();
        moon_env.workspace_root = workspace_root.to_owned();

        MoonHostData {
            moon_env: Arc::new(moon_env),
            proto_env: Arc::new(ProtoEnvironment::new_testing(workspace_root).unwrap()),
            ..Default::default()
        }
    }

    fn create_git_repository() -> starbase_sandbox::Sandbox {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();
        sandbox.create_file("initial.txt", "initial");
        sandbox.run_git(|command| {
            command.args(["add", "."]);
        });
        sandbox.run_git(|command| {
            command.args([
                "-c",
                "user.name=Moon",
                "-c",
                "user.email=moon@example.com",
                "commit",
                "-m",
                "initial",
            ]);
        });

        sandbox
    }

    fn git_output(repository: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(repository)
            .output()
            .unwrap();
        assert!(output.status.success());

        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    #[test]
    fn loads_valid_user_config() {
        let sandbox = create_empty_sandbox();
        let plugin_file = sandbox.path().join("plugin.wasm");
        let config_file = sandbox.path().join(USER_CONFIG_FILE);
        let config = VcsPluginConfig {
            enabled: true,
            plugin: PluginLocator::File(Box::new(FileLocator {
                file: format!("file://{}", plugin_file.display()),
                path: Some(plugin_file),
            })),
            sha256: "a".repeat(64),
        };
        fs::write(&config_file, serde_json::to_string(&config).unwrap()).unwrap();

        assert_eq!(load_user_vcs_config(&config_file).unwrap(), Some(config));
    }

    #[test]
    fn rejects_invalid_user_config() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join(USER_CONFIG_FILE);
        let config = VcsPluginConfig {
            enabled: true,
            plugin: PluginLocator::File(Box::new(warpgate::FileLocator {
                file: "file://relative.wasm".into(),
                path: Some("relative.wasm".into()),
            })),
            sha256: "invalid".into(),
        };
        fs::write(&config_file, serde_json::to_string(&config).unwrap()).unwrap();

        assert!(load_user_vcs_config(&config_file).is_err());
    }

    #[test]
    fn allows_disabled_user_config_with_an_invalid_pin() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join(USER_CONFIG_FILE);
        let config = VcsPluginConfig {
            enabled: false,
            plugin: PluginLocator::File(Box::new(warpgate::FileLocator {
                file: "file://relative.wasm".into(),
                path: Some("relative.wasm".into()),
            })),
            sha256: "invalid".into(),
        };
        fs::write(&config_file, serde_json::to_string(&config).unwrap()).unwrap();

        let loaded = load_user_vcs_config(&config_file).unwrap().unwrap();
        assert!(!loaded.enabled);
        assert_eq!(loaded.sha256, config.sha256);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_supplies_complete_source_control() {
        let sandbox = create_git_repository();
        sandbox.create_file("working.txt", "working");
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "master",
            &[],
        )
        .await
        .unwrap();
        sandbox.create_file("after-observation.txt", "later");

        assert!(adapter.is_enabled());
        assert!(
            !adapter
                .get_local_branch_revision()
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            adapter.get_default_branch().await.unwrap().as_str(),
            "master"
        );
        let changed = adapter.get_changed_files().await.unwrap();

        assert!(
            changed
                .files
                .contains_key(&WorkspaceRelativePathBuf::from("working.txt"))
        );
        assert!(
            changed
                .files
                .keys()
                .all(|path| !path.as_str().contains("plugins/vcs"))
        );
        assert!(
            !changed
                .files
                .contains_key(&WorkspaceRelativePathBuf::from("after-observation.txt"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_the_bundled_provider_without_a_git_repository() {
        let sandbox = create_empty_sandbox();
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "master",
            &[],
        )
        .await
        .unwrap();

        assert!(!adapter.is_enabled());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_tolerates_a_missing_baseline() {
        let sandbox = create_git_repository();
        sandbox.create_file("working.txt", "working");
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "not-fetched",
            &[],
        )
        .await
        .unwrap();

        assert!(adapter.is_enabled());
        assert!(
            adapter
                .get_changed_files()
                .await
                .unwrap()
                .files
                .contains_key(&WorkspaceRelativePathBuf::from("working.txt"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_reports_no_previous_changes_for_a_root_commit() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("initial.txt", "initial");
        sandbox.enable_git();
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "master",
            &[],
        )
        .await
        .unwrap();

        assert!(
            adapter
                .get_changed_files_against_previous_revision("master")
                .await
                .unwrap()
                .files
                .is_empty()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_supports_an_unborn_repository() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();
        sandbox.create_file("working.txt", "working");
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "master",
            &[],
        )
        .await
        .unwrap();

        assert!(adapter.is_enabled());
        assert!(
            adapter
                .get_changed_files()
                .await
                .unwrap()
                .files
                .contains_key(&WorkspaceRelativePathBuf::from("working.txt"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_uses_configured_remote_candidates() {
        let sandbox = create_git_repository();
        sandbox.run_git(|command| {
            command.args([
                "remote",
                "add",
                "fork",
                "https://github.com/moonrepo/custom.git",
            ]);
        });
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "master",
            &["fork".into()],
        )
        .await
        .unwrap();

        assert_eq!(
            adapter.get_repository_slug().await.unwrap().as_str(),
            "moonrepo/custom"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_includes_changes_inside_submodules() {
        let submodule = create_git_repository();
        let sandbox = create_git_repository();
        assert!(
            Command::new("git")
                .args([
                    "-c",
                    "protocol.file.allow=always",
                    "submodule",
                    "add",
                    submodule.path().to_str().unwrap(),
                    "modules/child",
                ])
                .current_dir(sandbox.path())
                .status()
                .unwrap()
                .success()
        );
        sandbox.run_git(|command| {
            command.args(["add", "."]);
        });
        sandbox.run_git(|command| {
            command.args([
                "-c",
                "user.name=Moon",
                "-c",
                "user.email=moon@example.com",
                "commit",
                "-m",
                "add submodule",
            ]);
        });
        sandbox.create_file("modules/child/initial.txt", "changed");
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "master",
            &[],
        )
        .await
        .unwrap();

        assert!(
            adapter
                .get_changed_files()
                .await
                .unwrap()
                .files
                .contains_key(&WorkspaceRelativePathBuf::from("modules/child/initial.txt"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_includes_revision_changes_inside_submodules() {
        let submodule = create_git_repository();
        let sandbox = create_git_repository();
        assert!(
            Command::new("git")
                .args([
                    "-c",
                    "protocol.file.allow=always",
                    "submodule",
                    "add",
                    submodule.path().to_str().unwrap(),
                    "modules/child",
                ])
                .current_dir(sandbox.path())
                .status()
                .unwrap()
                .success()
        );
        sandbox.run_git(|command| {
            command.args(["add", "."]);
        });
        sandbox.run_git(|command| {
            command.args([
                "-c",
                "user.name=Moon",
                "-c",
                "user.email=moon@example.com",
                "commit",
                "-m",
                "add submodule",
            ]);
        });
        let base = git_output(sandbox.path(), &["rev-parse", "HEAD"]);
        let checked_out_submodule = sandbox.path().join("modules/child");
        sandbox.create_file("modules/child/initial.txt", "changed");
        assert!(
            Command::new("git")
                .args([
                    "-c",
                    "user.name=Moon",
                    "-c",
                    "user.email=moon@example.com",
                    "commit",
                    "-am",
                    "change submodule",
                ])
                .current_dir(&checked_out_submodule)
                .status()
                .unwrap()
                .success()
        );
        sandbox.run_git(|command| {
            command.args(["add", "modules/child"]);
        });
        sandbox.run_git(|command| {
            command.args([
                "-c",
                "user.name=Moon",
                "-c",
                "user.email=moon@example.com",
                "commit",
                "-m",
                "update submodule",
            ]);
        });
        let head = git_output(sandbox.path(), &["rev-parse", "HEAD"]);
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "master",
            &[],
        )
        .await
        .unwrap();

        assert!(
            adapter
                .get_changed_files_between_revisions(&base, &head)
                .await
                .unwrap()
                .files
                .contains_key(&WorkspaceRelativePathBuf::from("modules/child/initial.txt"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_exposes_optional_hooks() {
        let sandbox = create_git_repository();
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "master",
            &[],
        )
        .await
        .unwrap();
        let hooks = adapter.setup_hooks().await.unwrap().unwrap();

        assert_eq!(hooks.hooks_dir, sandbox.path().join(".moon/hooks"));
        adapter.teardown_hooks().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_uses_the_workspace_config_directory_for_hooks() {
        let sandbox = create_git_repository();
        sandbox.create_file(".config/moon/workspace.yml", "projects: {}\n");
        let adapter = load_vcs_adapter(
            create_host_data(sandbox.path()),
            sandbox.path(),
            sandbox.path(),
            "master",
            &[],
        )
        .await
        .unwrap();
        let hooks = adapter.setup_hooks().await.unwrap().unwrap();

        assert_eq!(hooks.hooks_dir, sandbox.path().join(".config/moon/hooks"));
        adapter.teardown_hooks().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundled_git_provider_scopes_changes_to_a_nested_workspace() {
        let sandbox = create_git_repository();
        sandbox.create_file("workspace/inside.txt", "inside");
        sandbox.run_git(|command| {
            command.args(["add", "."]);
        });
        sandbox.run_git(|command| {
            command.args([
                "-c",
                "user.name=Moon",
                "-c",
                "user.email=moon@example.com",
                "commit",
                "-m",
                "nested workspace",
            ]);
        });
        let workspace_root = sandbox.path().join("workspace");
        sandbox.create_file("outside.txt", "outside");
        sandbox.create_file("workspace/inside.txt", "changed");
        let adapter = load_vcs_adapter(
            create_nested_host_data(sandbox.path(), &workspace_root),
            &workspace_root,
            &workspace_root,
            "master",
            &[],
        )
        .await
        .unwrap();

        let changed = adapter.get_changed_files().await.unwrap();

        assert_eq!(
            changed.files.len(),
            1,
            "unexpected files: {:?}",
            changed.files
        );
        assert!(
            changed
                .files
                .contains_key(&WorkspaceRelativePathBuf::from("inside.txt"))
        );

        let hooks = adapter.setup_hooks().await.unwrap().unwrap();
        let configured_hooks = Command::new("git")
            .args(["config", "--get", "core.hooksPath"])
            .current_dir(sandbox.path())
            .output()
            .unwrap();

        assert_eq!(hooks.hooks_dir, workspace_root.join(".moon/hooks"));
        assert_eq!(
            String::from_utf8_lossy(&configured_hooks.stdout).trim(),
            "workspace/.moon/hooks"
        );
        adapter.teardown_hooks().await.unwrap();
    }
}
