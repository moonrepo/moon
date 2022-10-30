use moon_error::MoonError;
use moon_utils::semver::Version;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum DependencyPathError {
    #[error("<symbol>{0}</symbol> is an invalid relative dependency path")]
    IsNotAbsolute(String),
}

impl From<DependencyPathError> for MoonError {
    fn from(error: DependencyPathError) -> Self {
        Self::Generic(error.to_string())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PnpmDependencyPath {
    pub host: Option<String>,
    pub is_absolute: bool,
    pub name: Option<String>,
    pub peers_suffix: Option<String>,
    pub version: Option<String>,
}

// Ported from
// https://github.com/pnpm/pnpm/blob/75ac5ca2e63101817b7c02144083641a5274c182/packages/dependency-path/src/index.ts
impl PnpmDependencyPath {
    fn is_absolute(path: &str) -> bool {
        !path.starts_with('/')
    }

    pub fn parse(path: &str) -> Result<Self, DependencyPathError> {
        let _is_absolute = Self::is_absolute(path);
        let mut parts = path.split('/').map(String::from).collect::<Vec<_>>();
        if !_is_absolute {
            parts.remove(0);
        }
        let host = if _is_absolute {
            Some(parts.remove(0))
        } else {
            None
        };
        let name = if parts[0].starts_with('@') {
            Some(format!("{}/{}", parts.remove(0), parts.remove(0)))
        } else {
            Some(parts.remove(0))
        };
        let version = if parts.is_empty() {
            None
        } else {
            Some(parts.remove(0))
        };
        if let Some(mut ver) = version {
            let underscore_index = ver.find('_');
            let mut peers_suffix = None;
            if let Some(index) = underscore_index {
                peers_suffix = Some(ver[index + 1..].to_string());
                ver = ver[..index].to_string()
            }
            if Version::parse(&ver).is_ok() {
                return Ok(Self {
                    host,
                    is_absolute: _is_absolute,
                    name,
                    peers_suffix,
                    version: Some(ver),
                });
            }
        }
        if !_is_absolute {
            return Err(DependencyPathError::IsNotAbsolute(path.to_string()));
        }
        Ok(Self {
            host,
            is_absolute: _is_absolute,
            name: None,
            peers_suffix: None,
            version: None,
        })
    }
}

// Ported from
// https://github.com/pnpm/pnpm/blob/75ac5ca2e63101817b7c02144083641a5274c182/packages/dependency-path/test/index.ts
#[cfg(test)]
mod tests {
    use super::PnpmDependencyPath;

    #[test]
    fn parses_basic() {
        assert_eq!(
            PnpmDependencyPath::parse("/foo/1.0.0"),
            Ok(PnpmDependencyPath {
                host: None,
                is_absolute: false,
                name: Some("foo".to_string()),
                peers_suffix: None,
                version: Some("1.0.0".to_string())
            })
        );
    }

    #[test]
    fn parses_scoped() {
        assert_eq!(
            PnpmDependencyPath::parse("/@foo/bar/1.0.0"),
            Ok(PnpmDependencyPath {
                host: None,
                is_absolute: false,
                name: Some("@foo/bar".to_string()),
                peers_suffix: None,
                version: Some("1.0.0".to_string()),
            })
        );
    }

    #[test]
    fn parses_with_registry() {
        assert_eq!(
            PnpmDependencyPath::parse("registry.npmjs.org/foo/1.0.0"),
            Ok(PnpmDependencyPath {
                host: Some("registry.npmjs.org".to_string()),
                is_absolute: true,
                name: Some("foo".to_string()),
                peers_suffix: None,
                version: Some("1.0.0".to_string()),
            })
        );
    }

    #[test]
    fn parses_from_github_() {
        assert_eq!(
            PnpmDependencyPath::parse("github.com/kevva/is-positive"),
            Ok(PnpmDependencyPath {
                host: Some("github.com".to_string()),
                is_absolute: true,
                name: None,
                peers_suffix: None,
                version: None,
            })
        );
    }

    #[test]
    fn parses_from_custom_regsitry_with_peer_deps() {
        assert_eq!(
            PnpmDependencyPath::parse("example.com/foo/1.0.0_bar@2.0.0"),
            Ok(PnpmDependencyPath {
                host: Some("example.com".to_string()),
                is_absolute: true,
                name: Some("foo".to_string()),
                peers_suffix: Some("bar@2.0.0".to_string()),
                version: Some("1.0.0".to_string()),
            })
        );
    }

    #[test]
    fn parses_with_peer_deps() {
        assert_eq!(
            PnpmDependencyPath::parse("/foo/1.0.0_@types+babel__core@7.1.14"),
            Ok(PnpmDependencyPath {
                host: None,
                is_absolute: false,
                name: Some("foo".to_string()),
                peers_suffix: Some("@types+babel__core@7.1.14".to_string()),
                version: Some("1.0.0".to_string()),
            })
        );
        assert!(PnpmDependencyPath::parse("/foo/bar").is_err());
    }
}
