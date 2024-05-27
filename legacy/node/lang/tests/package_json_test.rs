use moon_node_lang::*;
use moon_test_utils::{assert_fs::prelude::*, create_temp_dir};
use std::collections::BTreeMap;

#[test]
fn preserves_when_saving() {
    let json = "{\n  \"name\": \"hello\",\n  \"description\": \"world\",\n  \"private\": true\n}\n";

    let dir = create_temp_dir();
    let file = dir.child("package.json");
    file.write_str(json).unwrap();

    let mut package = PackageJsonCache::read(dir.path()).unwrap().unwrap();

    // Trigger dirty
    package.dirty.push("unknown".into());

    package.save().unwrap();

    assert_eq!(std::fs::read_to_string(file.path()).unwrap(), json);
}

mod add_dependency {
    use super::*;

    #[test]
    fn adds_if_not_set() {
        let mut pkg = PackageJsonCache::default();

        assert_eq!(pkg.data.dependencies, None);

        assert!(pkg.add_dependency("foo", "1.2.3", false));

        assert_eq!(pkg.data.dependencies.unwrap().get("foo").unwrap(), &"1.2.3");
    }

    #[test]
    fn adds_if_not_set_and_missing_true() {
        let mut pkg = PackageJsonCache::default();

        assert_eq!(pkg.data.dependencies, None);

        assert!(pkg.add_dependency("foo", "1.2.3", true));

        assert_eq!(pkg.data.dependencies.unwrap().get("foo").unwrap(), &"1.2.3");
    }

    #[test]
    fn adds_if_set() {
        let mut pkg = PackageJsonCache {
            data: PackageJson {
                dependencies: Some(BTreeMap::from([("foo".to_owned(), "1.2.3".to_owned())])),
                ..PackageJson::default()
            },
            ..Default::default()
        };

        assert!(pkg.add_dependency("foo", "4.5.6", false));

        assert_eq!(pkg.data.dependencies.unwrap().get("foo").unwrap(), &"4.5.6");
    }

    #[test]
    fn doesnt_add_if_set_and_missing_true() {
        let mut pkg = PackageJsonCache {
            data: PackageJson {
                dependencies: Some(BTreeMap::from([("foo".to_owned(), "1.2.3".to_owned())])),
                ..PackageJson::default()
            },
            ..Default::default()
        };

        assert!(!pkg.add_dependency("foo", "4.5.6", true));

        assert_eq!(pkg.data.dependencies.unwrap().get("foo").unwrap(), &"1.2.3");
    }
}

mod add_engine {
    use nodejs_package_json::EnginesMap;

    use super::*;

    #[test]
    fn adds_if_not_set() {
        let mut pkg = PackageJsonCache::default();

        assert_eq!(pkg.data.engines, None);

        assert!(pkg.add_engine("node", "1.2.3"));

        assert_eq!(pkg.data.engines.unwrap().get("node").unwrap(), &"1.2.3");
    }

    #[test]
    fn adds_if_set() {
        let mut pkg = PackageJsonCache {
            data: PackageJson {
                engines: Some(EnginesMap::from_iter([(
                    "node".to_owned(),
                    "1.2.3".to_owned(),
                )])),
                ..PackageJson::default()
            },
            ..Default::default()
        };

        assert!(pkg.add_engine("node", "4.5.6"));

        assert_eq!(pkg.data.engines.unwrap().get("node").unwrap(), &"4.5.6");
    }

    #[test]
    fn returns_false_for_same_value() {
        let mut pkg = PackageJsonCache {
            data: PackageJson {
                engines: Some(EnginesMap::from_iter([(
                    "node".to_owned(),
                    "1.2.3".to_owned(),
                )])),
                ..PackageJson::default()
            },
            ..Default::default()
        };

        assert!(!pkg.add_engine("node", "1.2.3"));
    }
}

mod set_package_manager {
    use super::*;

    #[test]
    fn adds_if_not_set() {
        let mut pkg = PackageJsonCache::default();

        assert_eq!(pkg.data.package_manager, None);

        assert!(pkg.set_package_manager("npm@1.2.3"));

        assert_eq!(pkg.data.package_manager.unwrap(), "npm@1.2.3".to_owned());
    }

    #[test]
    fn adds_if_set() {
        let mut pkg = PackageJsonCache {
            data: PackageJson {
                package_manager: Some(String::from("npm@1.2.3")),
                ..PackageJson::default()
            },
            ..Default::default()
        };

        assert!(pkg.set_package_manager("npm@4.5.6"));

        assert_eq!(pkg.data.package_manager.unwrap(), "npm@4.5.6".to_owned());
    }

    #[test]
    fn returns_false_for_same_value() {
        let mut pkg = PackageJsonCache {
            data: PackageJson {
                package_manager: Some(String::from("npm@1.2.3")),
                ..PackageJson::default()
            },
            ..Default::default()
        };

        assert!(!pkg.set_package_manager("npm@1.2.3"));
    }
}
