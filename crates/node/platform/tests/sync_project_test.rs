use moon_config::{InheritedTasksManager, ProjectLanguage, TypeScriptConfig};
use moon_node_platform::actions::create_missing_tsconfig;
use moon_project::Project;
use moon_test_utils::{create_sandbox_with_config, get_node_fixture_configs};
use moon_typescript_lang::tsconfig::TsConfigExtends;
use moon_typescript_lang::TsConfigJson;
use moon_utils::string_vec;

mod missing_tsconfig {
    use super::*;

    #[test]
    fn creates_tsconfig() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();
        let sandbox = create_sandbox_with_config(
            "node",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        let project = Project::new(
            "deps-a",
            "deps-a",
            sandbox.path(),
            &InheritedTasksManager::default(),
            |_| ProjectLanguage::Unknown,
        )
        .unwrap();

        let tsconfig_path = project.root.join("tsconfig.json");

        assert!(!tsconfig_path.exists());

        create_missing_tsconfig(&project, &TypeScriptConfig::default(), sandbox.path()).unwrap();

        assert!(tsconfig_path.exists());

        let tsconfig = TsConfigJson::read(tsconfig_path).unwrap().unwrap();

        assert_eq!(
            tsconfig.extends,
            Some(TsConfigExtends::String(
                "../tsconfig.options.json".to_owned()
            ))
        );
        assert_eq!(tsconfig.include, Some(string_vec!["**/*"]));
    }

    #[test]
    fn creates_tsconfig_with_custom_settings() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();
        let sandbox = create_sandbox_with_config(
            "node",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        let project = Project::new(
            "deps-a",
            "deps-a",
            sandbox.path(),
            &InheritedTasksManager::default(),
            |_| ProjectLanguage::Unknown,
        )
        .unwrap();

        let tsconfig_path = project.root.join("tsconfig.ref.json");

        assert!(!tsconfig_path.exists());

        create_missing_tsconfig(
            &project,
            &TypeScriptConfig {
                project_config_file_name: Some("tsconfig.ref.json".to_string()),
                root_options_config_file_name: Some("tsconfig.base.json".to_string()),
                ..TypeScriptConfig::default()
            },
            sandbox.path(),
        )
        .unwrap();

        assert!(tsconfig_path.exists());

        let tsconfig = TsConfigJson::read_with_name(&project.root, Some("tsconfig.ref.json"))
            .unwrap()
            .unwrap();

        assert_eq!(
            tsconfig.extends,
            Some(TsConfigExtends::String("../tsconfig.base.json".to_owned()))
        );
        assert_eq!(tsconfig.include, Some(string_vec!["**/*"]));
    }

    #[test]
    fn doesnt_create_if_a_config_exists() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();
        let sandbox = create_sandbox_with_config(
            "node",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        let project = Project::new(
            "deps-b",
            "deps-b",
            sandbox.path(),
            &InheritedTasksManager::default(),
            |_| ProjectLanguage::Unknown,
        )
        .unwrap();

        let tsconfig_path = project.root.join("tsconfig.json");

        assert!(tsconfig_path.exists());

        let created =
            create_missing_tsconfig(&project, &TypeScriptConfig::default(), sandbox.path())
                .unwrap();

        assert!(!created);
    }
}
