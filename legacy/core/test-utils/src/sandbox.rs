use crate::cli::{SandboxAssert, create_moon_command, output_to_string};
use crate::get_fixtures_path;
use assert_cmd::Command;
pub use assert_fs::TempDir;
use assert_fs::prelude::*;
use moon_config::{PartialInheritedTasksConfig, PartialToolchainConfig, PartialWorkspaceConfig};
use starbase_utils::glob;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

pub struct Sandbox {
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

    pub fn debug_configs(&self) -> &Self {
        for cfg in glob::walk_files(self.path(), [".moon/**/*.pkl", ".moon/**/*.yml"]).unwrap() {
            if cfg.exists() {
                println!("{:?}:\n{}", &cfg, fs::read_to_string(&cfg).unwrap());
            }
        }

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
            cmd.args(["commit", "-m", "Fixtures"]);
        });

        // Add another commit so we can do HEAD~1 tests
        self.run_git(|cmd| {
            cmd.args(["commit", "-m", "Bump", "--allow-empty"]);
        });

        self
    }

    pub fn run_git<C>(&self, handler: C) -> &Self
    where
        C: FnOnce(&mut StdCommand),
    {
        let mut cmd = StdCommand::new(if cfg!(windows) { "git.exe" } else { "git" });
        cmd.current_dir(self.path())
            .env("GIT_AUTHOR_NAME", "moon tests")
            .env("GIT_AUTHOR_EMAIL", "fakeemail@moonrepo.dev")
            .env("GIT_COMMITTER_NAME", "moon tests")
            .env("GIT_COMMITTER_EMAIL", "fakeemail@moonrepo.dev");

        handler(&mut cmd);

        let out = cmd.output().unwrap_or_else(|e| {
            panic!("{e:#?}");
        });

        if !out.status.success() {
            println!("{}", output_to_string(&out.stdout));
            eprintln!("{}", output_to_string(&out.stderr));
        }

        self
    }

    pub fn run_moon<C>(&self, handler: C) -> SandboxAssert
    where
        C: FnOnce(&mut Command),
    {
        let mut cmd = create_moon_command(self.path());

        handler(&mut cmd);

        SandboxAssert {
            inner: cmd.assert(),
            sandbox: self,
        }
    }
}

pub fn create_temp_dir() -> TempDir {
    TempDir::new().unwrap()
}

pub fn create_sandbox<T: AsRef<str>>(fixture: T) -> Sandbox {
    let temp_dir = create_temp_dir();

    temp_dir
        .copy_from(get_fixtures_path(fixture), &["**/*"])
        .unwrap();

    Sandbox {
        // command: None,
        fixture: temp_dir,
    }
}

pub fn create_sandbox_with_config<T: AsRef<str>>(
    fixture: T,
    workspace_config: Option<PartialWorkspaceConfig>,
    toolchain_config: Option<PartialToolchainConfig>,
    tasks_config: Option<PartialInheritedTasksConfig>,
) -> Sandbox {
    let sandbox = create_sandbox(fixture);

    sandbox.create_file(
        ".moon/workspace.yml",
        serde_yml::to_string(&workspace_config.unwrap_or_default()).unwrap(),
    );

    sandbox.create_file(
        ".moon/toolchain.yml",
        serde_yml::to_string(&toolchain_config.unwrap_or_default()).unwrap(),
    );

    if let Some(config) = tasks_config {
        sandbox.create_file(".moon/tasks.yml", serde_yml::to_string(&config).unwrap());
    }

    sandbox
}

pub fn create_sandbox_with_factory<
    T: AsRef<str>,
    F: FnOnce(
        &mut PartialWorkspaceConfig,
        &mut PartialToolchainConfig,
        &mut PartialInheritedTasksConfig,
    ),
>(
    fixture: T,
    factory: F,
) -> Sandbox {
    let mut workspace_config = PartialWorkspaceConfig::default();
    let mut toolchain_config = PartialToolchainConfig::default();
    let mut tasks_config = PartialInheritedTasksConfig::default();

    factory(
        &mut workspace_config,
        &mut toolchain_config,
        &mut tasks_config,
    );

    create_sandbox_with_config(
        fixture,
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    )
}

pub fn debug_sandbox_files(dir: &Path) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();

        if path.is_dir() {
            debug_sandbox_files(&path);
        } else {
            println!("- {}", path.to_string_lossy());
        }
    }
}
