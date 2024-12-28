use moon_config::{PartialPipConfig, PartialPythonConfig};
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_python_fixture_configs, Sandbox,
};
use proto_core::UnresolvedVersionSpec;

fn python_sandbox(config: PartialPythonConfig) -> Sandbox {
    python_sandbox_with_config(|_| {}, config)
}

fn python_sandbox_with_config<C>(callback: C, config: PartialPythonConfig) -> Sandbox
where
    C: FnOnce(&mut PartialPythonConfig),
{
    let (workspace_config, mut toolchain_config, tasks_config) = get_python_fixture_configs();

    toolchain_config.python = Some(config);

    if let Some(python_config) = &mut toolchain_config.python {
        callback(python_config);
    }

    let sandbox = create_sandbox_with_config(
        "python",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

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

    assert.debug();

    assert_snapshot!(assert.output());
}
