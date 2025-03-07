use moon_config::{PartialPipConfig, PartialPythonConfig, PartialUvConfig, PythonPackageManager};
use moon_test_utils::{
    Sandbox, assert_snapshot, create_sandbox_with_config, get_python_fixture_configs,
    predicates::prelude::*,
};
use proto_core::UnresolvedVersionSpec;

fn python_sandbox(config: PartialPythonConfig) -> Sandbox {
    python_sandbox_with_config("python", config)
}

fn python_sandbox_with_config(fixture: &str, config: PartialPythonConfig) -> Sandbox {
    let (workspace_config, mut toolchain_config, tasks_config) = get_python_fixture_configs();

    toolchain_config.python = Some(config);

    let sandbox = create_sandbox_with_config(
        fixture,
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

mod python {
    use super::*;

    #[test]
    fn runs_standard_script() {
        let sandbox = python_sandbox(PartialPythonConfig {
            version: Some(UnresolvedVersionSpec::parse("3.11.10").unwrap()),
            ..PartialPythonConfig::default()
        });
        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("python:standard");
        });

        assert_snapshot!(assert.output());
    }

    mod pip {
        use super::*;

        #[test]
        fn runs_install_deps_via_args() {
            let sandbox = python_sandbox(PartialPythonConfig {
                version: Some(UnresolvedVersionSpec::parse("3.11.10").unwrap()),
                pip: Some(PartialPipConfig {
                    install_args: Some(vec![
                        "--quiet".to_string(),
                        "--disable-pip-version-check".to_string(),
                        "poetry==1.8.4".to_string(),
                    ]),
                }),
                ..PartialPythonConfig::default()
            });

            // Needed for venv
            sandbox.create_file("base/requirements.txt", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("python:poetry");
            });

            assert!(predicate::str::contains("Poetry (version 1.8.4)").eval(&assert.output()));
        }
    }

    mod uv {
        use super::*;

        #[test]
        fn runs_install_deps_via_args() {
            let sandbox = python_sandbox_with_config(
                "python-uv",
                PartialPythonConfig {
                    version: Some(UnresolvedVersionSpec::parse("3.11.10").unwrap()),
                    package_manager: Some(PythonPackageManager::Uv),
                    uv: Some(PartialUvConfig {
                        version: Some(UnresolvedVersionSpec::parse("0.5.26").unwrap()),
                        ..PartialUvConfig::default()
                    }),
                    ..PartialPythonConfig::default()
                },
            );

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("python:uv");
            });

            let output = assert.output();

            assert!(predicate::str::contains("uv 0.5.26").eval(&output));
            assert!(predicate::str::contains("Creating virtual environment").eval(&output));
        }
    }
}
