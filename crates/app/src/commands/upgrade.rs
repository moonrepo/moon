use crate::app_error::AppError;
use crate::components::create_progress_loader;
use crate::session::MoonSession;
use bytes::Buf;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_api::Launchpad;
use moon_common::consts::BIN_NAME;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_env_var::GlobalEnvBag;
use starbase::AppResult;
use starbase_utils::fs;
use std::{
    env::{self, consts},
    fs::File,
    io::copy,
    path::{Component, PathBuf},
};
use tracing::{debug, instrument};

pub fn is_musl() -> bool {
    let Ok(output) = std::process::Command::new("ldd").arg("--version").output() else {
        return false;
    };

    String::from_utf8(output.stdout).is_ok_and(|out| out.contains("musl"))
}

#[instrument(skip_all)]
pub async fn upgrade(session: MoonSession) -> AppResult {
    if proto_core::is_offline() {
        return Err(AppError::UpgradeRequiresInternet.into());
    }

    let remote_version = match Launchpad::check_version_without_cache(
        &session.moon_env,
        &session.toolchain_config.moon.manifest_url,
    )
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
                "moon-{arch}-unknown-linux-{}",
                if is_musl() { "musl" } else { "gnu" }
            )
        }
        ("macos", arch) => format!("moon-{arch}-apple-darwin"),
        ("windows", "x86_64") => "moon-x86_64-pc-windows-msvc.exe".to_owned(),
        (_, arch) => return Err(miette::miette!("Unsupported architecture: {arch}")),
    };

    let current_bin_path = env::current_exe().into_diagnostic()?;
    let bin_dir = match GlobalEnvBag::instance().get("MOON_INSTALL_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => session.moon_env.store_root.join("bin"),
    };

    // We can only upgrade moon if it's installed under .moon
    let upgradeable = current_bin_path
        .components()
        .any(|comp| comp == Component::Normal(".moon".as_ref()));

    if !upgradeable {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "moon can only upgrade itself when installed in the <path>~/.moon</path> directory.")
                    StyledText(content: format!("moon is currently installed at <path>{}</path>", current_bin_path.display()))
                }
            }
        })?;

        return Ok(Some(1));
    }

    let progress = create_progress_loader(
        session.get_console()?,
        format!("Upgrading moon to version {remote_version}..."),
    );

    // Move the old binary to a versioned path
    let versioned_bin_path = bin_dir.join(session.cli_version.to_string()).join(BIN_NAME);

    fs::create_dir_all(versioned_bin_path.parent().unwrap())?;
    fs::rename(&current_bin_path, versioned_bin_path)?;

    // Download the new binary
    let bin_path = bin_dir.join(BIN_NAME);
    let mut file = File::create(bin_path).into_diagnostic()?;

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata().into_diagnostic()?.permissions();
        perms.set_mode(0o755);
        file.set_permissions(perms).into_diagnostic()?;
    }

    let download_url = &session.toolchain_config.moon.download_url;

    debug!(
        download_url = &download_url,
        target = target,
        "Download new version of moon"
    );

    let new_bin = reqwest::get(format!(
        "{download_url}{}{target}",
        if download_url.ends_with('/') { "" } else { "/" }
    ))
    .await
    .into_diagnostic()?
    .bytes()
    .await
    .into_diagnostic()?;

    copy(&mut new_bin.reader(), &mut file).into_diagnostic()?;

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
