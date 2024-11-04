use moon_action::Operation;
use moon_python_tool::PythonTool;
use starbase_utils::fs;
use std::path::Path;

pub async fn setup_tool(python: &PythonTool, workspace_root: &Path) -> miette::Result<()> {
    let mut operations = vec![];

    if let Some(pip_config) = &python.config.pip {
        let virtual_environment = &workspace_root.join(python.config.venv_name.clone());

        if !virtual_environment.exists() {
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

        if let Some(pip_version) = &pip_config.version {
            let p_version: String = if pip_version.is_latest() {
                "pip".to_string()
            } else {
                format!(
                    "pip{}",
                    pip_version.to_owned().to_string().replace("~", "~=")
                )
            };
            let args = vec!["-m", "pip", "install", "--quiet", "-U", &p_version];
            operations.push(
                Operation::task_execution(format!(" {} ", args.join(" ")))
                    .track_async(|| python.exec_python(args, workspace_root))
                    .await?,
            );
        }
    }

    // Create version file
    if let Some(python_version) = &python.config.version {
        let rc_path = workspace_root.join(".python-version");
        fs::write_file(&rc_path, python_version.to_string())?;
    }

    Ok(())
}
