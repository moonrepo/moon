use moon_action::Operation;
use moon_common::is_test_env;
use moon_config::PythonPackageManager;
use moon_console::{Checkpoint, Console};
use moon_logger::error;
use moon_python_tool::PythonTool;
use std::path::Path;

pub async fn install_deps(
    python: &PythonTool,
    workspace_root: &Path,
    working_dir: &Path,
    console: &Console,
) -> miette::Result<Vec<Operation>> {
    let mut operations = vec![];
    let venv_parent = python.find_venv_root(working_dir, workspace_root);

    let venv_root = if python.config.root_venv_only {
        workspace_root.join(&python.config.venv_name)
    } else {
        venv_parent
            .as_ref()
            .and_then(|vp| vp.parent())
            .unwrap_or(working_dir)
            .join(&python.config.venv_name)
    };

    if !venv_root.exists() && venv_parent.is_some() {
        let command = match python.config.package_manager {
            PythonPackageManager::Pip => "python -m venv",
            PythonPackageManager::Uv => "uv venv",
        };

        operations.push(
            Operation::task_execution(command)
                .track_async(|| async {
                    console.out.print_checkpoint(Checkpoint::Setup, command)?;

                    python
                        .exec_venv(&venv_root, working_dir, workspace_root)
                        .await
                })
                .await?,
        );
    }

    let package_manager = python.get_package_manager();

    // Install dependencies
    {
        let command = match python.config.package_manager {
            PythonPackageManager::Pip => "pip install",
            PythonPackageManager::Uv => "uv sync",
        };

        for attempt in 1..=3 {
            if attempt == 1 {
                console.out.print_checkpoint(Checkpoint::Setup, command)?;
            } else {
                console.out.print_checkpoint_with_comments(
                    Checkpoint::Setup,
                    command,
                    [format!("attempt {attempt} of 3")],
                )?;
            }

            let mut op = Operation::task_execution(command);
            let result = Operation::do_track_async(&mut op, || {
                package_manager.install_dependencies(python, working_dir, !is_test_env())
            })
            .await;

            operations.push(op);

            if let Err(error) = result {
                if attempt == 3 {
                    return Err(error);
                } else {
                    error!(
                        "Failed to install {} dependencies, retrying...",
                        python.config.package_manager
                    );
                }
            } else {
                break;
            }
        }
    }

    Ok(operations)
}
