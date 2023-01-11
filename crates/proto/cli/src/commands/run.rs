use crate::config::{Config, CONFIG_NAME};
use log::{debug, trace};
use proto::{color, create_tool, get_tools_dir, load_version_file, ProtoError, ToolType};
use std::{env, path::Path, process::exit};
use tokio::process::Command;

pub async fn run(tool_type: ToolType, args: Vec<String>) -> Result<(), ProtoError> {
    let mut tool = create_tool(&tool_type)?;
    let mut version = None;

    // Env var takes highest priority
    let env_var = format!("PROTO_{}_VERSION", tool.get_id().to_uppercase());

    if let Ok(session_version) = env::var(&env_var) {
        debug!(
            target: "proto:run",
            "Detected version {} from environment variable {}",
            session_version,
            env_var
        );

        version = Some(session_version);
    }

    // Traverse upwards and attempt to detect a local version
    if let Ok(working_dir) = env::current_dir() {
        trace!(
            target: "proto:run",
            "Attempting to find local version"
        );

        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = &current_dir {
            trace!(
                target: "proto:run",
                "Checking in directory {}",
                color::path(dir)
            );

            // We already found a version, so exit
            if version.is_some() {
                break;
            }

            // Detect from our config file
            trace!(
                target: "proto:run",
                "Checking proto configuration file"
            );

            let config_file = dir.join(CONFIG_NAME);

            if config_file.exists() {
                let config = Config::load(&config_file)?;

                if let Some(config_version) = config.tools.get(&tool_type) {
                    debug!(
                        target: "proto:run",
                        "Detected version {} from configuration file {}",
                        config_version,
                        color::path(&config_file)
                    );

                    version = Some(config_version.to_owned());
                    break;
                }
            }

            // Detect using the tool
            trace!(
                target: "proto:run",
                "Detecting from the tool's ecosystem"
            );

            if let Some(eco_version) = tool.detect_version_from(dir).await? {
                debug!(
                    target: "proto:run",
                    "Detected version {} from tool's ecosystem",
                    eco_version,
                );

                version = Some(eco_version);
                break;
            }

            current_dir = dir.parent();
        }
    }

    // If still no version, load the global version
    if version.is_none() {
        trace!(
            target: "proto:run",
            "Attempting to find global version"
        );

        let global_file = get_tools_dir()?.join(tool.get_id()).join("version");

        if global_file.exists() {
            let global_version = load_version_file(&global_file)?;

            debug!(
                target: "proto:run",
                "Detected global version {} from {}",
                global_version,
                color::path(&global_file)
            );

            version = Some(global_version);
        }
    }

    // We didn't find anything!
    let Some(version) = version else {
        return Err(ProtoError::Message(
            "Unable to detect an applicable version. Try setting a local or global version."
                .into(),
        ));
    };

    // Does the tool exist?
    if !tool.is_setup(&version).await? {
        return Err(ProtoError::MissingTool(tool.get_name()));
    }

    let status = Command::new(tool.get_bin_path()?)
        .args(&args)
        .env(env_var, tool.get_resolved_version())
        .spawn()
        .map_err(|e| ProtoError::Message(e.to_string()))?
        .wait()
        .await
        .map_err(|e| ProtoError::Message(e.to_string()))?;

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }

    Ok(())
}
