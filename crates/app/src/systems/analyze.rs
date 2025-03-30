use crate::app_error::AppError;
use moon_actions::utils::should_skip_action;
use moon_bun_platform::BunPlatform;
use moon_cache::CacheEngine;
use moon_common::{consts::PROTO_CLI_VERSION, is_test_env, path::exe_name};
use moon_config::{BunConfig, PlatformType, ToolchainConfig};
use moon_console::{Checkpoint, Console};
use moon_deno_platform::DenoPlatform;
use moon_env_var::GlobalEnvBag;
use moon_node_platform::NodePlatform;
use moon_platform::PlatformManager;
use moon_python_platform::PythonPlatform;
use moon_rust_platform::RustPlatform;
use moon_system_platform::SystemPlatform;
use moon_vcs::BoxedVcs;
use proto_core::{ProtoEnvError, ProtoEnvironment, is_offline};
use proto_installer::*;
use semver::{Version, VersionReq};
use starbase::AppResult;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn extract_repo_info(vcs: &BoxedVcs) -> miette::Result<()> {
    let bag = GlobalEnvBag::instance();

    if vcs.is_enabled() && !bag.has("MOON_VCS_REPO_SLUG") {
        if let Ok(slug) = vcs.get_repository_slug().await {
            bag.set("MOON_VCS_REPO_SLUG", slug.as_str());
        }
    }

    Ok(())
}

#[instrument]
pub fn validate_version_constraint(constraint: &VersionReq, version: &Version) -> AppResult {
    if !constraint.matches(version) {
        return Err(AppError::InvalidMoonVersion {
            actual: version.to_string(),
            expected: constraint.to_string(),
        }
        .into());
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn install_proto(
    console: &Console,
    proto_env: &Arc<ProtoEnvironment>,
    cache_engine: &CacheEngine,
    toolchain_config: &ToolchainConfig,
) -> AppResult {
    let _lock = cache_engine.create_lock("proto-install")?;

    let bin_name = exe_name("proto");
    let install_dir = proto_env
        .store
        .inventory_dir
        .join("proto")
        .join(PROTO_CLI_VERSION);

    debug!(proto = ?install_dir.join(&bin_name), "Checking if proto is installed");

    // Set the version so that proto lookup paths take it into account
    let bag = GlobalEnvBag::instance();
    bag.set("PROTO_VERSION", PROTO_CLI_VERSION);
    bag.set("PROTO_IGNORE_MIGRATE_WARNING", "true");
    bag.set("PROTO_VERSION_CHECK", "false");
    bag.set("PROTO_LOOKUP_DIR", &install_dir);

    // This causes a ton of issues when running the test suite,
    // so just avoid it and assume proto exists!
    if install_dir.exists() || is_test_env() || !toolchain_config.should_install_proto() {
        return Ok(None);
    }

    debug!("Installing proto");

    console.print_checkpoint(
        Checkpoint::Setup,
        format!("installing proto {}", PROTO_CLI_VERSION),
    )?;

    // If offline but a primary proto binary exists,
    // use that instead of failing, even if a different version!
    if is_offline() {
        let existing_bin = proto_env.store.bin_dir.join(&bin_name);

        if existing_bin.exists() {
            debug!(
                proto = ?existing_bin,
                "No internet connection, but using existing {} binary",
                bin_name
            );

            return Ok(None);
        } else {
            return Err(ProtoEnvError::RequiredInternetConnection.into());
        }
    }

    let target_triple = determine_triple()?;

    debug!("Downloading proto archive ({})", target_triple);

    let result = download_release(
        &target_triple,
        PROTO_CLI_VERSION,
        &proto_env.store.temp_dir,
        |_, _| {},
    )
    .await?;

    debug!("Unpacking archive and installing proto");

    install_release(result, &install_dir, &proto_env.store.temp_dir, false)?;

    debug!("Successfully installed proto!");

    Ok(None)
}

#[instrument(skip_all)]
pub async fn register_platforms(
    console: &Console,
    proto_env: &Arc<ProtoEnvironment>,
    toolchain_config: &ToolchainConfig,
    workspace_root: &Path,
) -> AppResult {
    let console = Arc::new(console.to_owned());
    let registry = PlatformManager::write();

    debug!(
        platforms = ?toolchain_config.get_enabled_platforms(),
        "Registering platforms based on toolchain configuration",
    );

    // Primarily for testing
    registry.reset();

    if let Some(bun_config) = &toolchain_config.bun {
        registry.register(
            PlatformType::Bun.get_toolchain_id(),
            Box::new(BunPlatform::new(
                bun_config,
                workspace_root,
                Arc::clone(proto_env),
                Arc::clone(&console),
            )),
        );
    }

    if let Some(deno_config) = &toolchain_config.deno {
        registry.register(
            PlatformType::Deno.get_toolchain_id(),
            Box::new(DenoPlatform::new(
                deno_config,
                workspace_root,
                Arc::clone(proto_env),
                Arc::clone(&console),
            )),
        );
    }

    if let Some(node_config) = &toolchain_config.node {
        registry.register(
            PlatformType::Node.get_toolchain_id(),
            Box::new(NodePlatform::new(
                node_config,
                workspace_root,
                Arc::clone(proto_env),
                Arc::clone(&console),
            )),
        );

        // TODO fix in 2.0
        if toolchain_config.bun.is_none() {
            if let Some(bunpm_config) = &node_config.bun {
                let bun_config = BunConfig {
                    plugin: bunpm_config.plugin.clone(),
                    version: bunpm_config.version.clone(),
                    ..Default::default()
                };

                registry.register(
                    PlatformType::Bun.get_toolchain_id(),
                    Box::new(BunPlatform::new(
                        &bun_config,
                        workspace_root,
                        Arc::clone(proto_env),
                        Arc::clone(&console),
                    )),
                );
            }
        }
    }

    if let Some(python_config) = &toolchain_config.python {
        registry.register(
            PlatformType::Python.get_toolchain_id(),
            Box::new(PythonPlatform::new(
                python_config,
                workspace_root,
                Arc::clone(proto_env),
                Arc::clone(&console),
            )),
        );
    }

    if let Some(rust_config) = &toolchain_config.rust {
        registry.register(
            PlatformType::Rust.get_toolchain_id(),
            Box::new(RustPlatform::new(
                rust_config,
                workspace_root,
                Arc::clone(proto_env),
                Arc::clone(&console),
            )),
        );
    }

    // Should be last since it's the most common
    registry.register(
        PlatformType::System.get_toolchain_id(),
        Box::new(SystemPlatform::new(
            workspace_root,
            Arc::clone(proto_env),
            Arc::clone(&console),
        )),
    );

    Ok(None)
}

#[instrument]
pub async fn load_toolchain() -> AppResult {
    // This isn't an action but we should also support skipping here!
    if should_skip_action("MOON_SKIP_SETUP_TOOLCHAIN").is_some() {
        return Ok(None);
    }

    for platform in PlatformManager::write().list_mut() {
        platform.setup_toolchain().await?;
    }

    Ok(None)
}
