use async_trait::async_trait;
use moon_common::Id;
use moon_config::{ExtensionsConfig, ToolchainsConfig, WorkspaceConfig};
use moon_env::MoonEnvironment;
use moon_plugin::{
    MoonHostData, Plugin, PluginLocator, PluginRegistration, PluginRegistry, PluginType,
};
use proto_core::{ProtoEnvironment, warpgate::FileLocator};
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

#[derive(Debug)]
struct TestPlugin {
    id: Id,
}

#[async_trait]
impl Plugin for TestPlugin {
    async fn new(reg: PluginRegistration) -> miette::Result<Self> {
        Ok(TestPlugin { id: reg.id })
    }

    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_type(&self) -> PluginType {
        PluginType::Extension
    }
}

fn create_registry(sandbox: &Path) -> PluginRegistry<TestPlugin> {
    let registry = PluginRegistry::new(
        PluginType::Extension,
        MoonHostData {
            moon_env: Arc::new(MoonEnvironment::new_testing(sandbox)),
            proto_env: Arc::new(ProtoEnvironment::new_testing(sandbox).unwrap()),
            extensions_config: Arc::new(ExtensionsConfig::default()),
            toolchains_config: Arc::new(ToolchainsConfig::default()),
            workspace_config: Arc::new(WorkspaceConfig::default()),
            workspace_graph: Arc::new(OnceLock::new()),
        },
    )
    .unwrap();

    // These must exist or extism errors
    for (host_path, _) in registry.get_virtual_paths() {
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

        for (_, guest) in registry.get_virtual_paths() {
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

    #[tokio::test]
    async fn verifies_plugin_before_registration() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path());
        let wasm_file = sandbox.path().join("test.wasm");

        registry
            .load_verified_without_config(
                Id::raw("verified"),
                PluginLocator::File(Box::new(FileLocator {
                    file: "".into(),
                    path: Some(wasm_file.clone()),
                })),
                |path, bytes| {
                    assert_eq!(path, wasm_file);
                    assert_eq!(bytes, fs::read(&wasm_file).unwrap());
                    Ok(())
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn rejects_plugin_that_fails_verification() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path());

        let error = registry
            .load_verified_without_config(
                Id::raw("untrusted"),
                PluginLocator::File(Box::new(FileLocator {
                    file: "".into(),
                    path: Some(sandbox.path().join("test.wasm")),
                })),
                |_, _| Err(miette::miette!("plugin digest mismatch")),
            )
            .await
            .unwrap_err();

        assert!(error.to_string().contains("plugin digest mismatch"));
        assert!(!registry.is_registered(&Id::raw("untrusted")).await);
    }

    #[tokio::test]
    async fn verifies_an_already_registered_plugin() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path());
        let locator = PluginLocator::File(Box::new(FileLocator {
            file: "".into(),
            path: Some(sandbox.path().join("test.wasm")),
        }));

        registry
            .load_without_config(Id::raw("existing"), locator.clone())
            .await
            .unwrap();

        let error = registry
            .load_verified_without_config(Id::raw("existing"), locator, |_, _| {
                Err(miette::miette!("existing plugin digest mismatch"))
            })
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("existing plugin digest mismatch")
        );
    }

    #[tokio::test]
    async fn instantiates_the_exact_verified_bytes() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path());
        let wasm_file = sandbox.path().join("test.wasm");
        let verified_bytes = fs::read(&wasm_file).unwrap();

        registry
            .load_verified_without_config(
                Id::raw("immutable"),
                PluginLocator::File(Box::new(FileLocator {
                    file: "".into(),
                    path: Some(wasm_file.clone()),
                })),
                |path, bytes| {
                    assert_eq!(bytes, verified_bytes);
                    fs::write(path, b"not wasm").unwrap();
                    Ok(())
                },
            )
            .await
            .unwrap();

        assert_eq!(fs::read(wasm_file).unwrap(), b"not wasm");
    }
}
