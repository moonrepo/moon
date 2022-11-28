use moon_config::GlobalProjectConfig;
use moon_node_platform::actions::sync_project;
use moon_test_utils::{create_sandbox_with_config, get_node_fixture_configs};

#[tokio::test]
async fn creates_tsconfig() {
    let (workspace_config, toolchain_config, projects_config) = get_node_fixture_configs();
    let sandbox = create_sandbox_with_config(
        "node",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let project = Project::new(
        "deps-a",
        "deps-a",
        sandbox.path(),
        &GlobalProjectConfig::default(),
    )
    .unwrap();

    let tsconfig_path = project.root.join("tsconfig.json");

    assert!(!tsconfig_path.exists());

    create_missing_tsconfig(&project, &TypeScriptConfig::default(), sandbox.path())
        .await
        .unwrap();

    assert!(tsconfig_path.exists());

    let tsconfig = TsConfigJson::read(tsconfig_path).unwrap().unwrap();

    assert_eq!(
        tsconfig.extends,
        Some("../tsconfig.options.json".to_owned())
    );
    assert_eq!(tsconfig.include, Some(string_vec!["**/*"]));
}

#[tokio::test]
async fn creates_tsconfig_with_custom_settings() {
    let (workspace_config, toolchain_config, projects_config) = get_node_fixture_configs();
    let sandbox = create_sandbox_with_config(
        "node",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let project = Project::new(
        "deps-a",
        "deps-a",
        sandbox.path(),
        &GlobalProjectConfig::default(),
    )
    .unwrap();

    let tsconfig_path = project.root.join("tsconfig.ref.json");

    assert!(!tsconfig_path.exists());

    create_missing_tsconfig(
        &project,
        &TypeScriptConfig {
            project_config_file_name: "tsconfig.ref.json".to_string(),
            root_options_config_file_name: "tsconfig.base.json".to_string(),
            ..TypeScriptConfig::default()
        },
        sandbox.path(),
    )
    .await
    .unwrap();

    assert!(tsconfig_path.exists());

    let tsconfig = TsConfigJson::read_with_name(&project.root, "tsconfig.ref.json")
        .unwrap()
        .unwrap();

    assert_eq!(tsconfig.extends, Some("../tsconfig.base.json".to_owned()));
    assert_eq!(tsconfig.include, Some(string_vec!["**/*"]));
}

#[tokio::test]
async fn doesnt_create_if_a_config_exists() {
    let (workspace_config, toolchain_config, projects_config) = get_node_fixture_configs();
    let sandbox = create_sandbox_with_config(
        "node",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let project = Project::new(
        "deps-b",
        "deps-b",
        sandbox.path(),
        &GlobalProjectConfig::default(),
    )
    .unwrap();

    let tsconfig_path = project.root.join("tsconfig.json");

    assert!(tsconfig_path.exists());

    let created = create_missing_tsconfig(&project, &TypeScriptConfig::default(), sandbox.path())
        .await
        .unwrap();

    assert!(!created);
}
