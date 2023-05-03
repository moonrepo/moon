use moon_action_context::ActionContext;
use moon_config::{PlatformType, RustConfig};
use moon_platform::{Platform, Runtime, Version};
use moon_project::Project;
use moon_rust_platform::RustPlatform;
use moon_task::Task;
use moon_test_utils::create_sandbox;
use moon_utils::{process::Command, string_vec};
use std::env;
use std::path::PathBuf;

fn create_platform() -> RustPlatform {
    RustPlatform::new(&RustConfig::default(), &PathBuf::new())
}

fn create_task() -> Task {
    Task {
        platform: PlatformType::Rust,
        ..Task::default()
    }
}

async fn create_target_command(task: Task) -> Command {
    let platform = create_platform();

    platform
        .create_run_target_command(
            &ActionContext::default(),
            &Project::default(),
            &task,
            &Runtime::Rust(Version::new_global()),
            &PathBuf::from("cwd"),
        )
        .await
        .unwrap()
}

mod target_command {
    use super::*;

    #[tokio::test]
    async fn uses_rustc() {
        let mut task = create_task();
        task.command = "rustc".into();
        task.args = string_vec!["-o", "test.out", "test.rs"];

        let command = create_target_command(task).await;

        assert_eq!(command.bin, "rustc");
        assert_eq!(command.args, &["-o", "test.out", "test.rs"]);
    }

    #[tokio::test]
    async fn uses_rust_others() {
        let mut task = create_task();
        task.command = "rust-analyzer".into();
        task.args = string_vec!["-o", "test.out", "test.rs"];

        let command = create_target_command(task).await;

        assert_eq!(command.bin, "rust-analyzer");
        assert_eq!(command.args, &["-o", "test.out", "test.rs"]);
    }

    #[tokio::test]
    async fn uses_cargo() {
        let mut task = create_task();
        task.command = "cargo".into();
        task.args = string_vec!["build", "-w"];

        let command = create_target_command(task).await;

        assert_eq!(command.bin, "cargo");
        assert_eq!(command.args, &["build", "-w"]);
    }

    #[tokio::test]
    async fn uses_cargo_bin() {
        let sandbox = create_sandbox("rust/project");
        sandbox.create_file("bin/cargo-nextest", "");

        let mut task = create_task();
        task.command = "nextest".into();
        task.args = string_vec!["run", "-w"];

        env::set_var("CARGO_HOME", sandbox.path());

        let command = create_target_command(task).await;

        env::remove_var("CARGO_HOME");

        assert_eq!(command.bin, "cargo");
        assert_eq!(command.args, &["nextest", "run", "-w"]);
    }

    #[tokio::test]
    async fn uses_cargo_bin_with_prefix() {
        let sandbox = create_sandbox("rust/project");
        sandbox.create_file("bin/cargo-nextest", "");

        let mut task = create_task();
        task.command = "cargo-nextest".into();
        task.args = string_vec!["run", "-w"];

        env::set_var("CARGO_HOME", sandbox.path());

        let command = create_target_command(task).await;

        env::remove_var("CARGO_HOME");

        assert_eq!(command.bin, "cargo");
        assert_eq!(command.args, &["nextest", "run", "-w"]);
    }

    #[tokio::test]
    async fn uses_global_bin() {
        let sandbox = create_sandbox("rust/project");
        sandbox.create_file("bin/sea-orm", "");

        let mut task = create_task();
        task.command = "sea-orm".into();
        task.args = string_vec!["migrate", "-u"];

        env::set_var("CARGO_HOME", sandbox.path());

        let command = create_target_command(task).await;

        env::remove_var("CARGO_HOME");

        assert_eq!(
            command.bin,
            sandbox.path().join("bin/sea-orm").to_str().unwrap()
        );
        assert_eq!(command.args, &["migrate", "-u"]);
    }
}
