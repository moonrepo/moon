//! PROTOTYPE: User-scoped activation, integrity, and fallback policy.

use crate::plugin::{VcsPlugin, load_verified_prototype_plugin};
use miette::{IntoDiagnostic, miette};
use minisign_verify::{PublicKey, Signature};
use moon_pdk_api::{DetectVcsInput, DetectVcsOutput, MoonContext};
use moon_plugin::{MoonEnvironment, PluginLocator};
use serde::{Deserialize, Serialize};
use starbase_utils::hash;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const USER_POLICY_FILE: &str = "vcs.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PrototypeVcsUserPolicy {
    pub enabled: bool,
    pub plugin: PluginLocator,
    pub sha256: String,
}

pub enum PrototypeVcsSelection {
    Git {
        reason: String,
    },
    Overlay {
        plugin: Arc<VcsPlugin>,
        context: MoonContext,
        detection: DetectVcsOutput,
    },
}

impl PrototypeVcsSelection {
    fn summary(&self, policy_file: PathBuf) -> PrototypeVcsActivationSummary {
        match self {
            Self::Git { reason } => PrototypeVcsActivationSummary {
                adapter: "git",
                policy_file,
                reason: reason.clone(),
            },
            Self::Overlay { detection, .. } => PrototypeVcsActivationSummary {
                adapter: "jj WASM overlay",
                policy_file,
                reason: detection.reason.clone(),
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct PrototypeVcsActivationSummary {
    adapter: &'static str,
    policy_file: PathBuf,
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrototypeVcsPolicyUpdateSummary {
    action: &'static str,
    enabled: bool,
    plugin: PluginLocator,
    policy_file: PathBuf,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SignedPrototypeVcsPolicy {
    plugin: PluginLocator,
    sha256: String,
}

pub fn get_user_policy_path(environment: &MoonEnvironment) -> PathBuf {
    environment.store_root.join(USER_POLICY_FILE)
}

pub fn load_user_policy(path: &Path) -> miette::Result<Option<PrototypeVcsUserPolicy>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path).into_diagnostic()?;
    let mut policy = serde_json::from_str::<PrototypeVcsUserPolicy>(&content).into_diagnostic()?;
    validate_policy(&mut policy)?;

    Ok(Some(policy))
}

pub fn write_user_policy(path: &Path, policy: &PrototypeVcsUserPolicy) -> miette::Result<()> {
    let mut policy = policy.clone();
    validate_policy(&mut policy)?;
    let parent = path
        .parent()
        .ok_or_else(|| miette!("user VCS policy path has no parent directory"))?;
    fs::create_dir_all(parent).into_diagnostic()?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .into_diagnostic()?
        .as_nanos();
    let temp_file = parent.join(format!(
        ".{USER_POLICY_FILE}.tmp-{}-{nonce}",
        std::process::id()
    ));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);

    #[cfg(unix)]
    options.mode(0o600);

    let result = (|| -> miette::Result<()> {
        let mut file = options.open(&temp_file).into_diagnostic()?;
        let mut content = serde_json::to_string_pretty(&policy).into_diagnostic()?;
        content.push('\n');
        file.write_all(content.as_bytes()).into_diagnostic()?;
        file.sync_all().into_diagnostic()?;
        replace_policy_file(&temp_file, path).into_diagnostic()?;

        Ok(())
    })();

    if result.is_err() && temp_file.exists() {
        let _ = fs::remove_file(temp_file);
    }

    result
}

#[cfg(not(windows))]
fn replace_policy_file(temp_file: &Path, policy_file: &Path) -> std::io::Result<()> {
    fs::rename(temp_file, policy_file)
}

#[cfg(windows)]
fn replace_policy_file(temp_file: &Path, policy_file: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVE_FILE_FLAGS, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let temp_file = temp_file
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let policy_file = policy_file
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let flags: MOVE_FILE_FLAGS = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;

    // SAFETY: Both paths are valid, NUL-terminated UTF-16 buffers that remain
    // alive for the duration of the call.
    if unsafe { MoveFileExW(temp_file.as_ptr(), policy_file.as_ptr(), flags) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub async fn activate(
    policy: Option<&PrototypeVcsUserPolicy>,
    repository_root: &Path,
) -> miette::Result<PrototypeVcsSelection> {
    let Some(policy) = policy else {
        return Ok(PrototypeVcsSelection::Git {
            reason: "no user VCS overlay policy is configured".into(),
        });
    };

    if !policy.enabled {
        return Ok(PrototypeVcsSelection::Git {
            reason: "the user VCS overlay policy is disabled".into(),
        });
    }

    let mut policy = policy.clone();
    validate_policy(&mut policy)?;
    let plugin =
        load_verified_prototype_plugin(repository_root, policy.plugin.clone(), &policy.sha256)
            .await?;
    let context = MoonContext {
        working_dir: plugin.to_virtual_path(repository_root),
        workspace_root: plugin.to_virtual_path(repository_root),
    };
    let detection = plugin
        .detect(DetectVcsInput {
            context: context.clone(),
        })
        .await?;

    if detection.active {
        Ok(PrototypeVcsSelection::Overlay {
            plugin,
            context,
            detection,
        })
    } else {
        Ok(PrototypeVcsSelection::Git {
            reason: detection.reason,
        })
    }
}

pub async fn run_user_policy(repository_root: &Path) -> miette::Result<()> {
    let environment = MoonEnvironment::new()?;
    let policy_file = get_user_policy_path(&environment);
    let policy = load_user_policy(&policy_file)?;
    let selection = activate(policy.as_ref(), repository_root).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&selection.summary(policy_file)).into_diagnostic()?
    );

    Ok(())
}

pub fn install_user_policy(locator: &str, sha256: &str) -> miette::Result<()> {
    let environment = MoonEnvironment::new()?;
    let policy_file = get_user_policy_path(&environment);
    let policy = PrototypeVcsUserPolicy {
        enabled: true,
        plugin: PluginLocator::from_str(locator).into_diagnostic()?,
        sha256: sha256.into(),
    };

    write_user_policy(&policy_file, &policy)?;
    print_update_summary("installed", policy_file, policy)
}

pub fn install_local_user_policy(plugin_file: &Path) -> miette::Result<()> {
    let plugin_file = plugin_file.canonicalize().into_diagnostic()?;
    let sha256 = hash::sha256::from_file(&plugin_file)?;
    let locator = format!("file://{}", plugin_file.display());

    install_user_policy(&locator, &sha256)
}

pub fn install_signed_user_policy(
    manifest_file: &Path,
    signature_file: &Path,
    public_key_file: &Path,
) -> miette::Result<()> {
    let manifest = fs::read(manifest_file).into_diagnostic()?;
    let signature = Signature::from_file(signature_file).into_diagnostic()?;
    let public_key = PublicKey::from_file(public_key_file).into_diagnostic()?;
    verify_signed_manifest(&manifest, &signature, &public_key)?;

    let signed = serde_json::from_slice::<SignedPrototypeVcsPolicy>(&manifest).into_diagnostic()?;
    let environment = MoonEnvironment::new()?;
    let policy_file = get_user_policy_path(&environment);
    let policy = PrototypeVcsUserPolicy {
        enabled: true,
        plugin: signed.plugin,
        sha256: signed.sha256,
    };
    write_user_policy(&policy_file, &policy)?;

    print_update_summary("installed-signed", policy_file, policy)
}

pub fn set_user_policy_enabled(enabled: bool) -> miette::Result<()> {
    let environment = MoonEnvironment::new()?;
    let policy_file = get_user_policy_path(&environment);
    let mut policy = load_user_policy(&policy_file)?
        .ok_or_else(|| miette!("no user VCS policy exists at {}", policy_file.display()))?;
    policy.enabled = enabled;
    write_user_policy(&policy_file, &policy)?;

    print_update_summary(
        if enabled { "enabled" } else { "disabled" },
        policy_file,
        policy,
    )
}

fn print_update_summary(
    action: &'static str,
    policy_file: PathBuf,
    policy: PrototypeVcsUserPolicy,
) -> miette::Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&PrototypeVcsPolicyUpdateSummary {
            action,
            enabled: policy.enabled,
            plugin: policy.plugin,
            policy_file,
            sha256: policy.sha256,
        })
        .into_diagnostic()?
    );

    Ok(())
}

fn verify_signed_manifest(
    manifest: &[u8],
    signature: &Signature,
    public_key: &PublicKey,
) -> miette::Result<()> {
    public_key
        .verify(manifest, signature, false)
        .into_diagnostic()
}

fn validate_and_resolve_locator(locator: &mut PluginLocator) -> miette::Result<()> {
    match locator {
        PluginLocator::File(file) => {
            let path = file.get_unresolved_path();

            if !path.is_absolute() {
                return Err(miette!(
                    "user VCS plugin file locators must use an absolute path"
                ));
            }

            file.path = Some(path);
        }
        PluginLocator::Url(url) if !url.url.starts_with("https://") => {
            return Err(miette!("user VCS plugin URLs must use HTTPS"));
        }
        _ => {}
    }

    Ok(())
}

fn validate_policy(policy: &mut PrototypeVcsUserPolicy) -> miette::Result<()> {
    if policy.sha256.len() != 64
        || !policy
            .sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(miette!(
            "trusted plugin SHA-256 must contain exactly 64 hexadecimal characters"
        ));
    }

    validate_and_resolve_locator(&mut policy.plugin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use warpgate::FileLocator;

    fn create_temp_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "moon-vcs-policy-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn create_policy(plugin_file: &Path) -> PrototypeVcsUserPolicy {
        PrototypeVcsUserPolicy {
            enabled: true,
            plugin: PluginLocator::File(Box::new(FileLocator {
                file: format!("file://{}", plugin_file.display()),
                path: Some(plugin_file.to_path_buf()),
            })),
            sha256: "a".repeat(64),
        }
    }

    #[test]
    fn atomically_writes_and_replaces_policy() {
        let root = create_temp_dir();
        let policy_file = root.join(USER_POLICY_FILE);
        let plugin_file = root.join("plugin.wasm");
        let mut policy = create_policy(&plugin_file);

        write_user_policy(&policy_file, &policy).unwrap();
        policy.enabled = false;
        write_user_policy(&policy_file, &policy).unwrap();

        assert_eq!(load_user_policy(&policy_file).unwrap(), Some(policy));
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_relative_file_locator_and_invalid_digest() {
        let root = create_temp_dir();
        let policy_file = root.join(USER_POLICY_FILE);
        let mut policy = create_policy(Path::new("relative.wasm"));

        assert!(write_user_policy(&policy_file, &policy).is_err());
        policy.sha256 = "invalid".into();
        assert!(write_user_policy(&policy_file, &policy).is_err());
        assert!(!policy_file.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifies_signed_provenance_and_rejects_tampering() {
        let public_key =
            PublicKey::from_base64("RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3")
                .unwrap();
        let signature = Signature::decode(
            "untrusted comment: signature from minisign secret key\n\
             RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\n\
             trusted comment: timestamp:1556193335\tfile:test\n\
             y/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==",
        )
        .unwrap();

        assert!(verify_signed_manifest(b"test", &signature, &public_key).is_ok());
        assert!(verify_signed_manifest(b"tampered", &signature, &public_key).is_err());
    }
}
