use crate::app_error::AppError;
use moon_actions::utils::should_skip_action;
use moon_bun_platform::BunPlatform;
use moon_config::{BunConfig, PlatformType, ToolchainConfig};
use moon_console::Console;
use moon_deno_platform::DenoPlatform;
use moon_env_var::GlobalEnvBag;
use moon_node_platform::NodePlatform;
use moon_pdk_api::SetupToolchainInput;
use moon_platform::PlatformManager;
use moon_python_platform::PythonPlatform;
use moon_rust_platform::RustPlatform;
use moon_system_platform::SystemPlatform;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::BoxedVcs;
use proto_core::ProtoEnvironment;
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
pub async fn load_toolchain(
    toolchain_registry: &ToolchainRegistry,
    toolchain_config: &ToolchainConfig,
) -> AppResult {
    // This isn't an action but we should also support skipping here!
    if should_skip_action("MOON_SKIP_SETUP_TOOLCHAIN").is_some() {
        return Ok(None);
    }

    for platform in PlatformManager::write().list_mut() {
        platform.setup_toolchain().await?;
    }

    toolchain_registry
        .setup_toolchain_all(|registry, toolchain| SetupToolchainInput {
            configured_version: toolchain_config
                .plugins
                .get(toolchain.id.as_str())
                .and_then(|plugin| plugin.version.clone()),
            context: registry.create_context(),
            toolchain_config: registry.create_config(&toolchain.id, toolchain_config),
            version: None,
        })
        .await?;

    Ok(None)
}
