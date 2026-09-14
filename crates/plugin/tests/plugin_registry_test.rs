use async_trait::async_trait;
use moon_common::Id;
use moon_env::MoonEnvironment;
use moon_pdk_api::{
    ProcessCommandInput, ProcessCommandResult, ProcessOutputChunk, ProcessOutputChunkInput,
    ProcessOutputStream, RegisterVcsInput, RegisterVcsOutput, VCS_PLUGIN_PROTOCOL_VERSION,
    VirtualPath,
};
use moon_plugin::{
    CallOptions, MoonHostData, Plugin, PluginLocator, PluginManifest, PluginRegistration,
    PluginRegistry, PluginType, PluginsConfig,
};
use proto_core::{ProtoEnvironment, warpgate::FileLocator};
use rustc_hash::FxHashMap;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct TestPlugin {
    id: Id,
}

#[async_trait]
impl Plugin for TestPlugin {
    async fn new(mut reg: PluginRegistration) -> miette::Result<Self> {
        assert!(reg.take_process_host_access().is_err());

        Ok(TestPlugin { id: reg.id })
    }

    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_type() -> PluginType {
        PluginType::Extension
    }

    async fn has_func(&self, name: &str) -> bool {
        name != "missing_func"
    }
}

struct ProcessVcsPlugin {
    id: Id,
    plugin: Arc<moon_plugin::PluginContainer>,
}

impl ProcessVcsPlugin {
    async fn execute_process(
        &self,
        input: ProcessCommandInput,
    ) -> miette::Result<(i32, Vec<u8>, Vec<u8>)> {
        Ok(self.plugin.call_func_with("execute_process", input).await?)
    }

    async fn start_process(
        &self,
        input: ProcessCommandInput,
    ) -> miette::Result<ProcessCommandResult> {
        Ok(self.plugin.call_func_with("start_process", input).await?)
    }

    async fn read_process_output(
        &self,
        input: ProcessOutputChunkInput,
    ) -> miette::Result<ProcessOutputChunk> {
        Ok(self
            .plugin
            .call_func_with("read_process_output", input)
            .await?)
    }
}

impl std::fmt::Debug for ProcessVcsPlugin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProcessVcsPlugin")
            .field("id", &self.id)
            .finish()
    }
}

#[async_trait]
impl Plugin for ProcessVcsPlugin {
    async fn new(mut reg: PluginRegistration) -> miette::Result<Self> {
        let process_access = reg.take_process_host_access()?;
        let workspace_root = reg.moon_env.workspace_root.clone();
        let plugin = Arc::new(reg.container);
        let metadata: RegisterVcsOutput = plugin
            .call_func_with(
                "register_vcs",
                RegisterVcsInput {
                    id: reg.id.clone(),
                    host_protocol_version: VCS_PLUGIN_PROTOCOL_VERSION,
                },
            )
            .await?;
        process_access.configure(&metadata.process_capabilities, &workspace_root)?;

        Ok(Self { id: reg.id, plugin })
    }

    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_type() -> PluginType {
        PluginType::Vcs
    }

    async fn has_func(&self, name: &str) -> bool {
        self.plugin.has_func(name).await
    }
}

#[derive(Debug)]
struct UnconfiguredProcessVcsPlugin;

#[async_trait]
impl Plugin for UnconfiguredProcessVcsPlugin {
    async fn new(mut reg: PluginRegistration) -> miette::Result<Self> {
        reg.take_process_host_access()?;
        let _: (i32, Vec<u8>, Vec<u8>) = reg
            .container
            .call_func_with(
                "execute_process",
                ProcessCommandInput {
                    capability: Id::raw("git"),
                    args: vec!["--version".into()],
                    cwd: None,
                    env: Default::default(),
                },
            )
            .await?;

        Ok(Self)
    }

    fn get_id(&self) -> &Id {
        unreachable!()
    }

    fn get_type() -> PluginType {
        PluginType::Vcs
    }

    async fn has_func(&self, _name: &str) -> bool {
        false
    }
}

#[derive(Debug, Default)]
struct TestConfig {
    plugins: FxHashMap<Id, PluginLocator>,
    configured: Mutex<Vec<Id>>,
    allowed_path: Option<PathBuf>,
}

impl TestConfig {
    fn new(ids: &[&str], sandbox: &Path) -> Self {
        Self {
            plugins: ids
                .iter()
                .map(|id| (Id::raw(id), create_locator(sandbox)))
                .collect(),
            configured: Mutex::default(),
            allowed_path: None,
        }
    }

    fn with_allowed_path(mut self, path: PathBuf) -> Self {
        self.allowed_path = Some(path);
        self
    }
}

impl PluginsConfig for TestConfig {
    fn configure_manifest(
        &self,
        id: &Id,
        _host_data: &MoonHostData,
        manifest: &mut PluginManifest,
    ) -> miette::Result<()> {
        self.configured.lock().unwrap().push(id.to_owned());

        if let Some(path) = &self.allowed_path {
            manifest
                .allowed_paths
                .get_or_insert_default()
                .insert(path.to_string_lossy().into_owned(), "/configured".into());
        }

        Ok(())
    }

    fn get_ids(&self) -> Vec<&Id> {
        self.plugins.keys().collect()
    }

    fn get_locator(&self, id: &Id) -> Option<&PluginLocator> {
        self.plugins.get(id)
    }
}

fn create_locator(sandbox: &Path) -> PluginLocator {
    PluginLocator::File(Box::new(FileLocator {
        file: "".into(),
        path: Some(sandbox.join("test.wasm")),
    }))
}

fn create_process_host_locator(sandbox: &Path) -> PluginLocator {
    PluginLocator::File(Box::new(FileLocator {
        file: "".into(),
        path: Some(sandbox.join("process_host.wasm")),
    }))
}

fn create_registry(sandbox: &Path, config: TestConfig) -> PluginRegistry<TestConfig, TestPlugin> {
    PluginRegistry::new(
        PluginType::Extension,
        MoonHostData {
            moon_env: Arc::new(MoonEnvironment::new_testing(sandbox)),
            proto_env: Arc::new(ProtoEnvironment::new_testing(sandbox).unwrap()),
            ..Default::default()
        },
        config,
    )
    .unwrap()
}

fn create_process_vcs_registry<Inst: Plugin>(sandbox: &Path) -> PluginRegistry<TestConfig, Inst> {
    PluginRegistry::new(
        PluginType::Vcs,
        MoonHostData {
            moon_env: Arc::new(MoonEnvironment::new_testing(sandbox)),
            proto_env: Arc::new(ProtoEnvironment::new_testing(sandbox).unwrap()),
            ..Default::default()
        },
        TestConfig::default(),
    )
    .unwrap()
}

fn create_test_plugin(id: &str) -> TestPlugin {
    TestPlugin { id: Id::raw(id) }
}

mod plugin_registry {
    use super::*;

    #[test]
    fn rejects_plugin_types_that_do_not_match_the_registry() {
        let sandbox = create_empty_sandbox();
        let error = PluginRegistry::<TestConfig, TestPlugin>::new(
            PluginType::Vcs,
            MoonHostData {
                moon_env: Arc::new(MoonEnvironment::new_testing(sandbox.path())),
                proto_env: Arc::new(ProtoEnvironment::new_testing(sandbox.path()).unwrap()),
                ..Default::default()
            },
            TestConfig::default(),
        )
        .err()
        .unwrap()
        .to_string();

        assert!(error.contains("extension plugin"), "{error}");
        assert!(error.contains("VCS registry"), "{error}");
    }

    #[test]
    fn removes_duplicate_workspace_vpath() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());
        let mut count = 0;

        for (_, guest) in registry.get_virtual_paths() {
            if guest.to_str().unwrap() == "/workspace" {
                count += 1;
            }
        }

        assert_eq!(count, 1);
    }

    #[test]
    fn returns_plugin_ids_from_config() {
        let sandbox = create_empty_sandbox();
        let registry =
            create_registry(sandbox.path(), TestConfig::new(&["a", "b"], sandbox.path()));

        let mut ids = registry.get_plugin_ids();
        ids.sort();

        assert_eq!(ids, vec![&Id::raw("a"), &Id::raw("b")]);
        assert!(registry.has_plugin_configs());
    }

    #[test]
    fn has_no_plugin_configs_when_config_is_empty() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());

        assert!(registry.get_plugin_ids().is_empty());
        assert!(!registry.has_plugin_configs());
    }

    #[test]
    fn creates_context_with_virtual_paths() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());

        let context = registry.create_context();

        assert_eq!(*context.workspace_root, PathBuf::from("/workspace"));
    }

    #[tokio::test]
    async fn registers_and_returns_an_instance() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());
        let id = Id::raw("test");

        assert!(!registry.is_registered(&id).await);

        registry
            .register(id.clone(), create_test_plugin("test"))
            .await
            .unwrap();

        assert!(registry.is_registered(&id).await);
        assert_eq!(registry.get_instance(&id).await.unwrap().get_id(), &id);
    }

    #[tokio::test]
    #[should_panic(expected = "The extension plugin dupe already exists.")]
    async fn errors_if_registering_an_existing_id() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());

        registry
            .register(Id::raw("dupe"), create_test_plugin("dupe"))
            .await
            .unwrap();
        registry
            .register(Id::raw("dupe"), create_test_plugin("dupe"))
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "The extension plugin unknown does not exist.")]
    async fn errors_if_unknown_id() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());

        registry.get_instance(&Id::raw("unknown")).await.unwrap();
    }

    #[tokio::test]
    async fn clones_share_the_same_plugin_cache() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());
        let clone = registry.clone();
        let id = Id::raw("test");

        clone
            .register(id.clone(), create_test_plugin("test"))
            .await
            .unwrap();

        assert!(registry.is_registered(&id).await);
        assert!(Arc::ptr_eq(
            &registry.get_instance(&id).await.unwrap(),
            &clone.get_instance(&id).await.unwrap(),
        ));
    }
}

mod registry_loader {
    use super::*;

    #[tokio::test]
    async fn executes_declared_processes_through_the_vcs_host() {
        let sandbox = create_sandbox("wasm");
        let registry = create_process_vcs_registry::<ProcessVcsPlugin>(sandbox.path());
        let plugin = registry
            .do_load(
                Id::raw("process-host"),
                create_process_host_locator(sandbox.path()),
            )
            .await
            .unwrap();

        let (exit_code, stdout, stderr) = plugin
            .execute_process(ProcessCommandInput {
                capability: Id::raw("git"),
                args: vec!["--version".into()],
                cwd: None,
                env: BTreeMap::new(),
            })
            .await
            .unwrap();
        assert_eq!(exit_code, 0);
        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .starts_with("git version")
        );
        assert!(stderr.is_empty());

        plugin
            .execute_process(ProcessCommandInput {
                capability: Id::raw("git"),
                args: vec!["init".into(), "--quiet".into()],
                cwd: Some(VirtualPath::new("/workspace")),
                env: BTreeMap::new(),
            })
            .await
            .unwrap();
        let expected = vec![0, 0xff, b'a', b'\n'];
        fs::write(sandbox.path().join("bytes.bin"), &expected).unwrap();
        let (_, hash, _) = plugin
            .execute_process(ProcessCommandInput {
                capability: Id::raw("git"),
                args: vec!["hash-object".into(), "-w".into(), "bytes.bin".into()],
                cwd: Some(VirtualPath::new("/workspace")),
                env: BTreeMap::new(),
            })
            .await
            .unwrap();
        let hash = String::from_utf8(hash).unwrap().trim().to_owned();
        let (exit_code, stdout, stderr) = plugin
            .execute_process(ProcessCommandInput {
                capability: Id::raw("git"),
                args: vec!["cat-file".into(), "blob".into(), hash],
                cwd: Some(VirtualPath::new("/workspace")),
                env: BTreeMap::new(),
            })
            .await
            .unwrap();
        assert_eq!(exit_code, 0);
        assert_eq!(stdout, expected);
        assert!(stderr.is_empty());

        let (exit_code, stdout, stderr) = plugin
            .execute_process(ProcessCommandInput {
                capability: Id::raw("git"),
                args: vec!["definitely-not-a-git-command".into()],
                cwd: None,
                env: BTreeMap::new(),
            })
            .await
            .unwrap();
        assert_ne!(exit_code, 0);
        assert!(stdout.is_empty());
        assert!(!stderr.is_empty());

        let (_, stdout, _) = plugin
            .execute_process(ProcessCommandInput {
                capability: Id::raw("git"),
                args: vec!["var".into(), "GIT_EDITOR".into()],
                cwd: None,
                env: BTreeMap::from([("GIT_EDITOR".into(), "moon-test-editor".into())]),
            })
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8(stdout).unwrap().trim(),
            "moon-test-editor"
        );

        let error = plugin
            .execute_process(ProcessCommandInput {
                capability: Id::raw("undeclared"),
                args: vec![],
                cwd: None,
                env: BTreeMap::new(),
            })
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("undeclared process capability"), "{error}");

        fs::write(sandbox.path().join("not-a-directory"), "file").unwrap();
        let error = plugin
            .execute_process(ProcessCommandInput {
                capability: Id::raw("git"),
                args: vec!["--version".into()],
                cwd: Some(VirtualPath::new("/workspace/not-a-directory")),
                env: BTreeMap::new(),
            })
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("not a directory"), "{error}");
    }

    #[tokio::test]
    async fn isolates_and_invalidates_raw_process_results() {
        let sandbox = create_sandbox("wasm");
        let registry = create_process_vcs_registry::<ProcessVcsPlugin>(sandbox.path());
        let first = registry
            .do_load(
                Id::raw("process-host-a"),
                create_process_host_locator(sandbox.path()),
            )
            .await
            .unwrap();
        let second = registry
            .do_load(
                Id::raw("process-host-b"),
                create_process_host_locator(sandbox.path()),
            )
            .await
            .unwrap();
        let result = first
            .start_process(ProcessCommandInput {
                capability: Id::raw("git"),
                args: vec!["--version".into()],
                cwd: None,
                env: BTreeMap::new(),
            })
            .await
            .unwrap();
        let read = ProcessOutputChunkInput {
            result_id: result.result_id,
            stream: ProcessOutputStream::Stdout,
            offset: 0,
        };

        assert!(second.read_process_output(read.clone()).await.is_err());
        assert!(
            !first
                .read_process_output(read.clone())
                .await
                .unwrap()
                .decode()
                .unwrap()
                .is_empty()
        );

        assert!(
            first
                .start_process(ProcessCommandInput {
                    capability: Id::raw("undeclared"),
                    args: vec![],
                    cwd: None,
                    env: BTreeMap::new(),
                })
                .await
                .is_err()
        );
        assert!(first.read_process_output(read).await.is_err());
    }

    #[tokio::test]
    async fn rejects_process_execution_before_capability_configuration() {
        let sandbox = create_sandbox("wasm");
        let registry = create_process_vcs_registry::<UnconfiguredProcessVcsPlugin>(sandbox.path());
        let error = registry
            .do_load(
                Id::raw("process-host"),
                create_process_host_locator(sandbox.path()),
            )
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("not configured"), "{error}");
    }

    #[tokio::test]
    async fn loads_a_plugin_with_an_explicit_locator() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::default());

        let plugin = registry
            .do_load(Id::raw("id"), create_locator(sandbox.path()))
            .await
            .unwrap();

        assert_eq!(plugin.get_id(), &Id::raw("id"));
        assert!(registry.is_registered(&Id::raw("id")).await);
    }

    #[tokio::test]
    async fn creates_final_manifest_allowed_paths_before_loading() {
        let sandbox = create_sandbox("wasm");
        let allowed_path = sandbox.path().join("configured").join("nested");
        let config = TestConfig::default().with_allowed_path(allowed_path.clone());
        let registry = create_registry(sandbox.path(), config);

        assert!(!allowed_path.exists());

        registry
            .do_load(Id::raw("id"), create_locator(sandbox.path()))
            .await
            .unwrap();

        assert!(allowed_path.is_dir());
    }

    #[tokio::test]
    async fn loads_a_plugin_with_the_configured_locator() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::new(&["a"], sandbox.path()));

        let plugin = registry.load("a").await.unwrap();

        assert_eq!(plugin.get_id(), &Id::raw("a"));
        assert!(registry.is_registered(&Id::raw("a")).await);
    }

    #[tokio::test]
    async fn configures_the_manifest_when_loading() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::new(&["a"], sandbox.path()));

        registry.load("a").await.unwrap();

        assert_eq!(
            *registry.config_data.configured.lock().unwrap(),
            vec![Id::raw("a")]
        );
    }

    #[tokio::test]
    #[should_panic(expected = "The extension plugin unknown does not exist.")]
    async fn errors_if_loading_an_unconfigured_id() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());

        registry.load("unknown").await.unwrap();
    }

    #[tokio::test]
    async fn loads_a_registered_instance_thats_not_configured() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());

        registry
            .register(Id::raw("manual"), create_test_plugin("manual"))
            .await
            .unwrap();

        let plugin = registry.load("manual").await.unwrap();

        assert_eq!(plugin.get_id(), &Id::raw("manual"));
    }

    #[tokio::test]
    async fn caches_and_reuses_loaded_instances() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::new(&["a"], sandbox.path()));

        let one = registry.load("a").await.unwrap();
        let two = registry.load("a").await.unwrap();

        assert!(Arc::ptr_eq(&one, &two));

        // Only configured once as well
        assert_eq!(registry.config_data.configured.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn concurrent_loads_share_one_instance() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::new(&["a"], sandbox.path()));

        let (one, two) = tokio::join!(registry.load("a"), registry.load("a"));

        assert!(Arc::ptr_eq(&one.unwrap(), &two.unwrap()));
    }

    #[tokio::test]
    async fn loads_many_plugins() {
        let sandbox = create_sandbox("wasm");
        let registry =
            create_registry(sandbox.path(), TestConfig::new(&["a", "b"], sandbox.path()));

        let plugins = registry.load_many(["a", "b"]).await.unwrap();
        let mut ids = plugins
            .iter()
            .map(|plugin| plugin.get_id().to_owned())
            .collect::<Vec<_>>();
        ids.sort();

        assert_eq!(ids, vec![Id::raw("a"), Id::raw("b")]);
    }

    #[tokio::test]
    async fn load_many_skips_unconfigured_ids() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::new(&["a"], sandbox.path()));

        let plugins = registry.load_many(["a", "unknown"]).await.unwrap();

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].get_id(), &Id::raw("a"));
    }

    #[tokio::test]
    async fn load_many_returns_registered_instances_in_requested_order() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());

        registry
            .register(Id::raw("a"), create_test_plugin("a"))
            .await
            .unwrap();
        registry
            .register(Id::raw("b"), create_test_plugin("b"))
            .await
            .unwrap();

        let plugins = registry.load_many(["b", "a"]).await.unwrap();
        let ids = plugins
            .iter()
            .map(|plugin| plugin.get_id().to_owned())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec![Id::raw("b"), Id::raw("a")]);
    }

    #[tokio::test]
    async fn load_many_loads_unregistered_plugins_in_requested_order() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(
            sandbox.path(),
            TestConfig::new(&["a", "b", "c"], sandbox.path()),
        );

        let plugins = registry.load_many(["c", "a", "b"]).await.unwrap();
        let ids = plugins
            .iter()
            .map(|plugin| plugin.get_id().to_owned())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec![Id::raw("c"), Id::raw("a"), Id::raw("b")]);
    }

    #[tokio::test]
    async fn load_all_returns_empty_when_no_configs() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());

        assert!(registry.load_all().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn load_all_loads_every_configured_plugin() {
        let sandbox = create_sandbox("wasm");
        let registry =
            create_registry(sandbox.path(), TestConfig::new(&["a", "b"], sandbox.path()));

        assert_eq!(registry.load_all().await.unwrap().len(), 2);
    }
}

mod registry_caller {
    use super::*;

    #[tokio::test]
    async fn calls_the_func_for_all_configured_plugins() {
        let sandbox = create_sandbox("wasm");
        let registry =
            create_registry(sandbox.path(), TestConfig::new(&["a", "b"], sandbox.path()));

        let results = registry
            .call_func_all(
                "do_thing",
                |plugin| plugin.get_id().to_string(),
                |_plugin, input| async move { Ok::<_, miette::Report>(format!("{input}-output")) },
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 2);

        let mut outputs = results
            .iter()
            .map(|result| result.output.clone())
            .collect::<Vec<_>>();
        outputs.sort();

        assert_eq!(outputs, vec!["a-output", "b-output"]);

        for result in results {
            assert_eq!(result.id, result.plugin.id);
            assert!(result.operation.finished_at.is_some());
        }
    }

    #[tokio::test]
    async fn calls_the_func_for_a_subset_of_ids() {
        let sandbox = create_sandbox("wasm");
        let registry =
            create_registry(sandbox.path(), TestConfig::new(&["a", "b"], sandbox.path()));

        let results = registry
            .call_func(
                "do_thing",
                ["a"],
                |plugin| plugin.get_id().to_string(),
                |_plugin, input| async move { Ok::<_, miette::Report>(input) },
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, Id::raw("a"));
    }

    #[tokio::test]
    async fn returns_empty_when_no_plugins_configured() {
        let sandbox = create_empty_sandbox();
        let registry = create_registry(sandbox.path(), TestConfig::default());

        let results = registry
            .call_func_all(
                "do_thing",
                |plugin| plugin.get_id().to_string(),
                |_plugin, input| async move { Ok::<_, miette::Report>(input) },
            )
            .await
            .unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn skips_plugins_without_the_func_by_default() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::new(&["a"], sandbox.path()));

        let results = registry
            .call_func_all(
                "missing_func",
                |plugin| plugin.get_id().to_string(),
                |_plugin, input| async move { Ok::<_, miette::Report>(input) },
            )
            .await
            .unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn calls_a_missing_func_when_the_check_is_disabled() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::new(&["a"], sandbox.path()));

        let results = registry
            .call_func_all_with_options(
                "missing_func",
                |plugin| plugin.get_id().to_string(),
                |_plugin, input| async move { Ok::<_, miette::Report>(input) },
                CallOptions {
                    check_func_exists: false,
                },
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn maps_call_result_outputs() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::new(&["a"], sandbox.path()));

        let results = registry
            .call_func_all(
                "do_thing",
                |plugin| plugin.get_id().to_string(),
                |_plugin, input| async move { Ok::<_, miette::Report>(input) },
            )
            .await
            .unwrap();

        let result = results.into_iter().next().unwrap();
        let mapped = result.map_output(|output| output.len());

        assert_eq!(mapped.id, Id::raw("a"));
        assert_eq!(mapped.output, 1);
        assert_eq!(mapped.plugin.get_id(), &Id::raw("a"));
        assert!(mapped.operation.finished_at.is_some());
    }

    #[tokio::test]
    #[should_panic(expected = "boom")]
    async fn propagates_func_call_errors() {
        let sandbox = create_sandbox("wasm");
        let registry = create_registry(sandbox.path(), TestConfig::new(&["a"], sandbox.path()));

        registry
            .call_func_all(
                "do_thing",
                |plugin| plugin.get_id().to_string(),
                |_plugin, _input| async move { Err::<(), _>(miette::miette!("boom")) },
            )
            .await
            .unwrap();
    }
}
