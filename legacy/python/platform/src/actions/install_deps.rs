use moon_action::Operation;
use moon_common::color;
use moon_console::{Checkpoint, Console};
use moon_python_tool::PythonTool;
use moon_utils::get_workspace_root;
use std::path::Path;

use crate::find_requirements_txt;

pub async fn install_deps(
    python: &PythonTool,
    working_dir: &Path,
    console: &Console,
) -> miette::Result<Vec<Operation>> {
    let mut operations = vec![];

    if let Some(pip_config) = &python.config.pip {
        let requirements_path = find_requirements_txt(working_dir, &get_workspace_root());

        if let Some(install_args) = &pip_config.install_args {
            if install_args.iter().any(|x| !x.starts_with("-")) && requirements_path.is_none() {
                console.out.print_checkpoint(
                    Checkpoint::Setup,
                    "Install pip dependencies from install args",
                )?;

                let mut args = vec!["-m", "pip", "install", "--quiet"];
                if pip_config.install_args.is_some() {
                    args.extend(
                        pip_config
                            .install_args
                            .as_ref()
                            .unwrap()
                            .iter()
                            .map(|c| c.as_str()),
                    );
                }

                operations.push(
                    Operation::task_execution(format!(" {}", args.join(" ")))
                        .track_async(|| python.exec_python(args, working_dir))
                        .await?,
                );
            }
        }

        if let Some(req) = requirements_path {
            console.out.print_checkpoint(
                Checkpoint::Setup,
                format!("Install pip dependencies from {}", color::path(&req)),
            )?;

            let mut args = vec!["-m", "pip", "install", "--quiet"];
            if pip_config.install_args.is_some() {
                args.extend(
                    pip_config
                        .install_args
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|c| c.as_str()),
                );
            }

            args.extend(["-r", req.as_os_str().to_str().unwrap()]);

            operations.push(
                Operation::task_execution(format!(" {}", args.join(" ")))
                    .track_async(|| python.exec_python(args, working_dir))
                    .await?,
            );
        }
    }

    Ok(operations)
}
