use moon::load_workspace_from;
use moon_config::{
    InheritedTasksConfig, InheritedTasksManager, PlatformType, ProjectLanguage, ProjectType,
    TaskCommandArgs, TaskConfig,
};
use moon_test_utils::create_sandbox;
use moon_utils::string_vec;
use rustc_hash::FxHashMap;

fn mock_task(command: &str, platform: PlatformType) -> TaskConfig {
    TaskConfig {
        command: Some(TaskCommandArgs::String(command.to_owned())),
        inputs: if command == "global" {
            None
        } else {
            Some(string_vec![format!("/.moon/tasks/{command}.yml")])
        },
        platform,
        ..TaskConfig::default()
    }
}

fn mock_tasks_config(command: &str) -> InheritedTasksConfig {
    let mut config = InheritedTasksConfig::default();
    config.tasks.insert(
        command.to_owned(),
        TaskConfig {
            command: Some(TaskCommandArgs::String(command.to_owned())),
            ..TaskConfig::default()
        },
    );
    config
}

#[tokio::test]
async fn loads_all_task_configs_into_manager() {
    let sandbox = create_sandbox("config-inheritance/files");
    let workspace = load_workspace_from(sandbox.path()).await.unwrap();

    assert_eq!(
        workspace.tasks_config.configs,
        FxHashMap::from_iter([
            ("*".into(), mock_tasks_config("global")),
            ("deno".into(), mock_tasks_config("deno")),
            ("node".into(), mock_tasks_config("node")),
            (
                "node-application".into(),
                mock_tasks_config("node-application")
            ),
            ("node-library".into(), mock_tasks_config("node-library")),
            ("javascript".into(), mock_tasks_config("javascript")),
            (
                "javascript-tool".into(),
                mock_tasks_config("javascript-tool")
            ),
            (
                "javascript-library".into(),
                mock_tasks_config("javascript-library")
            ),
            ("rust".into(), mock_tasks_config("rust")),
            ("typescript".into(), mock_tasks_config("typescript")),
        ])
    );
}

mod lookup_order {
    use super::*;

    #[test]
    fn includes_js() {
        let manager = InheritedTasksManager::default();

        assert_eq!(
            manager.get_lookup_order(
                PlatformType::Node,
                ProjectLanguage::JavaScript,
                ProjectType::Application
            ),
            string_vec![
                "*",
                "node",
                "javascript",
                "node-application",
                "javascript-application"
            ]
        );
    }

    #[test]
    fn includes_ts() {
        let manager = InheritedTasksManager::default();

        assert_eq!(
            manager.get_lookup_order(
                PlatformType::Node,
                ProjectLanguage::TypeScript,
                ProjectType::Library
            ),
            string_vec![
                "*",
                "node",
                "typescript",
                "node-library",
                "typescript-library"
            ]
        );
    }

    #[test]
    fn supports_other_langs() {
        let manager = InheritedTasksManager::default();

        assert_eq!(
            manager.get_lookup_order(
                PlatformType::Unknown,
                ProjectLanguage::Ruby,
                ProjectType::Tool
            ),
            string_vec!["*", "ruby", "ruby-tool"]
        );

        assert_eq!(
            manager.get_lookup_order(
                PlatformType::Unknown,
                ProjectLanguage::Rust,
                ProjectType::Application
            ),
            string_vec!["*", "rust", "rust-application"]
        );
    }
}

mod config_merging {
    use super::*;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn creates_js_config() {
        let sandbox = create_sandbox("config-inheritance/files");
        let workspace = load_workspace_from(sandbox.path()).await.unwrap();

        assert_eq!(
            workspace.tasks_config.get_inherited_config(
                PlatformType::Node,
                ProjectLanguage::JavaScript,
                ProjectType::Application
            ),
            InheritedTasksConfig {
                implicit_inputs: string_vec!["/.moon/*.yml"],
                tasks: BTreeMap::from_iter([
                    ("global".into(), mock_task("global", PlatformType::Unknown)),
                    ("node".into(), mock_task("node", PlatformType::Node)),
                    (
                        "node-application".into(),
                        mock_task("node-application", PlatformType::Node)
                    ),
                    (
                        "javascript".into(),
                        mock_task("javascript", PlatformType::Node)
                    ),
                ]),
                ..InheritedTasksConfig::default()
            }
        );
    }

    #[tokio::test]
    async fn creates_ts_config() {
        let sandbox = create_sandbox("config-inheritance/files");
        let workspace = load_workspace_from(sandbox.path()).await.unwrap();

        assert_eq!(
            workspace.tasks_config.get_inherited_config(
                PlatformType::Node,
                ProjectLanguage::TypeScript,
                ProjectType::Tool
            ),
            InheritedTasksConfig {
                implicit_inputs: string_vec!["/.moon/*.yml"],
                tasks: BTreeMap::from_iter([
                    ("global".into(), mock_task("global", PlatformType::Unknown)),
                    ("node".into(), mock_task("node", PlatformType::Node)),
                    (
                        "typescript".into(),
                        mock_task("typescript", PlatformType::Node)
                    ),
                ]),
                ..InheritedTasksConfig::default()
            }
        );
    }

    #[tokio::test]
    async fn creates_rust_config() {
        let sandbox = create_sandbox("config-inheritance/files");
        let workspace = load_workspace_from(sandbox.path()).await.unwrap();

        assert_eq!(
            workspace.tasks_config.get_inherited_config(
                PlatformType::System,
                ProjectLanguage::Rust,
                ProjectType::Library
            ),
            InheritedTasksConfig {
                implicit_inputs: string_vec!["/.moon/*.yml"],
                tasks: BTreeMap::from_iter([
                    ("global".into(), mock_task("global", PlatformType::Unknown)),
                    ("rust".into(), mock_task("rust", PlatformType::System)),
                ]),
                ..InheritedTasksConfig::default()
            }
        );
    }

    #[tokio::test]
    async fn entirely_overrides_task_of_same_name() {
        let sandbox = create_sandbox("config-inheritance/override");
        let workspace = load_workspace_from(sandbox.path()).await.unwrap();

        let mut task = mock_task("node-library", PlatformType::Node);
        task.inputs = Some(string_vec!["c", "/.moon/tasks/node-library.yml"]);

        assert_eq!(
            workspace.tasks_config.get_inherited_config(
                PlatformType::Node,
                ProjectLanguage::JavaScript,
                ProjectType::Library
            ),
            InheritedTasksConfig {
                implicit_inputs: string_vec!["/.moon/*.yml"],
                tasks: BTreeMap::from_iter([("command".into(), task)]),
                ..InheritedTasksConfig::default()
            }
        );
    }
}

mod config_loading {
    use super::*;
    use moon::generate_project_graph;

    #[tokio::test]
    async fn inherits_when_lang_is_detected() {
        let sandbox = create_sandbox("config-inheritance/detection");
        let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
        let graph = generate_project_graph(&mut workspace).await.unwrap();

        graph.get("explicit").unwrap().get_task("command").unwrap();

        let task = graph.get("explicit").unwrap().get_task("command").unwrap();

        assert_eq!(task.command, "node");
        assert_eq!(
            task.inputs,
            string_vec!["/.moon/tasks/node.yml", "**/*", "/.moon/*.yml"]
        );

        let task = graph.get("detected").unwrap().get_task("command").unwrap();

        assert_eq!(task.command, "node");
        assert_eq!(
            task.inputs,
            string_vec!["/.moon/tasks/node.yml", "**/*", "/.moon/*.yml"]
        );

        let task = graph.get("other").unwrap().get_task("command").unwrap();

        assert_eq!(task.command, "global");
        assert_eq!(task.inputs, string_vec!["**/*", "/.moon/*.yml"]);
    }

    #[tokio::test]
    async fn inherits_correct_task_platform() {
        let sandbox = create_sandbox("config-inheritance/platform");
        let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
        let graph = generate_project_graph(&mut workspace).await.unwrap();

        let project = graph.get("project").unwrap();

        assert_eq!(
            project.tasks.get("global").unwrap().platform,
            PlatformType::System
        );
        assert_eq!(
            project.tasks.get("node").unwrap().platform,
            PlatformType::Node
        );
        assert_eq!(
            project.tasks.get("node-detected").unwrap().platform,
            PlatformType::Node
        );
        assert_eq!(
            project.tasks.get("system-via-node").unwrap().platform,
            PlatformType::System
        );
    }
}
