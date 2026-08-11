use async_trait::async_trait;
use moon_common::Id;
use moon_env::MoonEnvironment;
use moon_plugin::{
    MoonHostData, Plugin, PluginLocator, PluginManifest, PluginRegistration, PluginRegistry,
    PluginType, PluginsConfig,
};
use proto_core::{ProtoEnvironment, warpgate::FileLocator};
use rustc_hash::FxHashMap;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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

    async fn has_func(&self, name: &str) -> bool {
        name != "missing_func"
    }
}

#[derive(Debug, Default)]
struct TestConfig {
    plugins: FxHashMap<Id, PluginLocator>,
    configured: Mutex<Vec<Id>>,
}

impl TestConfig {
    fn new(ids: &[&str], sandbox: &Path) -> Self {
        Self {
            plugins: ids
                .iter()
                .map(|id| (Id::raw(id), create_locator(sandbox)))
                .collect(),
            configured: Mutex::default(),
        }
    }
}

impl PluginsConfig for TestConfig {
    fn configure_manifest(
        &self,
        id: &Id,
        _host_data: &MoonHostData,
        _manifest: &mut PluginManifest,
    ) -> miette::Result<()> {
        self.configured.lock().unwrap().push(id.to_owned());

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

fn create_registry(sandbox: &Path, config: TestConfig) -> PluginRegistry<TestConfig, TestPlugin> {
    let registry = PluginRegistry::new(
        PluginType::Extension,
        MoonHostData {
            moon_env: Arc::new(MoonEnvironment::new_testing(sandbox)),
            proto_env: Arc::new(ProtoEnvironment::new_testing(sandbox).unwrap()),
            ..Default::default()
        },
        config,
    )
    .unwrap();

    // These must exist or extism errors
    for (host_path, _) in registry.get_virtual_paths() {
        fs::create_dir_all(host_path).unwrap();
    }

    registry
}

fn create_test_plugin(id: &str) -> TestPlugin {
    TestPlugin { id: Id::raw(id) }
}

mod plugin_registry {
    use super::*;

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
