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
        let virtual_environment = if python.config.root_requirements_only {
            workspace_root.join(&python.config.venv_name)
        } else {
            working_dir.join(&python.config.venv_name)
        };

        if !virtual_environment.exists() {
            console
                .out
                .print_checkpoint(Checkpoint::Setup, "activating virtual environment")?;

            let args = vec![
                "-m",
                "venv",
                virtual_environment.to_str().unwrap_or_default(),
            ];

            operations.push(
                Operation::task_execution(format!("python {}", args.join(" ")))
                    .track_async(|| python.exec_python(args, workspace_root))
                    .await?,
            );
        }

        let mut args = vec![];

        // Add pip installArgs, if users have given
        if let Some(install_args) = &pip_config.install_args {
            args.extend(install_args.iter().map(|c| c.as_str()));
        }

        // Add requirements.txt path, if found
        if let Some(req) = &requirements_path {
            args.extend(["-r", req.to_str().unwrap_or_default()]);
        }

        if !args.is_empty() {
            args.splice(0..0, vec!["-m", "pip", "install"]);

            console
                .out
                .print_checkpoint(Checkpoint::Setup, "pip install")?;

            operations.push(
                Operation::task_execution(format!("python {}", args.join(" ")))
                    .track_async(|| python.exec_python(args, working_dir))
                    .await?,
            );
        }
    }

    Ok(operations)
}
