mod utils;

use moon_config::{ConfigLoader, ExtensionPluginConfig, ExtensionsConfig};
use proto_core::warpgate::{PluginLocator, UrlLocator};
use rustc_hash::FxHashMap;
use std::path::Path;
use utils::*;

const FILENAME: &str = ".moon/extensions.yml";

fn load_config_from_root(root: &Path) -> miette::Result<ExtensionsConfig> {
    ConfigLoader::new(root.join(".moon")).load_extensions_config(root)
}

mod extensions_config {
    use super::*;

    //     #[test]
    //     #[should_panic(expected = "test-id.plugin: Missing plugin protocol.")]
    //     fn errors_invalid_locator() {
    //         test_load_config(
    //             FILENAME,
    //             r"
    // test-id:
    //   plugin: 'missing-scope'
    // ",
    //             load_config_from_root,
    //         );
    //     }

    #[test]
    #[should_panic(expected = "test-id.plugin: a locator is required for plugins")]
    fn errors_missing_locator() {
        test_load_config(
            FILENAME,
            r"
test-id:
  foo: 'bar'
",
            load_config_from_root,
        );
    }

    #[test]
    fn can_set_with_object() {
        let config = test_load_config(
            FILENAME,
            r"
test-id:
  plugin: 'https://domain.com'
",
            load_config_from_root,
        );

        assert_eq!(
            config.plugins.get("test-id").unwrap(),
            &ExtensionPluginConfig {
                config: FxHashMap::default(),
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://domain.com".into()
                }))),
            }
        );
    }

    #[test]
    fn can_set_additional_object_config() {
        let config = test_load_config(
            FILENAME,
            r"
test-id:
  plugin: 'https://domain.com'
  fooBar: 'abc'
  bar-baz: true
",
            load_config_from_root,
        );

        assert_eq!(
            config.plugins.get("test-id").unwrap(),
            &ExtensionPluginConfig {
                config: FxHashMap::from_iter([
                    ("fooBar".into(), serde_json::Value::String("abc".into())),
                    ("bar-baz".into(), serde_json::Value::Bool(true)),
                ]),
                plugin: Some(PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://domain.com".into()
                }))),
            }
        );
    }

    #[test]
    fn supports_hcl() {
        load_extensions_config_in_format("hcl");
    }

    #[test]
    fn supports_pkl() {
        load_extensions_config_in_format("pkl");
    }

    #[test]
    fn supports_toml() {
        load_extensions_config_in_format("toml");
    }
}
