// Most of this code is ported from
// https://github.com/pnpm/pnpm/blob/75ac5ca2e63101817b7c02144083641a5274c182/packages/dependency-path/test/index.ts
// (and the corresponding library). All credits go to original authors.

use moon_utils::semver::Version;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PnpmDependencyPath {
    pub host: Option<String>,
    pub is_absolute: bool,
    pub name: Option<String>,
    pub peers_suffix: Option<String>,
    pub version: Option<String>,
}

impl PnpmDependencyPath {
    fn is_absolute(path: &str) -> bool {
        !path.starts_with('/')
    }

    pub fn parse(path: &str) -> Self {
        let is_absolute = Self::is_absolute(path);
        let mut parts = path.split('/').map(String::from).collect::<Vec<_>>();

        if !is_absolute {
            parts.remove(0);
        }

        let host = if is_absolute {
            Some(parts.remove(0))
        } else {
            None
        };

        if parts.is_empty() {
            return PnpmDependencyPath {
                host,
                is_absolute,
                ..PnpmDependencyPath::default()
            };
        }

        let name = if parts[0].starts_with('@') {
            Some(format!("{}/{}", parts.remove(0), parts.remove(0)))
        } else {
            Some(parts.remove(0))
        };

        let version = if parts.is_empty() {
            None
        } else {
            Some(parts.join("/"))
        };

        if let Some(mut ver) = version {
            let mut peers_suffix = None;

            if ver.contains('(') && ver.ends_with(')') {
                if let Some(index) = ver.find('(') {
                    peers_suffix = Some(ver[index..].to_string());
                    ver = ver[0..index].to_string();
                }
            } else if let Some(index) = ver.find('_') {
                peers_suffix = Some(ver[index + 1..].to_string());
                ver = ver[0..index].to_string();
            }

            if Version::parse(&ver).is_ok() {
                return PnpmDependencyPath {
                    host,
                    is_absolute,
                    name,
                    peers_suffix,
                    version: Some(ver),
                };
            }
        }

        PnpmDependencyPath {
            host,
            is_absolute,
            ..PnpmDependencyPath::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_absolute_paths_correctly() {
        assert!(!PnpmDependencyPath::is_absolute("/foo/1.0.0"));
        assert!(PnpmDependencyPath::is_absolute(
            "registry.npmjs.org/foo/1.0.0"
        ));
    }

    #[test]
    fn parses_basic() {
        assert_eq!(
            PnpmDependencyPath::parse("/foo/1.0.0"),
            PnpmDependencyPath {
                host: None,
                is_absolute: false,
                name: Some("foo".to_string()),
                peers_suffix: None,
                version: Some("1.0.0".to_string())
            }
        );
    }

    #[test]
    fn parses_file() {
        assert_eq!(
            PnpmDependencyPath::parse("file:project(foo@1.0.0)"),
            PnpmDependencyPath {
                host: Some("file:project(foo@1.0.0)".to_string()),
                is_absolute: true,
                name: None,
                peers_suffix: None,
                version: None,
            }
        );
    }

    #[test]
    fn parses_scoped() {
        assert_eq!(
            PnpmDependencyPath::parse("/@foo/bar/1.0.0"),
            PnpmDependencyPath {
                host: None,
                is_absolute: false,
                name: Some("@foo/bar".to_string()),
                peers_suffix: None,
                version: Some("1.0.0".to_string()),
            }
        );
    }

    #[test]
    fn parses_with_registry() {
        assert_eq!(
            PnpmDependencyPath::parse("registry.npmjs.org/foo/1.0.0"),
            PnpmDependencyPath {
                host: Some("registry.npmjs.org".to_string()),
                is_absolute: true,
                name: Some("foo".to_string()),
                peers_suffix: None,
                version: Some("1.0.0".to_string()),
            }
        );
    }

    #[test]
    fn parses_with_registry_with_scope() {
        assert_eq!(
            PnpmDependencyPath::parse("registry.npmjs.org/@foo/bar/1.0.0"),
            PnpmDependencyPath {
                host: Some("registry.npmjs.org".to_string()),
                is_absolute: true,
                name: Some("@foo/bar".to_string()),
                peers_suffix: None,
                version: Some("1.0.0".to_string()),
            }
        );
    }

    #[test]
    fn parses_from_github() {
        assert_eq!(
            PnpmDependencyPath::parse("github.com/kevva/is-positive"),
            PnpmDependencyPath {
                host: Some("github.com".to_string()),
                is_absolute: true,
                name: None,
                peers_suffix: None,
                version: None,
            }
        );
    }

    #[test]
    fn parses_from_custom_registry_with_peer_deps() {
        assert_eq!(
            PnpmDependencyPath::parse("example.com/foo/1.0.0_bar@2.0.0"),
            PnpmDependencyPath {
                host: Some("example.com".to_string()),
                is_absolute: true,
                name: Some("foo".to_string()),
                peers_suffix: Some("bar@2.0.0".to_string()),
                version: Some("1.0.0".to_string()),
            }
        );
    }

    #[test]
    fn parses_with_peer_deps() {
        assert_eq!(
            PnpmDependencyPath::parse("/foo/1.0.0_@types+babel__core@7.1.14"),
            PnpmDependencyPath {
                host: None,
                is_absolute: false,
                name: Some("foo".to_string()),
                peers_suffix: Some("@types+babel__core@7.1.14".to_string()),
                version: Some("1.0.0".to_string()),
            }
        );
        assert_eq!(
            PnpmDependencyPath::parse("/foo/1.0.0(@types/babel__core@7.1.14)(foo@1.0.0)"),
            PnpmDependencyPath {
                host: None,
                is_absolute: false,
                name: Some("foo".to_string()),
                peers_suffix: Some("(@types/babel__core@7.1.14)(foo@1.0.0)".to_string()),
                version: Some("1.0.0".to_string()),
            }
        );
    }

    #[test]
    fn parses_with_peer_deps_new() {
        assert_eq!(
            PnpmDependencyPath::parse("example.com/foo/1.0.0(bar@2.0.0)"),
            PnpmDependencyPath {
                host: Some("example.com".to_string()),
                is_absolute: true,
                name: Some("foo".to_string()),
                peers_suffix: Some("(bar@2.0.0)".to_string()),
                version: Some("1.0.0".to_string()),
            }
        );
    }

    #[test]
    fn parses_with_peer_deps_new_multi() {
        assert_eq!(
            PnpmDependencyPath::parse("/foo/1.0.0(@types/babel__core@7.1.14)(foo@1.0.0)"),
            PnpmDependencyPath {
                host: None,
                is_absolute: false,
                name: Some("foo".to_string()),
                peers_suffix: Some("(@types/babel__core@7.1.14)(foo@1.0.0)".to_string()),
                version: Some("1.0.0".to_string()),
            }
        );
    }

    #[test]
    fn parses_with_peer_deps_hash() {
        assert_eq!(
            PnpmDependencyPath::parse("/ts-node/10.9.1_xl7wyiapi7jo5c2pfz5vjm55na"),
            PnpmDependencyPath {
                host: None,
                is_absolute: false,
                name: Some("ts-node".to_string()),
                peers_suffix: Some("xl7wyiapi7jo5c2pfz5vjm55na".to_string()),
                version: Some("10.9.1".to_string()),
            }
        );
    }
}
