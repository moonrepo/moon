use moon_common::Id;
use moon_config::{
    ProjectConfig, ProjectToolchainEntry, ToolchainPluginConfig, ToolchainsConfig,
    UnresolvedVersionSpec,
};
use moon_toolchain::{ToolchainSpec, get_toolchain_spec};

fn create_version(version: &str) -> UnresolvedVersionSpec {
    UnresolvedVersionSpec::parse(version).unwrap()
}

fn create_plugin_config(version: Option<&str>) -> ToolchainPluginConfig {
    ToolchainPluginConfig {
        version: version.map(create_version),
        ..Default::default()
    }
}

fn create_toolchains_config(id: &str, version: Option<&str>) -> ToolchainsConfig {
    let mut config = ToolchainsConfig::default();

    config
        .plugins
        .insert(Id::raw(id), create_plugin_config(version));

    config
}

fn create_project_config(id: &str, entry: ProjectToolchainEntry) -> ProjectConfig {
    let mut config = ProjectConfig::default();

    config.toolchains.plugins.insert(Id::raw(id), entry);

    config
}

mod get_toolchain_spec {
    use super::*;

    mod workspace {
        use super::*;

        #[test]
        fn returns_none_if_not_configured() {
            let config = create_toolchains_config("node", Some("20.0.0"));

            assert_eq!(get_toolchain_spec(&config, &Id::raw("rust"), None), None);
        }

        #[test]
        fn returns_global_if_no_version() {
            let config = create_toolchains_config("rust", None);

            assert_eq!(
                get_toolchain_spec(&config, &Id::raw("rust"), None),
                Some(ToolchainSpec::new_global(Id::raw("rust")))
            );
        }

        #[test]
        fn returns_configured_version() {
            let config = create_toolchains_config("rust", Some("1.90.0"));

            assert_eq!(
                get_toolchain_spec(&config, &Id::raw("rust"), None),
                Some(ToolchainSpec::new(
                    Id::raw("rust"),
                    create_version("1.90.0")
                ))
            );
        }

        #[test]
        fn matches_unstable_ids() {
            let config = create_toolchains_config("unstable_python", Some("3.12.0"));

            assert_eq!(
                get_toolchain_spec(&config, &Id::raw("python"), None),
                Some(ToolchainSpec::new(
                    Id::raw("python"),
                    create_version("3.12.0")
                ))
            );
        }
    }

    mod project {
        use super::*;

        #[test]
        fn inherits_workspace_if_no_entry() {
            let config = create_toolchains_config("rust", Some("1.90.0"));
            let project_config = create_project_config(
                "node",
                ProjectToolchainEntry::Object(create_plugin_config(Some("20.0.0"))),
            );

            assert_eq!(
                get_toolchain_spec(&config, &Id::raw("rust"), Some(&project_config)),
                Some(ToolchainSpec::new(
                    Id::raw("rust"),
                    create_version("1.90.0")
                ))
            );
        }

        #[test]
        fn overrides_version() {
            let config = create_toolchains_config("rust", Some("1.90.0"));
            let project_config = create_project_config(
                "rust",
                ProjectToolchainEntry::Object(create_plugin_config(Some("1.99.0"))),
            );

            assert_eq!(
                get_toolchain_spec(&config, &Id::raw("rust"), Some(&project_config)),
                Some(ToolchainSpec::new(
                    Id::raw("rust"),
                    create_version("1.99.0")
                ))
            );
        }

        #[test]
        fn overrides_version_if_not_configured() {
            let config = ToolchainsConfig::default();
            let project_config = create_project_config(
                "rust",
                ProjectToolchainEntry::Object(create_plugin_config(Some("1.99.0"))),
            );

            assert_eq!(
                get_toolchain_spec(&config, &Id::raw("rust"), Some(&project_config)),
                Some(ToolchainSpec::new(
                    Id::raw("rust"),
                    create_version("1.99.0")
                ))
            );
        }

        #[test]
        fn inherits_workspace_if_entry_has_no_version() {
            let config = create_toolchains_config("rust", Some("1.90.0"));

            for entry in [
                ProjectToolchainEntry::Enabled(true),
                ProjectToolchainEntry::Object(create_plugin_config(None)),
            ] {
                let project_config = create_project_config("rust", entry);

                assert_eq!(
                    get_toolchain_spec(&config, &Id::raw("rust"), Some(&project_config)),
                    Some(ToolchainSpec::new(
                        Id::raw("rust"),
                        create_version("1.90.0")
                    ))
                );
            }
        }

        #[test]
        fn returns_none_if_disabled() {
            let config = create_toolchains_config("rust", Some("1.90.0"));

            for entry in [
                ProjectToolchainEntry::Disabled,
                ProjectToolchainEntry::Enabled(false),
            ] {
                let project_config = create_project_config("rust", entry);

                assert_eq!(
                    get_toolchain_spec(&config, &Id::raw("rust"), Some(&project_config)),
                    None
                );
            }
        }

        #[test]
        fn matches_unstable_ids() {
            let config = create_toolchains_config("unstable_python", Some("3.12.0"));
            let project_config = create_project_config(
                "python",
                ProjectToolchainEntry::Object(create_plugin_config(Some("3.13.0"))),
            );

            assert_eq!(
                get_toolchain_spec(&config, &Id::raw("unstable_python"), Some(&project_config)),
                Some(ToolchainSpec::new(
                    Id::raw("unstable_python"),
                    create_version("3.13.0")
                ))
            );
        }
    }
}
