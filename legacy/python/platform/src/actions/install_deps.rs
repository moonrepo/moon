use moon_action::Operation;
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

    // python.exec_python(args, working_dir)
    

    if let Some(pip_config) = &python.config.pip {

        // Very first step: Activate virtual environment
        console
            .out
            .print_checkpoint(Checkpoint::Setup, format!("activate virtual environment"))?;
        let virtual_environment = &get_workspace_root().join(python.config.venv_name.clone());
        
        if !virtual_environment.exists() {                                        
            let args = vec!["-m", "venv", virtual_environment.as_os_str().to_str().unwrap()];
            operations.push(
                Operation::task_execution(format!("python {} ", args.join(" ")))
                    .track_async(|| python.exec_python(args, working_dir))
                    .await?,
            );             
        }




        if let Some(pip_version) = &pip_config.version {
            console
                .out
                .print_checkpoint(Checkpoint::Setup, format!("install pip {pip_version}"))?;
            
            let p_version: String = if pip_version.is_latest() {
                format!("pip")
            } else {
                format!("pip{}", pip_version.to_owned().to_string().replace("~", "~="))
            };
            let args = vec!["-m", "pip", "install", "-U", &p_version];
// #"--quiet",
            operations.push(
                Operation::task_execution(format!(" {} ", args.join(" ")))
                    .track_async(|| python.exec_python(args, working_dir))
                    .await?,
            ); 
        }

        



        if let Some(req) = find_requirements_txt(working_dir, &get_workspace_root()) {
            console
                .out
                .print_checkpoint(Checkpoint::Setup, format!("pip dependencies from {}", req.as_os_str().to_str().unwrap()))?;

            let mut args = vec!["-m", "pip", "install"];
            // #, "--quiet"
            if pip_config.install_args.is_some() {
                args.extend(pip_config.install_args.as_ref().unwrap().iter().map(|c| c.as_str()));
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