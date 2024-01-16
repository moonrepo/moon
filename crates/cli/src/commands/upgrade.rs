use crate::helpers::create_progress_bar;
use bytes::Buf;
use itertools::Itertools;
use miette::{miette, IntoDiagnostic};
use moon_api::Launchpad;
use moon_app_components::MoonEnv;
use moon_common::consts::BIN_NAME;
use starbase::system;
use starbase_utils::{dirs, fs};
use std::{
    env::{self, consts},
    fs::File,
    io::copy,
    path::{Component, PathBuf},
};
use tracing::error;

pub fn is_musl() -> bool {
    let Ok(output) = std::process::Command::new("ldd").arg("--version").output() else {
        return false;
    };

    String::from_utf8(output.stdout).map_or(false, |out| out.contains("musl"))
}

#[system]
pub async fn upgrade(moon_env: StateRef<MoonEnv>) {
    if proto_core::is_offline() {
        return Err(miette!("Upgrading moon requires an internet connection!"));
    }

    let remote_version = match Launchpad::check_version_without_cache(moon_env).await {
        Ok(Some(result)) if result.update_available => result.remote_version,
        Ok(_) => {
            println!("You're already on the latest version of moon!");
            return Ok(());
        }
        Err(err) => {
            error!("Failed to get current version of moon from remote: {err}");
            return Err(err);
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
    let bin_dir = match env::var("MOON_INSTALL_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => dirs::home_dir()
            .expect("Invalid home directory.")
            .join(".moon")
            .join("bin"),
    };

    // We can only upgrade moon if it's installed under .moon
    let upgradeable = current_bin_path
        .components()
        .contains(&Component::Normal(".moon".as_ref()));

    if !upgradeable {
        return Err(miette!(
            "moon can only upgrade itself when installed in the ~/.moon directory.\n\
            moon is currently installed at: {}",
            current_bin_path.to_string_lossy()
        ));
    }

    let done = create_progress_bar(format!("Upgrading moon to version {remote_version}..."));

    // Move the old binary to a versioned path
    let versioned_bin_path = bin_dir.join(&moon_env.version).join(BIN_NAME);

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

    let new_bin = reqwest::get(format!(
        "https://github.com/moonrepo/moon/releases/latest/download/{target}"
    ))
    .await
    .into_diagnostic()?
    .bytes()
    .await
    .into_diagnostic()?;

    copy(&mut new_bin.reader(), &mut file).into_diagnostic()?;

    done(
        format!("Successfully upgraded moon to version {remote_version}"),
        true,
    );
}
