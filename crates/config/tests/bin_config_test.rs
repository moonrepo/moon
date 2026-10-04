mod utils;

use moon_config::{BinConfig, BinEntry, config_struct};
use schematic::{Config, ConfigLoader as BaseLoader};
use utils::*;

// Mirrors how toolchain plugins configure their binaries
config_struct!(
    #[derive(Config)]
    struct BinsConfig {
        #[setting(nested)]
        pub bins: Vec<BinEntry>,
    }
);

fn load_config_from_code(code: &str) -> miette::Result<BinsConfig> {
    Ok(BaseLoader::<BinsConfig>::new()
        .code(code, "config.yml")?
        .load()?
        .config)
}

mod bin_config {
    use super::*;

    #[test]
    fn supports_strings() {
        let config = test_parse_config("bins: ['diesel_cli@2.3.9']", load_config_from_code);

        assert_eq!(
            config.bins,
            vec![BinEntry::String("diesel_cli@2.3.9".into())]
        );
    }

    #[test]
    fn supports_objects() {
        let config = test_parse_config(
            r"
bins:
  - bin: 'diesel_cli@2.3.9'
    force: true
",
            load_config_from_code,
        );

        assert_eq!(
            config.bins,
            vec![BinEntry::Object(BinConfig {
                bin: "diesel_cli@2.3.9".into(),
                force: true,
                ..Default::default()
            })]
        );
    }

    #[test]
    fn supports_args() {
        let config = test_parse_config(
            r"
bins:
  - bin: 'diesel_cli@2.3.9'
    args: ['--no-default-features', '--features', 'postgres']
",
            load_config_from_code,
        );

        assert_eq!(
            config.bins,
            vec![BinEntry::Object(BinConfig {
                args: vec![
                    "--no-default-features".into(),
                    "--features".into(),
                    "postgres".into()
                ],
                bin: "diesel_cli@2.3.9".into(),
                ..Default::default()
            })]
        );
    }

    #[test]
    #[should_panic(expected = "bin: must not be empty")]
    fn errors_if_bin_empty() {
        test_parse_config(
            r"
bins:
  - bin: ''
    args: ['--locked']
",
            load_config_from_code,
        );
    }
}
