//! Executable conformance checks for the provider-oriented interface.

use crate::plugin::load_prototype_plugin;
use miette::{IntoDiagnostic, miette};
use moon_pdk_api::*;
use moon_plugin::{MoonEnvironment, MoonHostData, ProtoEnvironment};
use moon_vcs::BoxedVcs;
use moon_vcs_plugin::{VcsPluginConfig, load_vcs_adapter};
use serde::Serialize;
use starbase_utils::hash;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConformanceReport {
    provider: String,
    observation_id: String,
    passed: bool,
    scenarios: Vec<Scenario>,
}

#[derive(Serialize)]
struct Scenario {
    name: &'static str,
    passed: bool,
}

pub async fn run(moon_root: &Path) -> miette::Result<()> {
    let fixture = create_fixture(true)?;
    let plugin = load_prototype_plugin(moon_root, &fixture).await?;
    let context = MoonContext {
        working_dir: plugin.to_virtual_path(&fixture),
        workspace_root: plugin.to_virtual_path(&fixture),
    };
    let detection = plugin
        .detect(DetectVcsInput {
            context: context.clone(),
        })
        .await?;

    if !detection.active {
        return Err(miette!(
            "Jujutsu provider did not activate: {}",
            detection.reason
        ));
    }

    let observation = plugin
        .observe(ObserveVcsInput {
            baseline: Some("master".into()),
            remote_candidates: vec![],
            consistency: VcsConsistency::FreshObservation,
            context: context.clone(),
        })
        .await?;
    let workspace = plugin
        .get_impacts(GetVcsImpactsInput {
            context: context.clone(),
            observation_id: observation.id.clone(),
            intent: VcsImpactIntent::Workspace,
        })
        .await?;
    let submission = plugin
        .get_impacts(GetVcsImpactsInput {
            context: context.clone(),
            observation_id: observation.id.clone(),
            intent: VcsImpactIntent::Submission {
                base: Some("master".into()),
                head: None,
                include_workspace: true,
            },
        })
        .await?;

    fs::write(fixture.join("after-observation.txt"), "later\n").into_diagnostic()?;
    let pinned = plugin
        .get_impacts(GetVcsImpactsInput {
            context: context.clone(),
            observation_id: observation.id.clone(),
            intent: VcsImpactIntent::Workspace,
        })
        .await?;
    let ambiguous = plugin
        .get_impacts(GetVcsImpactsInput {
            context,
            observation_id: observation.id.clone(),
            intent: VcsImpactIntent::Submission {
                base: Some("all()".into()),
                head: None,
                include_workspace: false,
            },
        })
        .await;
    let selected = load_production_adapter(moon_root, &fixture).await?;
    let selected_revision = selected.get_local_branch_revision().await?;
    let jj_revision = command_output(
        &fixture,
        "jj",
        ["log", "--no-graph", "-r", "@", "-T", "commit_id"],
    )?;
    let git_fixture = create_fixture(false)?;
    let fallback = load_production_adapter(moon_root, &git_fixture).await?;
    let git_head = command_output(&git_fixture, "git", ["rev-parse", "HEAD"])?;
    let nested_fixture = create_fixture(true)?;
    let nested_workspace = nested_fixture.join("nested");
    fs::create_dir_all(&nested_workspace).into_diagnostic()?;
    fs::write(nested_workspace.join("working.txt"), "nested\n").into_diagnostic()?;
    let nested_plugin = load_prototype_plugin(moon_root, &nested_workspace).await?;
    let nested_context = MoonContext {
        working_dir: nested_plugin.to_virtual_path(&nested_workspace),
        workspace_root: nested_plugin.to_virtual_path(&nested_workspace),
    };
    let nested_observation = nested_plugin
        .observe(ObserveVcsInput {
            baseline: Some("master".into()),
            remote_candidates: vec![],
            consistency: VcsConsistency::FreshObservation,
            context: nested_context.clone(),
        })
        .await?;
    let nested_impacts = nested_plugin
        .get_impacts(GetVcsImpactsInput {
            context: nested_context,
            observation_id: nested_observation.id,
            intent: VcsImpactIntent::Workspace,
        })
        .await?;

    let workspace_paths = paths(&workspace.effects);
    let submission_paths = paths(&submission.effects);
    let pinned_paths = paths(&pinned.effects);
    let scenarios = vec![
        Scenario {
            name: "provider returns a complete observation",
            passed: observation.provider == "jj"
                && observation.enabled
                && !observation.current.id.is_empty()
                && observation.baseline.is_some(),
        },
        Scenario {
            name: "workspace intent reports local effects",
            passed: workspace_paths.contains("working.txt"),
        },
        Scenario {
            name: "submission intent combines recorded and workspace effects",
            passed: submission_paths.contains("feature.txt")
                && submission_paths.contains("working.txt"),
        },
        Scenario {
            name: "observation pins later impact queries",
            passed: !pinned_paths.contains("after-observation.txt"),
        },
        Scenario {
            name: "provider rejects ambiguous external references",
            passed: ambiguous.is_err(),
        },
        Scenario {
            name: "configured provider is selected without composition",
            passed: selected_revision.as_str() == jj_revision.trim(),
        },
        Scenario {
            name: "inactive candidate falls back during provider selection",
            passed: fallback.get_local_branch_revision().await?.as_str() == git_head.trim(),
        },
        Scenario {
            name: "provider scopes paths to a nested Moon workspace",
            passed: paths(&nested_impacts.effects) == BTreeSet::from(["working.txt"]),
        },
    ];
    let passed = scenarios.iter().all(|scenario| scenario.passed);

    println!(
        "{}",
        serde_json::to_string_pretty(&ConformanceReport {
            provider: observation.provider,
            observation_id: observation.id,
            passed,
            scenarios,
        })
        .into_diagnostic()?
    );
    fs::remove_dir_all(fixture).into_diagnostic()?;
    fs::remove_dir_all(git_fixture).into_diagnostic()?;
    fs::remove_dir_all(nested_fixture).into_diagnostic()?;

    if passed {
        Ok(())
    } else {
        Err(miette!("source-control provider conformance failed"))
    }
}

fn paths(effects: &[VcsPathEffect]) -> BTreeSet<&str> {
    let mut paths = BTreeSet::new();

    for effect in effects {
        if let Some(path) = effect.before.as_deref() {
            paths.insert(path);
        }
        if let Some(path) = effect.after.as_deref() {
            paths.insert(path);
        }
    }

    paths
}

async fn load_production_adapter(moon_root: &Path, fixture: &Path) -> miette::Result<BoxedVcs> {
    let wasm_file = moon_root.join("wasm/target/wasm32-wasip1/release/vcs_jj_prototype.wasm");
    let moon_env = MoonEnvironment::new_testing(fixture);
    fs::create_dir_all(&moon_env.store_root).into_diagnostic()?;
    fs::write(
        moon_env.store_root.join("vcs.json"),
        serde_json::to_vec_pretty(&VcsPluginConfig {
            enabled: true,
            plugin: warpgate::PluginLocator::File(Box::new(warpgate::FileLocator {
                file: format!("file://{}", wasm_file.display()),
                path: Some(wasm_file.clone()),
            })),
            sha256: hash::sha256::from_file(&wasm_file)?,
        })
        .into_diagnostic()?,
    )
    .into_diagnostic()?;

    load_vcs_adapter(
        MoonHostData {
            moon_env: Arc::new(moon_env),
            proto_env: Arc::new(ProtoEnvironment::new_testing(fixture)?),
            ..Default::default()
        },
        fixture,
        fixture,
        "master",
        &[],
    )
    .await
}

fn create_fixture(with_jj: bool) -> miette::Result<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .into_diagnostic()?
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "moon-vcs-provider-conformance-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).into_diagnostic()?;
    command(&root, "git", ["init", "-b", "master"])?;
    fs::write(root.join("initial.txt"), "initial\n").into_diagnostic()?;
    command(&root, "git", ["add", "."])?;
    command(
        &root,
        "git",
        [
            "-c",
            "user.name=Moon",
            "-c",
            "user.email=moon@example.com",
            "commit",
            "-m",
            "initial",
        ],
    )?;
    command(&root, "git", ["checkout", "-b", "feature"])?;
    fs::write(root.join("feature.txt"), "feature\n").into_diagnostic()?;
    command(&root, "git", ["add", "."])?;
    command(
        &root,
        "git",
        [
            "-c",
            "user.name=Moon",
            "-c",
            "user.email=moon@example.com",
            "commit",
            "-m",
            "feature",
        ],
    )?;
    if with_jj {
        command(&root, "jj", ["git", "init", "--colocate", "."])?;
        fs::write(root.join("working.txt"), "working\n").into_diagnostic()?;
    }

    Ok(root)
}

fn command_output<I, S>(cwd: &Path, executable: &str, args: I) -> miette::Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new(executable)
        .args(args)
        .current_dir(cwd)
        .output()
        .into_diagnostic()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(miette!(
            "{executable} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn command<I, S>(cwd: &Path, executable: &str, args: I) -> miette::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new(executable)
        .args(args)
        .current_dir(cwd)
        .output()
        .into_diagnostic()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(miette!(
            "{executable} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
