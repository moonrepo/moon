use crate::app::BIN_NAME;
use crate::helpers::create_progress_bar;
use bytes::Buf;
use itertools::Itertools;
use moon_launchpad::check_version;
use moon_logger::error;
use moon_utils::semver::Version;
use proto::ProtoError;
use starbase::AppResult;
use starbase_utils::{dirs, fs};
use std::{
    env::{self, consts},
    fs::File,
    io::copy,
    path::Component,
};

pub async fn upgrade() -> AppResult {
    if proto::is_offline() {
        return Err("Upgrading moon requires an internet connection!".into());
    }

    let version = env!("CARGO_PKG_VERSION");
    let version_check = check_version(version, true).await;

    let new_version = match version_check {
        Ok(Some(newer_version))
            if Version::parse(&newer_version.current_version)? > Version::parse(version)? =>
        {
            newer_version.current_version
        }
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
            // Run ldd to check if we're running on musl
            let output = std::process::Command::new("ldd")
                .arg("--version")
                .output()?;
            let output = String::from_utf8(output.stdout)?;
            let libc = match output.contains("musl") {
                true => "musl",
                false => "gnu",
            };
            format!("moon-{arch}-unknown-linux-{libc}")
        }
        ("macos", arch) => format!("moon-{arch}-apple-darwin"),
        ("windows", "x86_64") => "moon-x86_64-pc-windows-msvc.exe".to_owned(),
        (_, arch) => {
            return Err(
                ProtoError::UnsupportedArchitecture("moon".to_owned(), arch.to_owned()).into(),
            )
        }
    };

    let current_bin_path = env::current_exe()?;
    let bin_dir = dirs::home_dir()
        .expect("Invalid home directory.")
        .join(".moon")
        .join("bin");

    // We can only upgrade moon if it's installed under .moon
    let upgradeable = current_bin_path
        .components()
        .contains(&Component::Normal(".moon".as_ref()));

    if !upgradeable {
        return Err(format!(
            "moon can only upgrade itself when installed in the ~/.moon directory.\n\
            moon is currently installed at: {}",
            current_bin_path.to_string_lossy()
        )
        .into());
    }

    let done = create_progress_bar(format!("Upgrading moon to version {new_version}..."));

    // Move the old binary to a versioned path
    let versioned_bin_path = bin_dir.join(version).join(BIN_NAME);

    fs::create_dir_all(versioned_bin_path.parent().unwrap())?;
    fs::rename(&current_bin_path, versioned_bin_path)?;

    // Download the new binary
    let bin_path = bin_dir.join(BIN_NAME);
    let mut file = File::create(bin_path)?;

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o755);
        file.set_permissions(perms)?;
    }

    let new_bin = reqwest::get(format!(
        "https://github.com/moonrepo/moon/releases/latest/download/{target}"
    ))
    .await?
    .bytes()
    .await?;

    copy(&mut new_bin.reader(), &mut file)?;

    done(
        format!("Successfully upgraded moon to version {new_version}"),
        true,
    );

    Ok(())
}
