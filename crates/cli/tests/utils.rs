#![allow(dead_code)]

use moon_common::{Id, is_ci};
use moon_config::PartialToolchainPluginConfig;
use moon_test_utils2::{MoonSandbox, create_empty_moon_sandbox, create_moon_sandbox};
use proto_core::UnresolvedVersionSpec;

pub fn create_projects_sandbox() -> MoonSandbox {
    let sandbox = create_moon_sandbox("projects");
    sandbox.with_default_projects();
    sandbox
}

pub fn create_tasks_sandbox() -> MoonSandbox {
    let sandbox = create_moon_sandbox("tasks");
    sandbox.with_default_projects();
    sandbox.update_toolchains_config(|config| {
        let plugins = config.plugins.get_or_insert_default();
        plugins.insert(
            Id::raw("javascript"),
            PartialToolchainPluginConfig::default(),
        );
        plugins.insert(
            Id::raw("node"),
            PartialToolchainPluginConfig {
                version: Some(UnresolvedVersionSpec::parse("24.0.0").unwrap()),
                ..Default::default()
            },
        );
    });
    sandbox
}

pub fn create_pipeline_sandbox() -> MoonSandbox {
    let sandbox = create_moon_sandbox("pipeline");
    sandbox.with_default_projects();
    sandbox.enable_git();
    sandbox
}

pub fn create_sync_heavy_pipeline_sandbox(depth: usize, width: usize) -> MoonSandbox {
    let sandbox = create_empty_moon_sandbox();
    sandbox.with_default_projects();

    let first_layer = (0..width)
        .map(|node| format!("layer0-node{node}"))
        .collect::<Vec<_>>();

    sandbox.create_file(
        "app/moon.yml",
        format!(
            "{}\n{}",
            format_depends_on(&first_layer),
            r#"tasks:
  noop:
    command: 'exit 0'
    options:
      cache: true
"#
        ),
    );

    for layer in 0..depth {
        let next_layer = if layer + 1 < depth {
            Some(
                (0..width)
                    .map(|node| format!("layer{}-node{node}", layer + 1))
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };

        for node in 0..width {
            let project_id = format!("layer{layer}-node{node}");
            let contents = next_layer
                .as_ref()
                .map(|deps| format_depends_on(deps))
                .unwrap_or_else(|| "tasks: {}\n".into());

            sandbox.create_file(format!("{project_id}/moon.yml"), contents);
        }
    }

    sandbox
}

pub fn create_query_sandbox() -> MoonSandbox {
    let sandbox = create_projects_sandbox();
    sandbox.enable_git();
    sandbox
}

fn format_depends_on(deps: &[String]) -> String {
    let mut contents = String::from("dependsOn:\n");

    for dep in deps {
        contents.push_str(&format!("  - {dep}\n"));
    }

    contents
}

pub fn change_branch<T: AsRef<str>>(sandbox: &MoonSandbox, branch: T) {
    sandbox.run_git(|cmd| {
        cmd.args(["checkout", "-b", branch.as_ref()]);
    });
}

pub fn change_files<I: IntoIterator<Item = V>, V: AsRef<str>>(sandbox: &MoonSandbox, files: I) {
    let files = files
        .into_iter()
        .map(|file| file.as_ref().to_string())
        .collect::<Vec<_>>();

    for file in &files {
        sandbox.create_file(file, "contents");
    }

    // CI uses `git diff` while local uses `git status`
    if is_ci() {
        change_branch(sandbox, "branch");

        sandbox.run_git(|cmd| {
            cmd.arg("add").args(files);
        });

        sandbox.run_git(|cmd| {
            cmd.args(["commit", "-m", "Change"]);
        });
    }
}
