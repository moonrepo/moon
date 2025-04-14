use moon_bun_platform::BunPlatform;
use moon_config::{BunConfig, ConfigLoader, PlatformType, ToolchainConfig};
use moon_console::{Console, MoonReporter};
use moon_deno_platform::DenoPlatform;
use moon_node_platform::NodePlatform;
use moon_platform::PlatformManager;
use moon_python_platform::PythonPlatform;
use moon_rust_platform::RustPlatform;
use moon_system_platform::SystemPlatform;
use proto_core::{ProtoConfig, ProtoEnvironment};
use std::path::Path;
use std::sync::Arc;

fn create_test_console() -> Console {
    let mut console = Console::new_testing();
    console.set_reporter(MoonReporter::default());
    console
}

pub async fn generate_platform_manager_from_sandbox(root: &Path) -> PlatformManager {
    let proto = Arc::new(ProtoEnvironment::new_testing(root).unwrap());
    let console = Arc::new(create_test_console());
    let config = ConfigLoader::default()
        .load_toolchain_config(root, &ProtoConfig::default())
        .unwrap();

    generate_platform_manager(root, &config, proto, console).await
}

pub async fn generate_platform_manager(
    root: &Path,
    config: &ToolchainConfig,
    proto: Arc<ProtoEnvironment>,
    console: Arc<Console>,
) -> PlatformManager {
    let mut manager = PlatformManager::default();

    if let Some(bun_config) = &config.bun {
        manager.register(
            PlatformType::Bun.get_toolchain_id(),
            Box::new(BunPlatform::new(
                bun_config,
                root,
                proto.clone(),
                console.clone(),
            )),
        );
    }

    if let Some(deno_config) = &config.deno {
        manager.register(
            PlatformType::Deno.get_toolchain_id(),
            Box::new(DenoPlatform::new(
                deno_config,
                root,
                proto.clone(),
                console.clone(),
            )),
        );
    }

    if let Some(node_config) = &config.node {
        manager.register(
            PlatformType::Node.get_toolchain_id(),
            Box::new(NodePlatform::new(
                node_config,
                root,
                proto.clone(),
                console.clone(),
            )),
        );

        // TODO fix in 2.0
        if config.bun.is_none() {
            if let Some(bunpm_config) = &node_config.bun {
                let bun_config = BunConfig {
                    plugin: bunpm_config.plugin.clone(),
                    version: bunpm_config.version.clone(),
                    ..Default::default()
                };

                manager.register(
                    PlatformType::Bun.get_toolchain_id(),
                    Box::new(BunPlatform::new(
                        &bun_config,
                        root,
                        proto.clone(),
                        console.clone(),
                    )),
                );
            }
        }
    }

    if let Some(python_config) = &config.python {
        manager.register(
            PlatformType::Python.get_toolchain_id(),
            Box::new(PythonPlatform::new(
                python_config,
                root,
                proto.clone(),
                console.clone(),
            )),
        );
    }

    if let Some(rust_config) = &config.rust {
        manager.register(
            PlatformType::Rust.get_toolchain_id(),
            Box::new(RustPlatform::new(
                rust_config,
                root,
                proto.clone(),
                console.clone(),
            )),
        );
    }

    manager.register(
        PlatformType::System.get_toolchain_id(),
        Box::new(SystemPlatform::new(root, proto.clone(), console.clone())),
    );

    manager
}
