use crate::app_error::AppError;
use crate::helpers::create_progress_loader;
use crate::session::MoonSession;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_api::Launchpad;
use moon_common::path;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_env_var::GlobalEnvBag;
use starbase::AppResult;
use starbase_archive::Archiver;
use starbase_utils::{fs, net};
use std::{
    env::{self, consts},
    path::{Component, PathBuf},
};
use tracing::{debug, instrument};

pub fn is_musl() -> bool {
    match std::process::Command::new("ldd").arg("--version").output() {
        Ok(output) => String::from_utf8(output.stdout).is_ok_and(|out| out.contains("musl")),
        Err(_) => false,
    }
}

#[instrument(skip(session))]
pub async fn upgrade(session: MoonSession) -> AppResult {
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
                "{arch}-unknown-linux-{}",
                if is_musl() { "musl" } else { "gnu" }
            )
        }
        ("macos", arch) => format!("{arch}-apple-darwin"),
        ("windows", arch) => format!("{arch}-pc-windows-msvc"),
        (os, arch) => {
            return Err(miette::miette!(
                "Unsupported os ({os}) + architecture ({arch})"
            ));
        }
    };
    let filename = if consts::OS == "windows" {
        format!("moon_cli-{target}.zip")
    } else {
        format!("moon_cli-{target}.tar.xz")
    };

    let current_bin_path = env::current_exe().into_diagnostic()?;
    let bin_dir = match GlobalEnvBag::instance().get("MOON_INSTALL_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => session.moon_env.store_root.join("bin"),
    };
    let versioned_bin_dir = bin_dir.join(session.cli_version.to_string());

    // We can only upgrade moon if it's installed under .moon
    let upgradeable = current_bin_path
        .components()
        .any(|comp| comp == Component::Normal(".moon".as_ref()));

    if !upgradeable {
        session.console.render_err(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "moon can only upgrade itself when installed in the <path>~/.moon</path> directory.")
                    StyledText(content: format!("moon is currently installed at <path>{}</path>!", current_bin_path.display()))
                }
            }
        })?;

        return Ok(Some(1));
    }

    let progress = create_progress_loader(
        session.get_console()?,
        format!("Upgrading moon to version {remote_version}..."),
    )
    .await;

    // Move the old executable to a versioned path
    let exe_names = vec![path::exe_name("moon"), path::exe_name("moonx")];
    let current_bin_dir = current_bin_path.parent().unwrap();

    fs::create_dir_all(&versioned_bin_dir)?;

    for exe_name in &exe_names {
        let exe_path = current_bin_dir.join(exe_name);

        if exe_path.exists() {
            fs::rename(exe_path, versioned_bin_dir.join(exe_name))?;
        }
    }

    // Download the archive
    let download_url = session
        .toolchains_config
        .moon
        .download_url
        .replace("{file}", &filename);
    let archive_file = session.moon_env.temp_dir.join(&filename);

    debug!(
        source_url = &download_url,
        dest_file = ?archive_file,
        target = target,
        "Download archive"
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

    Archiver::new(&unpacked_dir, &archive_file).unpack_from_ext()?;

    // Move executables
    for exe_name in exe_names {
        let input_path = unpacked_dir.join(&exe_name);
        let output_path = bin_dir.join(exe_name);

        if input_path.exists() {
            fs::copy_file(&input_path, &output_path)?;
            fs::update_perms(&output_path, None)?;
        }
    }

    progress.stop().await?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Info) {
                StyledText(content: format!("Upgraded moon to version {remote_version}!"))
            }
        }
    })?;

    Ok(None)
}
