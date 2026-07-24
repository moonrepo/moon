use crate::app_error::AppError;
use crate::helpers::create_progress_loader;
use crate::session::{MoonSession, SessionResult};
use clap::Args;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_api::Launchpad;
use moon_common::path;
use moon_config::PartialWorkspaceConfig;
use moon_config_loader::{read_config_based_on_extension, write_config_based_on_extension};
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_env_var::GlobalEnvBag;
use moon_process::Command;
use starbase_archive::Archiver;
use starbase_utils::fs::FsError;
use starbase_utils::{fs, net};
use std::env::{self, consts};
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};
use version_spec::{Requirement, Version};

pub fn is_musl() -> bool {
    match std::process::Command::new("ldd").arg("--version").output() {
        Ok(output) => String::from_utf8(output.stdout).is_ok_and(|out| out.contains("musl")),
        Err(_) => false,
    }
}

pub enum InstalledWith {
    Moon,
    Proto,
    Unknown(PathBuf),
}

pub fn is_installed_with(session: &MoonSession) -> miette::Result<InstalledWith> {
    let current_exe_path = env::current_exe().into_diagnostic()?;

    if current_exe_path.starts_with(&session.proto_env.store.dir) {
        return Ok(InstalledWith::Proto);
    }

    if current_exe_path.starts_with(&session.moon_env.store_root) {
        return Ok(InstalledWith::Moon);
    }

    Ok(InstalledWith::Unknown(current_exe_path))
}

#[derive(Args, Clone, Debug)]
pub struct UpgradeArgs {
    #[arg(
        long,
        help = "Update the version constraint in the workspace configuration"
    )]
    update_constraint: bool,
}

#[instrument(skip(session))]
pub async fn upgrade(session: MoonSession, args: UpgradeArgs) -> SessionResult {
    if proto_core::is_offline() {
        return Err(AppError::UpgradeRequiresInternet.into());
    }

    let remote_version = match Launchpad::instance()
        .unwrap()
        .check_version_without_cache(&session.toolchains_config.moon.manifest_url)
        .await
    {
        Ok(Some(result)) if result.update_available => result.remote_version,
        Ok(_) => {
            session.console.render(element! {
                Container {
                    Notice(variant: Variant::Info) {
                        StyledText(content: "You're already on the latest version of moon!")
                    }
                }
            })?;

            return Ok(None);
        }
        Err(error) => {
            return Err(error);
        }
    };

    let target = match (consts::OS, consts::ARCH) {
        ("linux", arch) => {
            format!(
                "moon_cli-{arch}-unknown-linux-{}",
                if is_musl() { "musl" } else { "gnu" }
            )
        }
        ("macos", arch) => format!("moon_cli-{arch}-apple-darwin"),
        ("windows", arch) => format!("moon_cli-{arch}-pc-windows-msvc"),
        (os, arch) => {
            return Err(miette::miette!(
                "Unsupported os ({os}) + architecture ({arch})"
            ));
        }
    };
    let filename = if consts::OS == "windows" {
        format!("{target}.zip")
    } else {
        format!("{target}.tar.xz")
    };

    let bin_dir = match GlobalEnvBag::instance().get("MOON_INSTALL_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => session.moon_env.store_root.join("bin"),
    };

    match is_installed_with(&session)? {
        // We can only upgrade moon if it's installed under .moon
        InstalledWith::Unknown(current_exe_path) => {
            session.console.render_err(element! {
                Container {
                    Notice(variant: Variant::Caution) {
                        StyledText(content: "moon can only upgrade itself when installed in the <path>~/.moon</path> directory.")
                        StyledText(content: format!("moon is currently installed at <path>{}</path>!", current_exe_path.display()))
                    }
                }
            })?;

            return Ok(Some(1));
        }

        // Special case to install with proto
        InstalledWith::Proto => {
            Command::new("proto")
                .args(["install", "moon", "latest", "--pin", "local"])
                .exec_stream_output()
                .await?;
        }

        // Otherwise, we can upgrade moon normally
        InstalledWith::Moon => {
            let progress = create_progress_loader(
                session.get_console()?,
                format!("Upgrading moon to version {remote_version}..."),
            )
            .await;

            // Download the archive
            let download_url = session
                .toolchains_config
                .moon
                .download_url
                .replace("{file}", &filename)
                .replace("{version}", &remote_version.to_string());
            let archive_file = session.moon_env.temp_dir.join(&filename);

            debug!(
                source_url = &download_url,
                dest_file = ?archive_file,
                target = target,
                "Downloading archive"
            );

            net::download_from_url(&download_url, &archive_file).await?;

            // Unpack the archive
            let unpacked_dir = session.moon_env.temp_dir.join(&target);

            debug!(
                archive_file = ?archive_file,
                unpacked_dir = ?unpacked_dir,
                target = target,
                "Unpacking archive"
            );

            let mut archiver = Archiver::new(&unpacked_dir, &archive_file);
            archiver.set_prefix(&target);
            archiver.unpack_from_ext()?;

            // Move executables
            for exe_name in [path::exe_name("moon"), path::exe_name("moonx")] {
                let input_path = unpacked_dir.join(&exe_name);
                let output_path = bin_dir.join(&exe_name);
                let relocate_path = bin_dir.join(format!("{exe_name}.backup"));

                if output_path.exists() {
                    self_replace(&output_path, &input_path, &relocate_path)?;
                } else {
                    fs::copy_file(&input_path, &output_path)?;
                    fs::update_perms(&output_path, None)?;
                }
            }

            if args.update_constraint {
                update_constraint(&session, &remote_version)?;
            }

            // Cleanup
            fs::remove(&unpacked_dir)?;
            fs::remove(&archive_file)?;

            progress.stop().await?;
        }
    };

    if args.update_constraint {
        update_constraint(&session, &remote_version)?;
    }

    // If we have an active daemon connection, we should stop it so
    // the new version can be used next time moon runs
    if let Some(mut client) = session.get_daemon_connector()?.connect_once().await? {
        client.stop().await?;
    }

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Info) {
                StyledText(content: format!("Upgraded moon to version {remote_version}!"))
            }
        }
    })?;

    Ok(None)
}

#[cfg(unix)]
fn self_replace(
    current_exe: &Path,
    replace_with: &Path,
    relocate_to: &Path,
) -> Result<(), FsError> {
    use std::os::unix::fs::PermissionsExt;

    // If we're a symlink, we need to find the real location and operate on
    // that instead of the link.
    let exe = current_exe.canonicalize().map_err(|error| FsError::Read {
        path: current_exe.to_path_buf(),
        error: Box::new(error),
    })?;
    let perms = fs::metadata(&exe)?.permissions();

    // Relocate the current executable. We do a rename/move as it keeps the
    // same inode's, just changes the literal path. This allows the binary
    // to keep executing without failure. A copy will *not* work!
    fs::rename(exe, relocate_to)?;

    // We then copy the replacement executable to the original location,
    // and attempt to persist the original permissions.
    fs::copy_file(replace_with, current_exe)?;
    fs::update_perms(current_exe, Some(perms.mode()))?;

    Ok(())
}

#[cfg(windows)]
fn self_replace(
    current_exe: &Path,
    replace_with: &Path,
    relocate_to: &Path,
) -> Result<(), FsError> {
    // If we're a symlink, we need to find the real location and operate on
    // that instead of the link.
    let exe = current_exe.canonicalize().map_err(|error| FsError::Read {
        path: current_exe.to_path_buf(),
        error: Box::new(error),
    })?;

    // Relocate the current executable. We do a rename/move as it keeps the
    // same ID/handle, just changes the literal path. This allows the binary
    // to keep executing without failure. A copy will *not* work!
    fs::rename(exe, relocate_to)?;

    // We then copy the replacement executable to a temporary location.
    let mut temp_exe = current_exe.to_path_buf();
    temp_exe.set_extension("temp.exe");

    fs::copy_file(replace_with, &temp_exe)?;

    // And lastly, we move the temporary to the original location. This avoids
    // writing/copying data to the original, and instead does a rename/move.
    fs::rename(temp_exe, current_exe)?;

    Ok(())
}

fn update_constraint(session: &MoonSession, version: &Version) -> miette::Result<()> {
    for file in session.config_loader.get_workspace_files() {
        if !file.exists() {
            continue;
        }

        let mut config: PartialWorkspaceConfig = read_config_based_on_extension(&file)?;

        if let Ok(req) = Requirement::parse(format!("^{version}")) {
            config.version_constraint = Some(req);

            write_config_based_on_extension(&file, config)?;
        }

        break;
    }

    Ok(())
}
