use async_trait::async_trait;
use moon_env::MoonEnvironment;
use moon_plugin::{
    Plugin, PluginHostData, PluginId as Id, PluginLocator, PluginRegistration, PluginRegistry,
    PluginType,
};
use proto_core::{ProtoEnvironment, warpgate::FileLocator};
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

#[derive(Debug)]
struct TestPlugin;

#[async_trait]
impl Plugin for TestPlugin {
    async fn new(_reg: PluginRegistration) -> miette::Result<Self> {
        Ok(TestPlugin)
    }

    fn get_type(&self) -> PluginType {
        PluginType::Extension
    }
}

fn create_registry(sandbox: &Path) -> PluginRegistry<TestPlugin> {
    let registry = PluginRegistry::new(
        PluginType::Extension,
        PluginHostData {
            moon_env: Arc::new(MoonEnvironment::new_testing(sandbox)),
            proto_env: Arc::new(ProtoEnvironment::new_testing(sandbox).unwrap()),
            workspace_graph: Arc::new(OnceLock::new()),
        },
    );

    // These must exist or extism errors
    for host_path in registry.get_virtual_paths().keys() {
        fs::create_dir_all(host_path).unwrap();
    }

    registry
}

mod plugin_registry {
    use super::*;

    #[test]
    fn removes_duplicate_workspace_vpath() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path());
        let mut count = 0;

        for guest in registry.get_virtual_paths().values() {
            if guest.to_str().unwrap() == "/workspace" {
                count += 1;
            }
        }

        assert_eq!(count, 1);
    }

    #[tokio::test]
    #[should_panic(expected = "The extension plugin unknown does not exist.")]
    async fn errors_if_unknown_id() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path());

        registry.get_instance(&Id::raw("unknown")).await.unwrap();
    }

    #[tokio::test]
    async fn loads_plugin_from_file() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path());

        registry
            .load_without_config(
                Id::raw("id"),
                PluginLocator::File(Box::new(FileLocator {
                    file: "".into(),
                    path: Some(sandbox.path().join("test.wasm")),
                })),
            )
            .await
            .unwrap();
    }
}
