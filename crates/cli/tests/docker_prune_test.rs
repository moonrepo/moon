use moon_test_utils::{MoonSandbox, create_moon_sandbox, predicates};
use starbase_utils::fs;

mod docker_prune {
    use super::*;

    fn create_workdir(sandbox: &MoonSandbox) -> std::path::PathBuf {
        sandbox
            .run_bin(|cmd| {
                cmd.arg("docker").arg("scaffold").arg("prune");
            })
            .success();

        fs::copy_dir_all(
            sandbox.path().join(".moon/docker/configs"),
            sandbox.path().join("work"),
        )
        .unwrap();

        fs::copy_dir_all(
            sandbox.path().join(".moon/docker/sources"),
            sandbox.path().join("work"),
        )
        .unwrap();

        sandbox.path().join("work")
    }

    #[test]
    fn errors_for_missing_manifest() {
        let sandbox = create_moon_sandbox("docker");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("docker").arg("prune");
            })
            .failure()
            .stderr(predicates::str::contains(
                "Unable to continue, docker manifest missing.",
            ));
    }

    #[test]
    fn removes_vendor_dirs() {
        let sandbox = create_moon_sandbox("docker");
        let workdir = create_workdir(&sandbox);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("docker").arg("prune").current_dir(&workdir);
            })
            .success();

        assert!(!workdir.join("prune/vendor").exists());
    }
}

// The `docker-layout` fixture is a dependencies workspace at the root (with
// `packages/*` members, where `a` depends on `b`), a `standalone` package root
// that overrides the version and a setting, and `tools`, which isn't a member.
// The `tc-tier2` test plugin records the inputs of every prune and install.
mod docker_prune_layout {
    use super::*;
    use starbase_utils::json::serde_json::{self, Value};
    use std::path::{Path, PathBuf};

    fn scaffold_and_prune(sandbox: &MoonSandbox, ids: &[&str]) -> PathBuf {
        sandbox
            .run_bin(|cmd| {
                cmd.arg("docker").arg("scaffold").args(ids);
            })
            .success();

        let workdir = sandbox.path().join("work");

        fs::copy_dir_all(sandbox.path().join(".moon/docker/configs"), &workdir).unwrap();
        fs::copy_dir_all(sandbox.path().join(".moon/docker/sources"), &workdir).unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("docker").arg("prune").current_dir(&workdir);
            })
            .success();

        workdir
    }

    fn read_inputs(workdir: &Path, func: &str) -> Vec<Value> {
        let Ok(entries) =
            std::fs::read_dir(workdir.join(".moon/cache/tcInputs/tc-tier2").join(func))
        else {
            return vec![];
        };

        entries
            .map(|entry| {
                serde_json::from_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap())
                    .unwrap()
            })
            .collect()
    }

    fn get_single_input(workdir: &Path, func: &str) -> Value {
        let mut inputs = read_inputs(workdir, func);

        assert_eq!(inputs.len(), 1, "Expected one {func} input");

        inputs.remove(0)
    }

    fn get_ids(projects: &Value) -> Vec<&str> {
        projects
            .as_array()
            .unwrap()
            .iter()
            .map(|project| project["id"].as_str().unwrap())
            .collect()
    }

    // A dependencies workspace's actions use the workspace's config, and no
    // project, while its unfocused members are passed as dependencies
    #[test]
    fn prunes_a_workspace_with_its_config() {
        let sandbox = create_moon_sandbox("docker-layout");
        let workdir = scaffold_and_prune(&sandbox, &["a"]);

        let prune = get_single_input(&workdir, "prune_docker");
        let install = get_single_input(&workdir, "install_dependencies");

        assert_eq!(prune["root"], "/workspace");
        assert_eq!(get_ids(&prune["projects"]), ["a"]);
        assert_eq!(get_ids(&prune["project_dependencies"]), ["b"]);
        assert!(prune["toolchain_config"].get("customKey").is_none());

        assert_eq!(install["root"], "/workspace");
        assert_eq!(install["production"], true);
        assert!(install["project"].is_null());
        assert!(install["toolchain_config"].get("customKey").is_none());
    }

    // With a root project, a dependencies workspace's actions use its config,
    // still without a project, whichever projects are focused
    #[test]
    fn prunes_a_workspace_with_the_root_project_config() {
        for focused in [&["root"][..], &["a"]] {
            let sandbox = create_moon_sandbox("docker-layout");

            sandbox.create_file(
                "moon.yml",
                "toolchains:\n  default: 'tc-tier2'\n  tc-tier2:\n    customKey: 'root'\n",
            );

            let workdir = scaffold_and_prune(&sandbox, focused);

            let prune = get_single_input(&workdir, "prune_docker");
            let install = get_single_input(&workdir, "install_dependencies");

            assert_eq!(get_ids(&prune["projects"]), focused, "{focused:?}");
            assert_eq!(
                prune["toolchain_config"]["customKey"], "root",
                "{focused:?}"
            );
            assert!(install["project"].is_null(), "{focused:?}");
            assert_eq!(
                install["toolchain_config"]["customKey"], "root",
                "{focused:?}"
            );
        }
    }

    // A package root's actions use its owner project's config, and project
    #[test]
    fn prunes_a_package_with_the_project_config() {
        let sandbox = create_moon_sandbox("docker-layout");
        let workdir = scaffold_and_prune(&sandbox, &["standalone"]);

        let prune = get_single_input(&workdir, "prune_docker");
        let install = get_single_input(&workdir, "install_dependencies");

        assert_eq!(prune["root"], "/workspace/standalone");
        assert_eq!(get_ids(&prune["projects"]), ["standalone"]);
        assert_eq!(get_ids(&prune["project_dependencies"]), Vec::<&str>::new());
        assert_eq!(prune["toolchain_config"]["customKey"], "standalone");
        assert_eq!(prune["toolchain_config"]["version"], "2.0.0");

        assert_eq!(install["root"], "/workspace/standalone");
        assert_eq!(install["project"]["id"], "standalone");
        assert_eq!(install["toolchain_config"]["customKey"], "standalone");
    }

    #[test]
    fn prunes_each_root_of_the_focused_projects() {
        let sandbox = create_moon_sandbox("docker-layout");
        let workdir = scaffold_and_prune(&sandbox, &["a", "standalone"]);

        let mut roots = read_inputs(&workdir, "prune_docker")
            .into_iter()
            .map(|input| input["root"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        roots.sort();

        assert_eq!(roots, ["/workspace", "/workspace/standalone"]);
        assert_eq!(read_inputs(&workdir, "install_dependencies").len(), 2);
    }

    // Not a member of the workspace that it located
    #[test]
    fn doesnt_prune_a_project_outside_of_a_root() {
        let sandbox = create_moon_sandbox("docker-layout");
        let workdir = scaffold_and_prune(&sandbox, &["tools"]);

        assert!(read_inputs(&workdir, "prune_docker").is_empty());
        assert!(read_inputs(&workdir, "install_dependencies").is_empty());
    }

    #[test]
    fn doesnt_prune_a_disabled_toolchain() {
        let sandbox = create_moon_sandbox("docker-layout");
        let workdir = scaffold_and_prune(&sandbox, &["off"]);

        assert!(read_inputs(&workdir, "prune_docker").is_empty());
        assert!(read_inputs(&workdir, "install_dependencies").is_empty());
    }

    #[test]
    fn doesnt_install_if_disabled() {
        let sandbox = create_moon_sandbox("docker-layout");

        sandbox.update_workspace_config(|config| {
            config.docker = Some(moon_config::PartialDockerConfig {
                prune: Some(moon_config::PartialDockerPruneConfig {
                    install_toolchain_dependencies: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            });
        });

        let workdir = scaffold_and_prune(&sandbox, &["a"]);

        assert_eq!(read_inputs(&workdir, "prune_docker").len(), 1);
        assert!(read_inputs(&workdir, "install_dependencies").is_empty());
    }
}
