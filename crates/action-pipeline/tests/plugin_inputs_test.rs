use moon_action_graph::BuildOutput;
use moon_common::Id;
use moon_test_utils::WorkspaceMocker;
use serde_json::Value;
use starbase_sandbox::{Sandbox, create_sandbox};

const TC: &str = "tc-tier3-setup-env";

fn create_mocker(sandbox: &Sandbox) -> WorkspaceMocker {
    WorkspaceMocker::new(sandbox.path())
        .load_default_configs()
        .with_test_toolchains()
        .update_toolchains_config(|cfg| {
            let plugin = cfg.plugins.entry(Id::raw(TC)).or_default();

            plugin
                .config
                .insert("testLocateRoots".into(), serde_json::json!(true));
            plugin
                .config
                .insert("testRecordInputs".into(), serde_json::json!(true));
        })
}

async fn run_installs(mocker: &WorkspaceMocker, project_ids: &[&str]) {
    let workspace_graph = mocker.mock_workspace_graph().await;
    let mut builder = mocker.create_action_graph().await;

    for id in project_ids {
        builder.install_dependencies_by_project(workspace_graph.get_project(id).unwrap());
    }

    let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

    mocker
        .mock_action_pipeline()
        .await
        .run_with_context(graph, context)
        .await
        .unwrap();
}

fn read_inputs(sandbox: &Sandbox, func: &str) -> Vec<Value> {
    let dir = sandbox
        .path()
        .join(".moon/cache/tcInputs")
        .join(TC)
        .join(func);

    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };

    entries
        .map(|entry| {
            serde_json::from_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap()).unwrap()
        })
        .collect()
}

fn write_project(sandbox: &Sandbox, source: &str, toolchains: &str) {
    sandbox.create_file(
        format!("{source}/moon.yml"),
        format!("{toolchains}\ntasks:\n  build:\n    command: 'build'\n"),
    );
}

fn uses_toolchain(sandbox: &Sandbox, source: &str) {
    write_project(sandbox, source, &format!("toolchains:\n  default: '{TC}'"));
}

// Also sets a custom setting, to check which config is passed
fn overrides_version(sandbox: &Sandbox, source: &str) {
    write_project(
        sandbox,
        source,
        &format!(
            "toolchains:\n  default: '{TC}'\n  {TC}:\n    version: '1.99.0'\n    customKey: '{source}'"
        ),
    );
}

/// The `configured_version` and config `version` of each toolchain setup.
fn get_setup_toolchain_versions(sandbox: &Sandbox) -> Vec<(Value, Value)> {
    let mut versions = read_inputs(sandbox, "setup_toolchain")
        .into_iter()
        .map(|input| {
            (
                input["configured_version"].clone(),
                input["toolchain_config"]["version"].clone(),
            )
        })
        .collect::<Vec<_>>();

    versions.sort_by_key(|(version, _)| version.to_string());
    versions
}

fn get_single_input(sandbox: &Sandbox, func: &str) -> Value {
    let mut inputs = read_inputs(sandbox, func);

    assert_eq!(inputs.len(), 1, "Expected one {func} input");

    inputs.remove(0)
}

fn assert_root_input(input: &Value, root: &str, project: Option<&str>, version: &str) {
    assert_eq!(input["root"], root);
    assert_eq!(input["toolchain_config"]["version"], version);

    match project {
        Some(id) => {
            assert_eq!(input["project"]["id"], id);
            assert_eq!(input["toolchain_config"]["customKey"], *id);
        }
        None => {
            assert!(input["project"].is_null());
            assert!(input["toolchain_config"].get("customKey").is_none());
        }
    };
}

mod plugin_inputs {
    use super::*;

    mod workspace {
        use super::*;

        // The root's actions run with the workspace's version and config, and
        // without a project, while the member's version is only setup
        #[tokio::test(flavor = "multi_thread")]
        async fn with_overrides() {
            let sandbox = create_sandbox("shared-root");

            overrides_version(&sandbox, "packages/a");
            uses_toolchain(&sandbox, "packages/b");

            run_installs(&create_mocker(&sandbox), &["a", "b"]).await;

            let env = get_single_input(&sandbox, "setup_environment");
            let install = get_single_input(&sandbox, "install_dependencies");

            assert_root_input(&env, "/workspace", None, "1.2.3");
            assert_root_input(&install, "/workspace", None, "1.2.3");
            assert_eq!(install["packages"], serde_json::json!([]));
            assert_eq!(install["production"], false);
            assert_eq!(
                get_setup_toolchain_versions(&sandbox),
                [
                    ("1.2.3".into(), "1.2.3".into()),
                    ("1.99.0".into(), "1.99.0".into())
                ]
            );
        }

        // The root's actions run with the root project's version and config,
        // but without it as the project, as they're for the whole workspace
        #[tokio::test(flavor = "multi_thread")]
        async fn with_an_override_in_the_root_project() {
            let sandbox = create_sandbox("shared-root");

            overrides_version(&sandbox, ".");
            uses_toolchain(&sandbox, "packages/b");

            run_installs(&create_mocker(&sandbox), &["root", "b"]).await;

            for func in ["setup_environment", "install_dependencies"] {
                let input = get_single_input(&sandbox, func);

                assert_eq!(input["root"], "/workspace");
                assert!(input["project"].is_null());
                assert_eq!(input["toolchain_config"]["version"], "1.99.0");
                assert_eq!(input["toolchain_config"]["customKey"], ".");
            }

            // And `b` sets up the workspace's version for its tasks
            assert_eq!(
                get_setup_toolchain_versions(&sandbox),
                [
                    ("1.2.3".into(), "1.2.3".into()),
                    ("1.99.0".into(), "1.99.0".into())
                ]
            );
        }
    }

    mod package {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn without_override() {
            let sandbox = create_sandbox("shared-root");

            uses_toolchain(&sandbox, "standalone");

            run_installs(&create_mocker(&sandbox), &["standalone"]).await;

            for func in ["setup_environment", "install_dependencies"] {
                let input = get_single_input(&sandbox, func);

                assert_eq!(input["root"], "/workspace/standalone");
                assert_eq!(input["project"]["id"], "standalone");
                assert_eq!(input["toolchain_config"]["version"], "1.2.3");
            }

            assert_eq!(
                get_setup_toolchain_versions(&sandbox),
                [("1.2.3".into(), "1.2.3".into())]
            );
        }

        // The root's actions run with the project's version and config
        #[tokio::test(flavor = "multi_thread")]
        async fn with_override() {
            let sandbox = create_sandbox("shared-root");

            overrides_version(&sandbox, "standalone");

            run_installs(&create_mocker(&sandbox), &["standalone"]).await;

            assert_root_input(
                &get_single_input(&sandbox, "setup_environment"),
                "/workspace/standalone",
                Some("standalone"),
                "1.99.0",
            );
            assert_root_input(
                &get_single_input(&sandbox, "install_dependencies"),
                "/workspace/standalone",
                Some("standalone"),
                "1.99.0",
            );
            assert_eq!(
                get_setup_toolchain_versions(&sandbox),
                [("1.99.0".into(), "1.99.0".into())]
            );
        }
    }

    mod no_deps {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn without_override() {
            let sandbox = create_sandbox("shared-root");

            uses_toolchain(&sandbox, "tools");

            run_installs(&create_mocker(&sandbox), &["tools"]).await;

            assert!(read_inputs(&sandbox, "setup_environment").is_empty());
            assert!(read_inputs(&sandbox, "install_dependencies").is_empty());
            assert_eq!(
                get_setup_toolchain_versions(&sandbox),
                [("1.2.3".into(), "1.2.3".into())]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn with_override() {
            let sandbox = create_sandbox("shared-root");

            overrides_version(&sandbox, "tools");

            run_installs(&create_mocker(&sandbox), &["tools"]).await;

            assert!(read_inputs(&sandbox, "setup_environment").is_empty());
            assert!(read_inputs(&sandbox, "install_dependencies").is_empty());
            assert_eq!(
                get_setup_toolchain_versions(&sandbox),
                [("1.99.0".into(), "1.99.0".into())]
            );
        }
    }
}
