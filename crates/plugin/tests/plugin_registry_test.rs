use moon_env::MoonEnvironment;
use moon_plugin::{
    Plugin, PluginId as Id, PluginLocator, PluginRegistration, PluginRegistry, PluginType,
};
use proto_core::ProtoEnvironment;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::fs;
use std::path::Path;
use std::sync::Arc;

struct TestPlugin;

impl Plugin for TestPlugin {
    fn new(_reg: PluginRegistration) -> miette::Result<Self> {
        Ok(TestPlugin)
    }

    fn get_type(&self) -> PluginType {
        PluginType::Extension
    }
}

fn create_registry(sandbox: &Path) -> PluginRegistry<TestPlugin> {
    let registry = PluginRegistry::new(
        PluginType::Extension,
        Arc::new(MoonEnvironment::new_testing(sandbox)),
        Arc::new(ProtoEnvironment::new_testing(sandbox).unwrap()),
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
    async fn access_errors_if_unknown_id() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path());

        registry
            .access(&Id::raw("unknown"), |_| async { Ok(()) })
            .await
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "The extension plugin unknown does not exist.")]
    fn access_sync_errors_if_unknown_id() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path());

        registry
            .access_sync(&Id::raw("unknown"), |_| Ok(()))
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "The extension plugin unknown does not exist.")]
    async fn perform_errors_if_unknown_id() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path());

        registry
            .perform(&Id::raw("unknown"), |_, _| async { Ok(()) })
            .await
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "The extension plugin unknown does not exist.")]
    fn perform_sync_errors_if_unknown_id() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path());

        registry
            .perform_sync(&Id::raw("unknown"), |_, _| Ok(()))
            .unwrap();
    }

    #[tokio::test]
    async fn loads_plugin_from_file() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path());

        registry
            .load(
                Id::raw("id"),
                PluginLocator::File {
                    file: "".into(),
                    path: Some(sandbox.path().join("test.wasm")),
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "The extension plugin id already exists.")]
    async fn loads_errors_if_id_exists() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path());

        registry
            .load(
                Id::raw("id"),
                PluginLocator::File {
                    file: "".into(),
                    path: Some(sandbox.path().join("test.wasm")),
                },
            )
            .await
            .unwrap();

        registry
            .load(
                Id::raw("id"),
                PluginLocator::File {
                    file: "".into(),
                    path: Some(sandbox.path().join("test.wasm")),
                },
            )
            .await
            .unwrap();
    }
}
