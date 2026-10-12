use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// How many hops away from a node a graph walk may travel.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Depth {
    /// Up to this many hops away. 0 = none, 1 = direct.
    Limit(u32),
    /// Unbounded.
    Deep,
}

impl Depth {
    pub const NONE: Self = Self::Limit(0);
    pub const DIRECT: Self = Self::Limit(1);

    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }

    pub fn is_deep(&self) -> bool {
        *self == Self::Deep
    }

    /// Whether a node `distance` hops away (1 = direct neighbour) is within this depth.
    pub fn includes(&self, distance: u32) -> bool {
        match self {
            Self::Limit(limit) => distance <= *limit,
            Self::Deep => true,
        }
    }
}

impl Default for Depth {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("invalid depth `{0}`, expected `none`, `direct`, `deep`, or an unsigned integer")]
pub struct ParseDepthError(pub String);

impl FromStr for Depth {
    type Err = ParseDepthError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();

        if value.eq_ignore_ascii_case("none") {
            Ok(Self::NONE)
        } else if value.eq_ignore_ascii_case("direct") {
            Ok(Self::DIRECT)
        } else if value.eq_ignore_ascii_case("deep") {
            Ok(Self::Deep)
        } else {
            value
                .parse::<u32>()
                .map(Self::Limit)
                .map_err(|_| ParseDepthError(value.to_owned()))
        }
    }
}

impl fmt::Display for Depth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit(0) => write!(f, "none"),
            Self::Limit(1) => write!(f, "direct"),
            Self::Limit(limit) => write!(f, "{limit}"),
            Self::Deep => write!(f, "deep"),
        }
    }
}

impl Serialize for Depth {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Limit(0) => serializer.serialize_str("none"),
            Self::Limit(1) => serializer.serialize_str("direct"),
            Self::Limit(limit) => serializer.serialize_u32(*limit),
            Self::Deep => serializer.serialize_str("deep"),
        }
    }
}

struct DepthVisitor;

impl Visitor<'_> for DepthVisitor {
    type Value = Depth;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("`none`, `direct`, `deep`, or an unsigned integer")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Depth::from_str(value).map_err(de::Error::custom)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u32::try_from(value)
            .map(Depth::Limit)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Unsigned(value), &self))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u32::try_from(value)
            .map(Depth::Limit)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Signed(value), &self))
    }
}

impl<'de> Deserialize<'de> for Depth {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DepthVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_utils::json::serde_json;

    #[test]
    fn parses_keywords() {
        assert_eq!(Depth::from_str("none").unwrap(), Depth::NONE);
        assert_eq!(Depth::from_str("direct").unwrap(), Depth::DIRECT);
        assert_eq!(Depth::from_str("deep").unwrap(), Depth::Deep);
        assert_eq!(Depth::from_str(" DeEp ").unwrap(), Depth::Deep);
        assert_eq!(Depth::from_str("NONE").unwrap(), Depth::Limit(0));
    }

    #[test]
    fn parses_numbers() {
        assert_eq!(Depth::from_str("0").unwrap(), Depth::NONE);
        assert_eq!(Depth::from_str("1").unwrap(), Depth::DIRECT);
        assert_eq!(Depth::from_str(" 3 ").unwrap(), Depth::Limit(3));
        assert_eq!(
            Depth::from_str("4294967295").unwrap(),
            Depth::Limit(u32::MAX)
        );
    }

    #[test]
    fn errors_for_invalid() {
        for value in ["", "-1", "1.5", "deeper", "4294967296"] {
            assert_eq!(
                Depth::from_str(value).unwrap_err(),
                ParseDepthError(value.to_owned())
            );
        }
    }

    #[test]
    fn displays() {
        assert_eq!(Depth::NONE.to_string(), "none");
        assert_eq!(Depth::DIRECT.to_string(), "direct");
        assert_eq!(Depth::Limit(2).to_string(), "2");
        assert_eq!(Depth::Deep.to_string(), "deep");
    }

    #[test]
    fn display_round_trips() {
        for depth in [Depth::NONE, Depth::DIRECT, Depth::Limit(7), Depth::Deep] {
            assert_eq!(Depth::from_str(&depth.to_string()).unwrap(), depth);
        }
    }

    #[test]
    fn defaults_to_none() {
        assert_eq!(Depth::default(), Depth::NONE);
        assert!(Depth::default().is_none());
    }

    #[test]
    fn is_checks() {
        assert!(Depth::NONE.is_none());
        assert!(!Depth::DIRECT.is_none());
        assert!(!Depth::Deep.is_none());
        assert!(Depth::Deep.is_deep());
        assert!(!Depth::Limit(u32::MAX).is_deep());
    }

    #[test]
    fn includes() {
        assert!(!Depth::NONE.includes(1));
        assert!(Depth::DIRECT.includes(1));
        assert!(!Depth::DIRECT.includes(2));
        assert!(Depth::Limit(2).includes(2));
        assert!(!Depth::Limit(2).includes(3));
        assert!(Depth::Deep.includes(1));
        assert!(Depth::Deep.includes(u32::MAX));
    }

    #[test]
    fn serializes() {
        assert_eq!(serde_json::to_string(&Depth::NONE).unwrap(), "\"none\"");
        assert_eq!(serde_json::to_string(&Depth::DIRECT).unwrap(), "\"direct\"");
        assert_eq!(serde_json::to_string(&Depth::Deep).unwrap(), "\"deep\"");
        assert_eq!(serde_json::to_string(&Depth::Limit(3)).unwrap(), "3");
    }

    #[test]
    fn deserializes() {
        let parse = |value: &str| serde_json::from_str::<Depth>(value);

        assert_eq!(parse("\"none\"").unwrap(), Depth::NONE);
        assert_eq!(parse("\"Direct\"").unwrap(), Depth::DIRECT);
        assert_eq!(parse("\"deep\"").unwrap(), Depth::Deep);
        assert_eq!(parse("\"5\"").unwrap(), Depth::Limit(5));
        assert_eq!(parse("0").unwrap(), Depth::NONE);
        assert_eq!(parse("1").unwrap(), Depth::DIRECT);
        assert_eq!(parse("5").unwrap(), Depth::Limit(5));

        assert!(parse("-1").is_err());
        assert!(parse("1.5").is_err());
        assert!(parse("4294967296").is_err());
        assert!(parse("\"deeper\"").is_err());
        assert!(parse("true").is_err());
        assert!(parse("null").is_err());
    }

    #[test]
    fn serde_round_trips() {
        for depth in [
            Depth::NONE,
            Depth::DIRECT,
            Depth::Limit(2),
            Depth::Limit(u32::MAX),
            Depth::Deep,
        ] {
            let json = serde_json::to_string(&depth).unwrap();

            assert_eq!(serde_json::from_str::<Depth>(&json).unwrap(), depth);
        }
    }

    #[test]
    fn deserializes_within_struct() {
        #[derive(Deserialize)]
        struct Wrapper {
            depth: Option<Depth>,
        }

        let wrapper: Wrapper = serde_json::from_str(r#"{"depth": 2}"#).unwrap();
        assert_eq!(wrapper.depth, Some(Depth::Limit(2)));

        let wrapper: Wrapper = serde_json::from_str(r#"{"depth": null}"#).unwrap();
        assert_eq!(wrapper.depth, None);
    }
}
