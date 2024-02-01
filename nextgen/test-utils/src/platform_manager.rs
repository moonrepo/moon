use moon_bun_platform::BunPlatform;
use moon_config::{PlatformType, ToolchainConfig};
use moon_console::Console;
use moon_node_platform::NodePlatform;
use moon_platform::PlatformManager;
use moon_rust_platform::RustPlatform;
use moon_system_platform::SystemPlatform;
use proto_core::{ProtoConfig, ProtoEnvironment};
use std::path::Path;
use std::sync::Arc;

pub async fn generate_platform_manager_from_sandbox(root: &Path) -> PlatformManager {
    let proto = Arc::new(ProtoEnvironment::new_testing(root));
    let console = Arc::new(Console::new_testing());
    let config = ToolchainConfig::load_from(root, &ProtoConfig::default()).unwrap();
    let mut manager = PlatformManager::default();

    if let Some(bun_config) = &config.bun {
        manager.register(
            PlatformType::Bun,
            Box::new(BunPlatform::new(
                bun_config,
                &None,
                root,
                proto.clone(),
                console.clone(),
            )),
        );
    }

    if let Some(node_config) = &config.node {
        manager.register(
            PlatformType::Node,
            Box::new(NodePlatform::new(
                node_config,
                &None,
                root,
                proto.clone(),
                console.clone(),
            )),
        );
    }

    if let Some(rust_config) = &config.rust {
        manager.register(
            PlatformType::Rust,
            Box::new(RustPlatform::new(
                rust_config,
                root,
                proto.clone(),
                console.clone(),
            )),
        );
    }

    manager.register(
        PlatformType::System,
        Box::new(SystemPlatform::new(root, proto.clone(), console.clone())),
    );

    manager
}
