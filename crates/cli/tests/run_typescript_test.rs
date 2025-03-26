use moon_test_utils::{
    Sandbox, assert_snapshot, create_sandbox_with_config, get_typescript_fixture_configs,
};
use starbase_utils::json::JsonValue;
use std::collections::BTreeMap;
use std::fs::{self, read_to_string};

fn typescript_sandbox<C>(callback: C) -> Sandbox
where
    C: FnOnce(&mut BTreeMap<String, JsonValue>),
{
    let (workspace_config, mut toolchain_config, tasks_config) = get_typescript_fixture_configs();

    if let Some(tc) = &mut toolchain_config.plugins {
        if let Some(ts_config) = tc.get_mut("typescript") {
            callback(ts_config.config.get_or_insert_default());
        }
    }

    let sandbox = create_sandbox_with_config(
        "typescript",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

#[test]
fn creates_missing_tsconfig() {
    let sandbox = typescript_sandbox(|cfg| {
        cfg.insert("createMissingConfig".into(), JsonValue::Bool(true));
    });

    assert!(!sandbox.path().join("create-config/tsconfig.json").exists());

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("create-config:noop");
    });

    assert!(sandbox.path().join("create-config/tsconfig.json").exists());

    // root
    assert_snapshot!(read_to_string(sandbox.path().join("tsconfig.json")).unwrap());

    // project
    assert_snapshot!(read_to_string(sandbox.path().join("create-config/tsconfig.json")).unwrap());
}

#[test]
fn doesnt_create_missing_tsconfig_if_setting_off() {
    let sandbox = typescript_sandbox(|cfg| {
        cfg.insert("createMissingConfig".into(), JsonValue::Bool(false));
    });

    assert!(!sandbox.path().join("create-config/tsconfig.json").exists());

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("create-config:noop");
    });

    assert!(!sandbox.path().join("create-config/tsconfig.json").exists());
}

#[test]
fn doesnt_create_missing_tsconfig_if_syncing_off() {
    let sandbox = typescript_sandbox(|cfg| {
        cfg.insert("createMissingConfig".into(), JsonValue::Bool(true));
        cfg.insert("syncProjectReferences".into(), JsonValue::Bool(false));
    });

    assert!(!sandbox.path().join("create-config/tsconfig.json").exists());

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("create-config:noop");
    });

    assert!(!sandbox.path().join("create-config/tsconfig.json").exists());
}

#[test]
fn doesnt_create_missing_tsconfig_if_project_disabled() {
    let sandbox = typescript_sandbox(|cfg| {
        cfg.insert("createMissingConfig".into(), JsonValue::Bool(true));
        cfg.insert("syncProjectReferences".into(), JsonValue::Bool(true));
    });

    assert!(!sandbox.path().join("create-config/tsconfig.json").exists());

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("create-config-disabled:noop");
    });

    assert!(!sandbox.path().join("create-config/tsconfig.json").exists());
}

mod refs {
    use super::*;

    #[test]
    fn syncs_ref_to_root_config() {
        let sandbox = typescript_sandbox(|_| {});

        let initial_root = read_to_string(sandbox.path().join("tsconfig.json")).unwrap();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("create-config:noop");
        });

        let synced_root = read_to_string(sandbox.path().join("tsconfig.json")).unwrap();

        assert_ne!(initial_root, synced_root);
        assert_snapshot!(synced_root);
    }

    #[test]
    fn syncs_depends_on_as_refs() {
        let sandbox = typescript_sandbox(|_| {});

        assert!(
            !sandbox
                .path()
                .join("syncs-deps-refs/tsconfig.json")
                .exists()
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("syncs-deps-refs:noop");
        });

        // should not have `deps-no-config-disabled` or `deps-with-config-disabled`
        assert_snapshot!(
            read_to_string(sandbox.path().join("syncs-deps-refs/tsconfig.json")).unwrap()
        );
    }

    #[test]
    fn syncs_root_project_if_root_config_custom_name() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert(
                "rootConfigFileName".into(),
                JsonValue::String("tsconfig.root.json".into()),
            );
        });

        fs::write(sandbox.path().join("tsconfig.root.json"), "{}").unwrap();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("root:noop");
        });

        assert_snapshot!(read_to_string(sandbox.path().join("tsconfig.root.json")).unwrap());
    }

    #[test]
    fn doesnt_sync_root_project() {
        let sandbox = typescript_sandbox(|_| {});

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("root:noop");
        });

        assert_snapshot!(read_to_string(sandbox.path().join("tsconfig.json")).unwrap());
    }

    #[test]
    fn doesnt_sync_depends_on_as_refs_if_disabled() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert("syncProjectReferences".into(), JsonValue::Bool(false));
        });

        assert!(
            !sandbox
                .path()
                .join("syncs-deps-refs/tsconfig.json")
                .exists()
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("syncs-deps-refs:noop");
        });

        assert!(
            !sandbox
                .path()
                .join("syncs-deps-refs/tsconfig.json")
                .exists()
        );
    }

    #[test]
    fn doesnt_sync_depends_on_as_refs_if_disabled_in_project() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert("syncProjectReferences".into(), JsonValue::Bool(true));
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("syncs-deps-refs-project-disabled:noop");
        });

        // should not have anything
        assert_snapshot!(
            read_to_string(
                sandbox
                    .path()
                    .join("syncs-deps-refs-project-disabled/tsconfig.json")
            )
            .unwrap()
        );
    }
}

mod out_dir {
    use super::*;

    #[test]
    fn routes_to_cache() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert("routeOutDirToCache".into(), JsonValue::Bool(true));
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("out-dir-routing:noop");
        });

        assert_snapshot!(
            read_to_string(sandbox.path().join("out-dir-routing/tsconfig.json")).unwrap()
        );
    }

    #[test]
    fn routes_to_cache_when_no_compiler_options() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert("routeOutDirToCache".into(), JsonValue::Bool(true));
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("out-dir-routing-no-options:noop");
        });

        assert_snapshot!(
            read_to_string(
                sandbox
                    .path()
                    .join("out-dir-routing-no-options/tsconfig.json")
            )
            .unwrap()
        );
    }

    #[test]
    fn doesnt_route_to_cache_if_disabled() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert("routeOutDirToCache".into(), JsonValue::Bool(false));
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("out-dir-routing:noop");
        });

        assert_snapshot!(
            read_to_string(sandbox.path().join("out-dir-routing/tsconfig.json")).unwrap()
        );
    }

    #[test]
    fn doesnt_route_to_cache_if_disabled_in_project() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert("routeOutDirToCache".into(), JsonValue::Bool(true));
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("out-dir-routing-project-disabled:noop");
        });

        assert_snapshot!(
            read_to_string(
                sandbox
                    .path()
                    .join("out-dir-routing-project-disabled/tsconfig.json")
            )
            .unwrap()
        );
    }
}

mod paths {
    use super::*;

    #[test]
    fn maps_paths() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert("syncProjectReferencesToPaths".into(), JsonValue::Bool(true));
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("sync").arg("projects");
        });

        assert_snapshot!(
            read_to_string(sandbox.path().join("syncs-paths-refs/tsconfig.json")).unwrap()
        );
    }

    #[test]
    fn doesnt_map_paths_if_no_refs() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert("syncProjectReferences".into(), JsonValue::Bool(false));
            cfg.insert("syncProjectReferencesToPaths".into(), JsonValue::Bool(true));
        });

        std::fs::remove_file(sandbox.path().join("syncs-paths-refs/moon.yml")).unwrap();

        sandbox.run_moon(|cmd| {
            cmd.arg("sync").arg("projects");
        });

        assert_snapshot!(
            read_to_string(sandbox.path().join("syncs-paths-refs/tsconfig.json")).unwrap()
        );
    }

    #[test]
    fn doesnt_map_paths_if_disabled() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert(
                "syncProjectReferencesToPaths".into(),
                JsonValue::Bool(false),
            );
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("sync").arg("projects");
        });

        assert_snapshot!(
            read_to_string(sandbox.path().join("syncs-paths-refs/tsconfig.json")).unwrap()
        );
    }

    #[test]
    fn doesnt_map_paths_if_disabled_in_project() {
        let sandbox = typescript_sandbox(|cfg| {
            cfg.insert("syncProjectReferencesToPaths".into(), JsonValue::Bool(true));
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("sync").arg("projects");
        });

        assert_snapshot!(
            read_to_string(
                sandbox
                    .path()
                    .join("syncs-paths-refs-project-disabled/tsconfig.json")
            )
            .unwrap()
        );
    }
}
