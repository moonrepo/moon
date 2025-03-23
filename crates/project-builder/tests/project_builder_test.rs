use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{
    BunConfig, ConfigLoader, DenoConfig, DependencyConfig, DependencyScope, DependencySource,
    LanguageType, NodeConfig, RustConfig, TaskArgs, TaskConfig, ToolchainConfig,
    ToolchainPluginConfig,
};
use moon_file_group::FileGroup;
use moon_plugin::PluginId;
use moon_project::Project;
use moon_project_builder::{ProjectBuilder, ProjectBuilderContext};
use moon_toolchain_plugin::ToolchainRegistry;
use rustc_hash::FxHashMap;
use starbase_sandbox::create_sandbox;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// We need some top-level struct to hold the data used for lifetime refs.
struct Stub {
    config_loader: ConfigLoader,
    enabled_toolchains: Vec<Id>,
    toolchain_config: ToolchainConfig,
    workspace_root: PathBuf,
    id: Id,
    source: WorkspaceRelativePathBuf,
}

impl Stub {
    pub fn new(id: &str, root: &Path) -> Self {
        // Enable platforms so that detection works
        let mut toolchain_config = ToolchainConfig {
            bun: Some(BunConfig::default()),
            deno: Some(DenoConfig::default()),
            node: Some(NodeConfig::default()),
            rust: Some(RustConfig::default()),
            plugins: FxHashMap::from_iter([(
                Id::raw("typescript"),
                ToolchainPluginConfig::default(),
            )]),
            ..ToolchainConfig::default()
        };

        toolchain_config.inherit_plugin_locators().unwrap();

        Self {
            config_loader: ConfigLoader::default(),
            enabled_toolchains: toolchain_config.get_enabled(),
            toolchain_config,
            workspace_root: root.to_path_buf(),
            id: Id::raw(id),
            source: WorkspaceRelativePathBuf::from(id),
        }
    }

    pub async fn create_builder(&self) -> ProjectBuilder {
        let mut toolchain_registry = ToolchainRegistry::default();

        for (id, config) in &self.toolchain_config.plugins {
            toolchain_registry
                .configs
                .insert(PluginId::raw(id.as_str()), config.to_owned());
        }

        ProjectBuilder::new(
            &self.id,
            &self.source,
            ProjectBuilderContext {
                config_loader: &self.config_loader,
                enabled_toolchains: &self.enabled_toolchains,
                monorepo: true,
                root_project_id: None,
                toolchain_config: &self.toolchain_config,
                toolchain_registry: Arc::new(toolchain_registry),
                workspace_root: &self.workspace_root,
            },
        )
        .unwrap()
    }
}

async fn build_project(id: &str, root: &Path) -> Project {
    let stub = Stub::new(id, root);

    let manager = ConfigLoader::default()
        .load_tasks_manager_from(root, root.join("global"))
        .unwrap();

    let mut builder = stub.create_builder().await;
    builder.load_local_config().await.unwrap();
    builder.inherit_global_config(&manager).unwrap();
    builder.build().await.unwrap()
}

async fn build_project_without_inherited(id: &str, root: &Path) -> Project {
    let stub = Stub::new(id, root);

    let mut builder = stub.create_builder().await;
    builder.load_local_config().await.unwrap();
    builder.build().await.unwrap()
}

async fn build_lang_project(id: &str) -> Project {
    let sandbox = create_sandbox("langs");
    let stub = Stub::new(id, sandbox.path());

    let mut builder = stub.create_builder().await;
    builder.load_local_config().await.unwrap();
    builder.build().await.unwrap()
}

mod project_builder {
    use super::*;

    #[tokio::test]
    async fn sets_common_fields() {
        let sandbox = create_sandbox("builder");
        let project = build_project_without_inherited("baz", sandbox.path()).await;

        assert_eq!(project.id, Id::raw("baz"));
        assert_eq!(project.source, WorkspaceRelativePathBuf::from("baz"));
        assert_eq!(project.root, sandbox.path().join("baz"));
    }

    #[tokio::test]
    async fn builds_depends_on() {
        let sandbox = create_sandbox("builder");
        let project = build_project_without_inherited("baz", sandbox.path()).await;

        assert_eq!(
            project.dependencies,
            vec![
                DependencyConfig {
                    id: "foo".try_into().unwrap(),
                    source: DependencySource::Explicit,
                    scope: DependencyScope::Development,
                    ..Default::default()
                },
                DependencyConfig {
                    id: "bar".try_into().unwrap(),
                    source: DependencySource::Explicit,
                    ..Default::default()
                },
            ]
        );
    }

    // Tasks are tested heavily in the tasks-builder crate
    #[tokio::test]
    async fn builds_tasks() {
        let sandbox = create_sandbox("builder");
        let a = build_project("foo", sandbox.path()).await;
        let b = build_project("bar", sandbox.path()).await;
        let c = build_project("baz", sandbox.path()).await;

        assert_eq!(a.tasks.len(), 4);
        assert_eq!(b.tasks.len(), 3);
        assert_eq!(c.tasks.len(), 5);
    }

    mod file_groups {
        use super::*;

        #[tokio::test]
        async fn inherits_from_global_when_no_local() {
            let sandbox = create_sandbox("builder");
            let project = build_project("foo", sandbox.path()).await;

            assert_eq!(
                project.file_groups,
                BTreeMap::from_iter([
                    (
                        "sources".try_into().unwrap(),
                        FileGroup::new_with_source(
                            "sources",
                            [WorkspaceRelativePathBuf::from("foo/node")]
                        )
                        .unwrap()
                    ),
                    (
                        "tests".try_into().unwrap(),
                        FileGroup::new_with_source(
                            "tests",
                            [WorkspaceRelativePathBuf::from("foo/global")]
                        )
                        .unwrap()
                    ),
                    (
                        "other".try_into().unwrap(),
                        FileGroup::new_with_source(
                            "other",
                            [WorkspaceRelativePathBuf::from("foo/global")]
                        )
                        .unwrap()
                    )
                ])
            );
        }

        #[tokio::test]
        async fn inherits_from_global_but_local_overrides() {
            let sandbox = create_sandbox("builder");
            let project = build_project("bar", sandbox.path()).await;

            assert_eq!(
                project.file_groups,
                BTreeMap::from_iter([
                    (
                        "sources".try_into().unwrap(),
                        FileGroup::new_with_source(
                            "sources",
                            // Not node since the language is rust
                            [WorkspaceRelativePathBuf::from("bar/global")]
                        )
                        .unwrap()
                    ),
                    (
                        "tests".try_into().unwrap(),
                        FileGroup::new_with_source(
                            "tests",
                            [WorkspaceRelativePathBuf::from("bar/global")]
                        )
                        .unwrap()
                    ),
                    (
                        "other".try_into().unwrap(),
                        FileGroup::new_with_source(
                            "other",
                            [WorkspaceRelativePathBuf::from("bar/bar")]
                        )
                        .unwrap()
                    )
                ])
            );
        }
    }

    mod language_detect {
        use super::*;

        #[tokio::test]
        async fn inherits_from_config() {
            let sandbox = create_sandbox("builder");
            let project = build_project_without_inherited("bar", sandbox.path()).await;

            assert_eq!(project.language, LanguageType::Rust);
        }

        #[tokio::test]
        async fn detects_from_env() {
            let sandbox = create_sandbox("builder");
            let project = build_project_without_inherited("qux", sandbox.path()).await;

            assert_eq!(project.language, LanguageType::TypeScript);
        }

        #[tokio::test]
        async fn detects_bash() {
            let project = build_lang_project("bash").await;

            assert_eq!(project.language, LanguageType::Bash);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test]
        async fn detects_batch() {
            let project = build_lang_project("batch").await;

            assert_eq!(project.language, LanguageType::Batch);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test]
        async fn detects_bun() {
            let project = build_lang_project("bun").await;

            assert_eq!(project.language, LanguageType::JavaScript);
            assert_eq!(project.toolchains, vec![Id::raw("bun")]);

            let project = build_lang_project("bun-config").await;

            assert_eq!(project.language, LanguageType::JavaScript);
            assert_eq!(project.toolchains, vec![Id::raw("bun")]);
        }

        #[tokio::test]
        async fn detects_deno() {
            let project = build_lang_project("deno").await;

            assert_eq!(project.language, LanguageType::JavaScript);
            assert_eq!(project.toolchains, vec![Id::raw("deno")]);

            let project = build_lang_project("deno-config").await;

            assert_eq!(project.language, LanguageType::TypeScript);
            assert_eq!(
                project.toolchains,
                vec![Id::raw("deno"), Id::raw("typescript")]
            );
        }

        #[tokio::test]
        async fn detects_go() {
            let project = build_lang_project("go").await;

            assert_eq!(project.language, LanguageType::Go);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);

            let project = build_lang_project("go-config").await;

            assert_eq!(project.language, LanguageType::Go);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test]
        async fn detects_js() {
            let project = build_lang_project("js").await;

            assert_eq!(project.language, LanguageType::JavaScript);
            assert_eq!(project.toolchains, vec![Id::raw("node")]);

            let project = build_lang_project("js-config").await;

            assert_eq!(project.language, LanguageType::JavaScript);
            assert_eq!(project.toolchains, vec![Id::raw("node")]);
        }

        #[tokio::test]
        async fn detects_other() {
            let project = build_lang_project("other").await;

            assert_eq!(
                project.language,
                LanguageType::Other("kotlin".try_into().unwrap())
            );
            assert_eq!(project.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test]
        async fn detects_php() {
            let project = build_lang_project("php").await;

            assert_eq!(project.language, LanguageType::Php);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);

            let project = build_lang_project("php-config").await;

            assert_eq!(project.language, LanguageType::Php);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test]
        async fn detects_python() {
            let project = build_lang_project("python").await;

            assert_eq!(project.language, LanguageType::Python);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);

            let project = build_lang_project("python-config").await;

            assert_eq!(project.language, LanguageType::Python);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test]
        async fn detects_ruby() {
            let project = build_lang_project("ruby").await;

            assert_eq!(project.language, LanguageType::Ruby);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);

            let project = build_lang_project("ruby-config").await;

            assert_eq!(project.language, LanguageType::Ruby);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);
        }

        #[tokio::test]
        async fn detects_rust() {
            let project = build_lang_project("rust").await;

            assert_eq!(project.language, LanguageType::Rust);
            assert_eq!(project.toolchains, vec![Id::raw("rust")]);

            let project = build_lang_project("rust-config").await;

            assert_eq!(project.language, LanguageType::Rust);
            assert_eq!(project.toolchains, vec![Id::raw("rust")]);
        }

        #[tokio::test]
        async fn detects_ts() {
            let project = build_lang_project("ts").await;

            assert_eq!(project.language, LanguageType::TypeScript);
            assert_eq!(project.toolchains, vec![Id::raw("typescript")]);

            let project = build_lang_project("ts-config").await;

            assert_eq!(project.language, LanguageType::TypeScript);
            assert_eq!(project.toolchains, vec![Id::raw("typescript")]);

            let project = build_lang_project("ts-enabled").await;

            assert_eq!(project.language, LanguageType::Unknown);
            assert_eq!(
                project.toolchains,
                vec![Id::raw("typescript"), Id::raw("system")]
            );

            let project = build_lang_project("ts-disabled").await;

            assert_eq!(project.language, LanguageType::TypeScript);
            assert_eq!(project.toolchains, vec![Id::raw("system")]);
        }
    }

    mod detect_toolchain {
        use super::*;

        #[tokio::test]
        async fn inherits_from_config() {
            let sandbox = create_sandbox("builder");
            let project = build_project_without_inherited("baz", sandbox.path()).await;

            assert_eq!(project.toolchains, vec![Id::raw("node")]);
        }

        #[tokio::test]
        async fn infers_from_config_lang() {
            let sandbox = create_sandbox("builder");
            let project = build_project_without_inherited("bar", sandbox.path()).await;

            assert_eq!(project.toolchains, vec![Id::raw("rust")]);
        }

        #[tokio::test]
        async fn infers_from_detected_lang() {
            let sandbox = create_sandbox("builder");
            let project = build_project_without_inherited("foo", sandbox.path()).await;

            assert_eq!(project.toolchains, vec![Id::raw("node")]);
        }

        #[tokio::test]
        async fn fallsback_to_project() {
            let project = build_lang_project("project-platform").await;

            assert_eq!(
                project.tasks.get("node-a").unwrap().toolchains,
                vec![Id::raw("node")]
            );

            assert_eq!(
                project.tasks.get("node-b").unwrap().toolchains,
                vec![Id::raw("node")]
            );

            assert_eq!(
                project.tasks.get("system").unwrap().toolchains,
                vec![Id::raw("system")]
            );
        }
    }

    mod graph_extending {
        use super::*;

        #[tokio::test]
        async fn inherits_dep() {
            let sandbox = create_sandbox("builder");
            let stub = Stub::new("bar", sandbox.path());

            let mut builder = stub.create_builder().await;
            builder.load_local_config().await.unwrap();

            builder.extend_with_dependency(DependencyConfig {
                id: "foo".try_into().unwrap(),
                scope: DependencyScope::Development,
                ..DependencyConfig::default()
            });

            let project = builder.build().await.unwrap();

            assert_eq!(
                project.dependencies,
                vec![DependencyConfig {
                    id: "foo".try_into().unwrap(),
                    scope: DependencyScope::Development,
                    source: DependencySource::Implicit,
                    ..DependencyConfig::default()
                }]
            );
        }

        #[tokio::test]
        async fn inherits_task() {
            let sandbox = create_sandbox("builder");
            let stub = Stub::new("bar", sandbox.path());

            let mut builder = stub.create_builder().await;
            builder.load_local_config().await.unwrap();

            builder.extend_with_task(
                Id::raw("task"),
                TaskConfig {
                    ..TaskConfig::default()
                },
            );

            let project = builder.build().await.unwrap();

            assert!(project.tasks.contains_key("task"));
        }

        #[tokio::test]
        async fn doesnt_override_task_of_same_id() {
            let sandbox = create_sandbox("builder");
            let stub = Stub::new("baz", sandbox.path());

            let mut builder = stub.create_builder().await;
            builder.load_local_config().await.unwrap();

            builder.extend_with_task(
                Id::raw("baz"),
                TaskConfig {
                    command: TaskArgs::String("new-command-name".into()),
                    ..TaskConfig::default()
                },
            );

            let project = builder.build().await.unwrap();

            assert!(project.tasks.contains_key("baz"));
            assert_eq!(project.tasks.get("baz").unwrap().command, "baz");
        }
    }
}
