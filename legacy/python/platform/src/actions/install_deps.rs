use moon_action::Operation;
use moon_console::{Checkpoint, Console};
use moon_python_tool::PythonTool;
use std::path::Path;

use crate::find_requirements_txt;

pub async fn install_deps(
    python: &PythonTool,
    workspace_root: &Path,
    working_dir: &Path,
    console: &Console,
) -> miette::Result<Vec<Operation>> {
    let mut operations = vec![];

    if let Some(pip_config) = &python.config.pip {
        let requirements_path = find_requirements_txt(working_dir, workspace_root);
        let virtual_environment = &working_dir.join(python.config.venv_name.clone());

        if !virtual_environment.exists() {
            console
                .out
                .print_checkpoint(Checkpoint::Setup, "activate virtual environment")?;
            let args = vec![
                "-m",
                "venv",
                virtual_environment.as_os_str().to_str().unwrap(),
            ];
            operations.push(
                Operation::task_execution(format!("python {} ", args.join(" ")))
                    .track_async(|| python.exec_python(args, workspace_root))
                    .await?,
            );
        }

        if let Some(install_args) = &pip_config.install_args {
            if install_args.iter().any(|x| !x.starts_with("-")) && requirements_path.is_none() {
                console
                    .out
                    .print_checkpoint(Checkpoint::Setup, "pip install")?;

                let mut args = vec!["-m", "pip", "install"];
                if let Some(install_args) = &pip_config.install_args {
                    args.extend(install_args.iter().map(|c| c.as_str()));
                }

                operations.push(
                    Operation::task_execution(format!("python {}", args.join(" ")))
                        .track_async(|| python.exec_python(args, working_dir))
                        .await?,
                );
            }
        }

        if let Some(req) = requirements_path {
            console
                .out
                .print_checkpoint(Checkpoint::Setup, "pip install")?;

            let mut args = vec!["-m", "pip", "install"];
            if let Some(install_args) = &pip_config.install_args {
                args.extend(install_args.iter().map(|c| c.as_str()));
            }

            args.extend(["-r", req.as_os_str().to_str().unwrap()]);

            operations.push(
                Operation::task_execution(format!("python {}", args.join(" ")))
                    .track_async(|| python.exec_python(args, working_dir))
                    .await?,
            );
        }
    }

    Ok(operations)
}
