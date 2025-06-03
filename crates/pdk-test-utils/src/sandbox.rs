use crate::extension_wrapper::*;
use crate::toolchain_wrapper::*;
use moon_pdk_api::{
    RegisterExtensionInput, RegisterExtensionOutput, RegisterToolchainInput,
    RegisterToolchainOutput,
};
use proto_core::{ProtoEnvironment, Tool, inject_proto_manifest_config};
use proto_pdk_test_utils::WasmTestWrapper as ToolTestWrapper;
use starbase_sandbox::{Sandbox, create_empty_sandbox, create_sandbox};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use warpgate::{
    Id, PluginContainer, PluginLoader, PluginManifest, Wasm, host::*,
    inject_default_manifest_config, test_utils::*,
};

pub struct MoonWasmSandbox {
    pub sandbox: Sandbox,
    pub home_dir: PathBuf,
    pub moon_dir: PathBuf,
    pub proto: Arc<ProtoEnvironment>,
    pub proto_dir: PathBuf,
    pub root: PathBuf,
    pub wasm_file: PathBuf,
}

impl MoonWasmSandbox {
    pub fn new(sandbox: Sandbox) -> Self {
        let root = sandbox.path().to_path_buf();
        let home_dir = root.join(".home");
        let moon_dir = root.join(".moon");
        let proto_dir = root.join(".proto");
        let wasm_file = find_wasm_file();

        // Folders must exist for WASM to compile correctly!
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(&moon_dir).unwrap();
        fs::create_dir_all(&proto_dir).unwrap();

        // Required for toolchains
        let mut proto = ProtoEnvironment::new_testing(&root).unwrap();
        proto.working_dir = root.clone();

        Self {
            home_dir,
            moon_dir,
            proto: Arc::new(proto),
            proto_dir,
            root,
            sandbox,
            wasm_file,
        }
    }

    pub fn create_config(&self) -> ConfigBuilder {
        ConfigBuilder::new(&self.root, &self.home_dir)
    }

    pub async fn create_extension(&self, id: &str) -> ExtensionTestWrapper {
        self.create_extension_with_config(id, |_| {}).await
    }

    pub async fn create_extension_with_config(
        &self,
        id: &str,
        mut op: impl FnMut(&mut ConfigBuilder),
    ) -> ExtensionTestWrapper {
        let id = Id::new(id).unwrap();

        // Create manifest
        let mut manifest = PluginManifest::new([Wasm::file(self.wasm_file.clone())]);

        // Create config
        let mut config = self.create_config();
        config.plugin_id(&id);

        op(&mut config);

        manifest.config.extend(config.build());

        // Create plugin
        let plugin = self.create_plugin_container(id, manifest, false);
        let metadata: RegisterExtensionOutput = plugin
            .cache_func_with(
                "register_extension",
                RegisterExtensionInput {
                    id: plugin.id.to_string(),
                },
            )
            .await
            .unwrap();

        ExtensionTestWrapper {
            metadata,
            plugin,
            root: self.root.clone(),
        }
    }

    pub async fn create_toolchain(&self, id: &str) -> ToolchainTestWrapper {
        self.create_toolchain_with_config(id, |_| {}).await
    }

    pub async fn create_toolchain_with_config(
        &self,
        id: &str,
        mut op: impl FnMut(&mut ConfigBuilder),
    ) -> ToolchainTestWrapper {
        let id = Id::new(id).unwrap();

        // Create manifest
        let mut manifest = PluginManifest::new([Wasm::file(self.wasm_file.clone())]);

        // Create config
        let mut config = self.create_config();
        config.plugin_id(&id);

        op(&mut config);

        manifest.config.extend(config.build());

        // Create plugin
        let plugin = Arc::new(self.create_plugin_container(id, manifest, true));
        let metadata: RegisterToolchainOutput = plugin
            .cache_func_with(
                "register_toolchain",
                RegisterToolchainInput {
                    id: plugin.id.to_string(),
                },
            )
            .await
            .unwrap();

        ToolchainTestWrapper {
            metadata,
            plugin: plugin.clone(),
            root: self.root.clone(),
            tool: if plugin.has_func("register_tool").await {
                Some(ToolTestWrapper {
                    tool: Tool::new(plugin.id.clone(), self.proto.clone(), plugin)
                        .await
                        .unwrap(),
                })
            } else {
                None
            },
        }
    }

    fn create_plugin_container(
        &self,
        id: Id,
        mut manifest: PluginManifest,
        with_proto: bool,
    ) -> PluginContainer {
        let loader = PluginLoader::new(self.moon_dir.join("plugins"), self.moon_dir.join("temp"));

        let virtual_paths = BTreeMap::<PathBuf, PathBuf>::from_iter([
            (self.root.clone(), "/cwd".into()),
            (self.root.clone(), "/workspace".into()),
            (self.home_dir.clone(), "/userhome".into()),
            (self.moon_dir.clone(), "/moon".into()),
            (self.proto_dir.clone(), "/proto".into()),
        ]);

        manifest.timeout_ms = None;
        manifest = manifest.with_allowed_host("*");
        manifest = manifest.with_allowed_paths(
            virtual_paths
                .iter()
                .map(|(key, value)| (key.to_string_lossy().to_string(), value.to_owned())),
        );

        inject_default_manifest_config(&id, &self.home_dir, &mut manifest).unwrap();

        if with_proto {
            inject_proto_manifest_config(&id, &self.proto, &mut manifest).unwrap();
        }

        let funcs = create_host_functions(HostData {
            cache_dir: self.moon_dir.join("cache"),
            http_client: loader.get_client().unwrap().clone(),
            virtual_paths,
            working_dir: self.root.clone(),
        });

        PluginContainer::new(id, manifest, funcs).unwrap()
    }

    pub fn enable_logging(&self) {
        enable_wasm_logging(&self.wasm_file);
    }
}

impl Deref for MoonWasmSandbox {
    type Target = Sandbox;

    fn deref(&self) -> &Self::Target {
        &self.sandbox
    }
}

impl fmt::Debug for MoonWasmSandbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MoonWasmSandbox")
            .field("home_dir", &self.home_dir)
            .field("moon_dir", &self.moon_dir)
            .field("proto_dir", &self.proto_dir)
            .field("root", &self.root)
            .field("wasm_file", &self.wasm_file)
            .finish()
    }
}

pub fn create_moon_sandbox(fixture: &str) -> MoonWasmSandbox {
    MoonWasmSandbox::new(create_sandbox(fixture))
}

pub fn create_empty_moon_sandbox() -> MoonWasmSandbox {
    MoonWasmSandbox::new(create_empty_sandbox())
}
