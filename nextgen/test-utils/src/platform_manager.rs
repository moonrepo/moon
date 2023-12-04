use moon_bun_platform::BunPlatform;
use moon_config::{PlatformType, ToolchainConfig, ToolsConfig};
use moon_node_platform::NodePlatform;
use moon_platform::PlatformManager;
use moon_rust_platform::RustPlatform;
use moon_system_platform::SystemPlatform;
use proto_core::ProtoEnvironment;
use std::path::Path;
use std::sync::Arc;

pub async fn generate_platform_manager_from_sandbox(root: &Path) -> PlatformManager {
    let proto = Arc::new(ProtoEnvironment::new_testing(root));
    let config = ToolchainConfig::load_from(root, &ToolsConfig::default()).unwrap();
    let mut manager = PlatformManager::default();

    if let Some(bun_config) = &config.bun {
        manager.register(
            PlatformType::Bun,
            Box::new(BunPlatform::new(
                bun_config,
                &None,
                root,
                proto.clone(),
                config.node.is_some(),
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
                config.bun.is_some(),
            )),
        );
    }

    if let Some(rust_config) = &config.rust {
        manager.register(
            PlatformType::Rust,
            Box::new(RustPlatform::new(rust_config, root, proto.clone())),
        );
    }

    manager.register(PlatformType::System, Box::<SystemPlatform>::default());

    manager
}
