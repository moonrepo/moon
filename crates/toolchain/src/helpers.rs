use crate::errors::ToolchainError;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use tokio::process::Command;

pub fn is_ci() -> bool {
    env::var("CI").is_ok()
}

pub async fn exec_command(bin: &Path, args: Vec<&str>, cwd: &Path) -> Result<(), ToolchainError> {
    let command_line = format!(
        "{} {}",
        bin.file_name().unwrap().to_str().unwrap(),
        args.join(" ")
    );

    println!("Running command `{}` in {}", command_line, cwd.display());

    Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .spawn()?
        .wait()
        .await?;

    // TODO: map these errors for a better response?

    // let mut child = Command::new(bin)
    // 	.args(args)
    // 	.current_dir(cwd)
    // 	.spawn()
    // 	.map_err(|error| ToolchainError::CommandFailed(command_line.clone(), error.sds))?;

    // child
    // 	.wait()
    // 	.await
    // 	.map_err(|error| ToolchainError::CommandFailed(command_line.clone()))?;

    Ok(())
}

pub async fn get_bin_version(bin: &Path) -> Result<String, ToolchainError> {
    let output = Command::new(bin)
        .args(["--version"])
        .spawn()?
        .wait_with_output()
        .await?;

    let mut version = String::from_utf8(output.stdout)
        .unwrap_or_else(|_| String::from("0.0.0"))
        .trim()
        .to_owned();

    if version.starts_with('v') {
        version = version.replace('v', "");
    }

    Ok(version)
}

pub fn get_file_sha256_hash(path: &Path) -> Result<String, ToolchainError> {
    let mut file = fs::File::open(path)?;
    let mut sha = Sha256::new();

    io::copy(&mut file, &mut sha)?;

    Ok(format!("{:?}", sha.finalize()))
}

pub async fn download_file_from_url(url: &str, dest: &Path) -> Result<(), ToolchainError> {
    // Ensure parent directories exist
    fs::create_dir_all(dest.parent().unwrap())?;

    // Fetch the file from the HTTP source
    let response = reqwest::get(url).await?;

    // Write the bytes to our local file
    let mut contents = io::Cursor::new(response.bytes().await?);
    let mut file = fs::File::create(dest)?;

    io::copy(&mut contents, &mut file)?;

    Ok(())
}
