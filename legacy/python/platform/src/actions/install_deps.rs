use moon_action::Operation;
use moon_console::{Checkpoint, Console};
use moon_python_tool::{find_requirements_txt, PythonTool};
use std::path::Path;

pub async fn install_deps(
    python: &PythonTool,
    workspace_root: &Path,
    working_dir: &Path,
    console: &Console,
) -> miette::Result<Vec<Operation>> {
    let mut operations = vec![];
    let requirements_path = find_requirements_txt(working_dir, workspace_root);

    let venv_root = if python.config.root_requirements_only {
        workspace_root.join(&python.config.venv_name)
    } else {
        requirements_path
            .as_ref()
            .and_then(|rp| rp.parent())
            .unwrap_or(working_dir)
            .join(&python.config.venv_name)
    };

    if !venv_root.exists() {
        console
            .out
            .print_checkpoint(Checkpoint::Setup, "python venv")?;

        let args = vec!["-m", "venv", venv_root.to_str().unwrap_or_default()];

        operations.push(
            Operation::task_execution(format!("python {}", args.join(" ")))
                .track_async(|| python.exec_python(args, working_dir, workspace_root))
                .await?,
        );
    }

    if let Some(pip_config) = &python.config.pip {
        let mut args = vec![];

        // Add pip installArgs, if users have given
        if let Some(install_args) = &pip_config.install_args {
            args.extend(install_args.iter().map(|c| c.as_str()));
        }

        // Add requirements.txt path, if found
        if let Some(reqs_path) = requirements_path.as_ref().and_then(|req| req.to_str()) {
            args.extend(["-r", reqs_path]);
        }

        if !args.is_empty() {
            args.splice(0..0, vec!["-m", "pip", "install"]);

            console
                .out
                .print_checkpoint(Checkpoint::Setup, "pip install")?;

            operations.push(
                Operation::task_execution(format!("python {}", args.join(" ")))
                    .track_async(|| python.exec_python(args, working_dir, workspace_root))
                    .await?,
            );
        }
    }

    Ok(operations)
}
