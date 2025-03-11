use moon_test_utils::{assert_snapshot, create_sandbox, predicates::prelude::*};
use std::fs;

#[test]
fn creates_files_in_dest() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();
    let workspace_config = root.join(".moon").join("workspace.yml");
    let gitignore = root.join(".gitignore");

    assert!(!workspace_config.exists());
    assert!(!gitignore.exists());

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg("--to").arg(root);
    });

    assert
        .success()
        .code(0)
        .stdout(predicate::str::contains("Successfully initialized moon in"));

    assert!(workspace_config.exists());
    assert!(gitignore.exists());
}

#[test]
fn doesnt_create_project_config_when_minimal() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();
    let project_config = root.join(".moon").join("tasks.yml");

    assert!(!project_config.exists());

    sandbox.run_moon(|cmd| {
        cmd.arg("init")
            .arg("--yes")
            .arg("--minimal")
            .arg("--to")
            .arg(root);
    });

    assert!(!project_config.exists());
}

#[test]
fn creates_workspace_config_from_template() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();
    let workspace_config = root.join(".moon").join("workspace.yml");

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg("--to").arg(root);
    });

    assert!(
        predicate::str::contains("schemas/workspace.json")
            .eval(&fs::read_to_string(workspace_config).unwrap())
    );
}

// #[test]
// fn creates_project_config_from_template() {
//     let sandbox = create_sandbox("init-sandbox");
//     let root = sandbox.path().to_path_buf();
//     let project_config = root
//         .join(".moon")
//         .join(moon_constants::CONFIG_TASKS_FILENAME);

//     sandbox.run_moon(|cmd| {
//         cmd.arg("init").arg("--yes").arg("--to").arg(root);
//     });

//     assert!(
//         predicate::str::contains("https://moonrepo.dev/schemas/tasks.json")
//             .eval(&fs::read_to_string(project_config).unwrap())
//     );
// }

#[test]
fn creates_gitignore_file() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();
    let gitignore = root.join(".gitignore");

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg("--to").arg(root);
    });

    assert_eq!(
        fs::read_to_string(gitignore).unwrap(),
        "\n# moon\n.moon/cache\n.moon/docker\n"
    );
}

#[test]
fn appends_existing_gitignore_file() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();

    sandbox.create_file(".gitignore", "*.js\n*.log");

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg("--to").arg(&root);
    });

    assert_eq!(
        fs::read_to_string(root.join(".gitignore")).unwrap(),
        "*.js\n*.log\n# moon\n.moon/cache\n.moon/docker\n"
    );
}

#[test]
fn does_overwrite_existing_config_if_force_passed() {
    let sandbox = create_sandbox("init-sandbox");
    let root = sandbox.path().to_path_buf();

    sandbox.run_moon(|cmd| {
        cmd.arg("init").arg("--yes").arg("--to").arg(&root);
    });

    // Run again
    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("init")
            .arg("--yes")
            .arg("--to")
            .arg(root)
            .arg("--force");
    });

    assert
        .success()
        .code(0)
        .stdout(predicate::str::contains("Successfully initialized moon in"));
}

mod vcs {
    use super::*;

    #[test]
    fn detects_git() {
        let sandbox = create_sandbox("init-sandbox");
        sandbox.enable_git();

        let root = sandbox.path().to_path_buf();
        let workspace_config = root.join(".moon").join("workspace.yml");

        // Checkout a new branch
        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "sandboxs-test"]);
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("init").arg("--yes").arg("--to").arg(root);
        });

        let content = fs::read_to_string(workspace_config).unwrap();

        // We don't show the vcs block if it defaults to git/master
        assert!(!predicate::str::contains("manager: 'git'").eval(&content));
    }
}

mod init_toolchain {
    use super::*;

    #[test]
    fn errors_for_missing_locator() {
        let sandbox = create_sandbox("init-sandbox");
        let root = sandbox.path().to_path_buf();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("init").arg("tc").arg("--yes").arg("--to").arg(root);
        });

        assert!(
            predicate::str::contains(
                "A plugin locator is required as the 2nd argument when initializing a toolchain!"
            )
            .eval(&assert.output())
        );
    }

    #[test]
    fn errors_for_invalid_locator() {
        let sandbox = create_sandbox("init-sandbox");
        let root = sandbox.path().to_path_buf();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("init")
                .arg("tc")
                .arg("invalid")
                .arg("--yes")
                .arg("--to")
                .arg(root);
        });

        assert!(predicate::str::contains("Missing plugin protocol").eval(&assert.output()));
    }

    #[test]
    fn renders_full() {
        let sandbox = create_sandbox("init-sandbox");
        let root = sandbox.path().to_path_buf();
        let config = root.join(".moon").join("toolchain.yml");

        sandbox.run_moon(|cmd| {
            cmd.arg("init")
                .arg("typescript")
                .arg("--yes")
                .arg("--to")
                .arg(root);
        });

        assert_snapshot!(fs::read_to_string(config).unwrap());
    }

    #[test]
    fn renders_minimal() {
        let sandbox = create_sandbox("init-sandbox");
        let root = sandbox.path().to_path_buf();
        let config = root.join(".moon").join("toolchain.yml");

        sandbox.run_moon(|cmd| {
            cmd.arg("init")
                .arg("typescript")
                .arg("--yes")
                .arg("--minimal")
                .arg("--to")
                .arg(root);
        });

        assert_snapshot!(fs::read_to_string(config).unwrap());
    }
}
