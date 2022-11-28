use crate::cli::{
    create_moon_command, get_assert_stderr_output, get_assert_stdout_output, output_to_string,
};
use crate::get_fixtures_dir;
use assert_cmd::assert::Assert;
use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use moon_config::{GlobalProjectConfig, ToolchainConfig, WorkspaceConfig};
use std::path::Path;
use std::process::Command as StdCommand;

pub struct Sandbox {
    assert: Option<Assert>,
    command: Option<Command>,
    pub fixture: TempDir,
}

impl Sandbox {
    pub fn path(&self) -> &Path {
        self.fixture.path()
    }

    pub fn create_file<T: AsRef<str>>(&self, name: &str, content: T) -> &Self {
        self.fixture
            .child(name)
            .write_str(content.as_ref())
            .unwrap();

        self
    }

    pub fn debug(&self) -> &Self {
        let assert = self.assert
            .as_ref()
            .expect("Debugging the sandbox requires a `moon` command to be ran with `run_moon()`. If you only want to debug files, use `debug_files()` instead.");

        // List all files in the sandbox
        println!("sandbox:");
        debug_sandbox_files(self.path());
        println!("\n");

        // Debug outputs
        println!("stdout:\n{}\n", get_assert_stdout_output(assert));
        println!("stderr:\n{}\n", get_assert_stderr_output(assert));
        println!("status: {:#?}", assert.get_output().status);

        self
    }

    pub fn debug_files(&self) -> &Self {
        debug_sandbox_files(self.path());

        self
    }

    pub fn enable_git(&self) -> &Self {
        if !self.path().join(".gitignore").exists() {
            self.create_file(".gitignore", "node_modules");
        }

        // Initialize a git repo so that VCS commands work
        self.run_git(|cmd| {
            cmd.args(["init", "--initial-branch", "master"]);
        });

        // We must also add the files to the index
        self.run_git(|cmd| {
            cmd.args(["add", "--all", "."]);
        });

        // And commit them... this seems like a lot of overhead?
        self.run_git(|cmd| {
            cmd.args(["commit", "-m", "Fixtures"])
                .env("GIT_AUTHOR_NAME", "moon tests")
                .env("GIT_AUTHOR_EMAIL", "fakeemail@moonrepo.dev")
                .env("GIT_COMMITTER_NAME", "moon tests")
                .env("GIT_COMMITTER_EMAIL", "fakeemail@moonrepo.dev");
        });

        self
    }

    pub fn run_git<C>(&self, handler: C) -> &Self
    where
        C: FnOnce(&mut StdCommand),
    {
        let mut cmd = StdCommand::new(if cfg!(windows) { "git.exe" } else { "git" });
        cmd.current_dir(self.path());

        handler(&mut cmd);

        let out = cmd.output().unwrap_or_else(|e| {
            panic!("{:#?}", e);
        });

        if !out.status.success() {
            println!("{}", output_to_string(&out.stdout));
            eprintln!("{}", output_to_string(&out.stderr));
        }

        self
    }

    pub fn run_moon<C>(&mut self, handler: C) -> &Self
    where
        C: FnOnce(&mut Command),
    {
        let mut cmd = create_moon_command(self.path());

        handler(&mut cmd);

        self.assert = Some(cmd.assert());
        self.command = Some(cmd);
        self
    }
}

pub fn create_sandbox<T: AsRef<str>>(fixture: T) -> Sandbox {
    let temp_dir = TempDir::new().unwrap();

    temp_dir
        .copy_from(get_fixtures_dir(fixture), &["**/*"])
        .unwrap();

    Sandbox {
        assert: None,
        command: None,
        fixture: temp_dir,
    }
}

pub fn create_sandbox_with_config<T: AsRef<str>>(
    fixture: T,
    workspace_config: Option<&WorkspaceConfig>,
    toolchain_config: Option<&ToolchainConfig>,
    projects_config: Option<&GlobalProjectConfig>,
) -> Sandbox {
    let sandbox = create_sandbox(fixture);

    sandbox.create_file(
        ".moon/workspace.yml",
        serde_yaml::to_string(
            &workspace_config
                .map(|c| c.to_owned())
                .unwrap_or_else(|| WorkspaceConfig::default()),
        )
        .unwrap(),
    );

    sandbox.create_file(
        ".moon/toolchain.yml",
        serde_yaml::to_string(
            &toolchain_config
                .map(|c| c.to_owned())
                .unwrap_or_else(|| ToolchainConfig::default()),
        )
        .unwrap(),
    );

    if let Some(config) = projects_config {
        sandbox.create_file(".moon/project.yml", serde_yaml::to_string(&config).unwrap());
    }

    sandbox
}

fn debug_sandbox_files(dir: &Path) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();

        if path.is_dir() {
            debug_sandbox_files(&path);
        } else {
            println!("- {}", path.to_string_lossy());
        }
    }
}
