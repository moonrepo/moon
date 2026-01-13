mod graph_context;
mod graph_formats;
mod graph_traits;

pub use graph_context::*;
pub use graph_formats::*;
pub use graph_traits::*;

use serde::de::{self, Visitor};
use serde::de::{Deserialize, Deserializer};
use std::fmt;
use std::marker::PhantomData;

#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum NodeState<T> {
    Loaded(T),
    Loading,
}

// By default, if any of the variants of an untagged enum fail to deserialize, serde
// will return a generic error.
//
// This custom implementation of Deserialize for NodeState<T> tells serde to pass through the
// specific deserialization errors from T, which provides more context when deserialization fails.
impl<'de, T: Deserialize<'de>> Deserialize<'de> for NodeState<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NodeStateVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for NodeStateVisitor<T> {
            type Value = NodeState<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a value that can be deserialized into NodeState")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NodeState::Loading)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NodeState::Loading)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                T::deserialize(deserializer).map(NodeState::Loaded)
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::BoolDeserializer::new(v)).map(NodeState::Loaded)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::I64Deserializer::new(v)).map(NodeState::Loaded)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::U64Deserializer::new(v)).map(NodeState::Loaded)
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::F64Deserializer::new(v)).map(NodeState::Loaded)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::StrDeserializer::new(v)).map(NodeState::Loaded)
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::StringDeserializer::new(v)).map(NodeState::Loaded)
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::BytesDeserializer::new(v)).map(NodeState::Loaded)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                T::deserialize(de::value::SeqAccessDeserializer::new(seq)).map(NodeState::Loaded)
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                T::deserialize(de::value::MapAccessDeserializer::new(map)).map(NodeState::Loaded)
            }
        }

        deserializer.deserialize_any(NodeStateVisitor(PhantomData))
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use super::NodeState;
    use rustc_hash::FxHashMap;
    use serde::{
        Deserialize,
        de::{Deserializer, Error},
    };
    use starbase_utils::json::serde_json;

    pub fn custom_err<'de, D, T>(_deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        Err(D::Error::custom("fails with custom error"))
    }

    fn run_custom_err_test<T: Debug + Deserialize<'static>>(val_str: &'static str) {
        #[derive(Deserialize, Debug)]
        struct TestData<T: Debug + Deserialize<'static>> {
            #[allow(dead_code)]
            #[serde(deserialize_with = "custom_err")]
            value: T,
        }

        let s = format!(r#"{{"value": {}}}"#, val_str);
        let result: Result<NodeState<TestData<T>>, _> = serde_json::from_str(&s);

        let Err(e) = result else {
            panic!("Expected error but got success!");
        };
        assert!(e.to_string().contains("fails with custom error"));
    }

    #[test]
    fn test_deserialize_i32_custom_err() {
        run_custom_err_test::<i32>("1");
    }

    #[test]
    fn test_deserialize_u32_custom_err() {
        run_custom_err_test::<u32>("1");
    }

    #[test]
    fn test_deserialize_f64_custom_err() {
        run_custom_err_test::<f64>("1.0");
    }

    #[test]
    fn test_deserialize_bool_custom_err() {
        run_custom_err_test::<bool>("true");
    }

    #[test]
    fn test_deserialize_string_custom_err() {
        run_custom_err_test::<String>(r#""test""#);
    }

    #[test]
    fn test_deserialize_vec_custom_err() {
        run_custom_err_test::<Vec<i32>>("[]");
    }

    #[test]
    fn test_deserialize_bytes_custom_err() {
        run_custom_err_test::<Vec<u8>>("\"abcd\"");
    }

    // Success state tests
    #[test]
    fn test_deserialize_loading_from_null() {
        let result: NodeState<String> = serde_json::from_str("null").unwrap();
        assert!(matches!(result, NodeState::Loading));
    }

    #[test]
    fn test_deserialize_loaded_bool() {
        let result: NodeState<bool> = serde_json::from_str("true").unwrap();
        assert!(matches!(result, NodeState::Loaded(true)));
    }

    #[test]
    fn test_deserialize_loaded_i64() {
        let result: NodeState<i64> = serde_json::from_str("42").unwrap();
        assert!(matches!(result, NodeState::Loaded(42)));
    }

    #[test]
    fn test_deserialize_loaded_u64() {
        let result: NodeState<u64> = serde_json::from_str("100").unwrap();
        assert!(matches!(result, NodeState::Loaded(100)));
    }

    #[test]
    fn test_deserialize_loaded_f64() {
        let result: NodeState<f64> = serde_json::from_str("3.12").unwrap();
        assert!(matches!(result, NodeState::Loaded(v) if (v - 3.12).abs() < 0.001));
    }

    #[test]
    fn test_deserialize_loaded_string() {
        let result: NodeState<String> = serde_json::from_str(r#""hello""#).unwrap();
        assert!(matches!(result, NodeState::Loaded(ref s) if s == "hello"));
    }

    #[test]
    fn test_deserialize_loaded_vec() {
        let result: NodeState<Vec<i32>> = serde_json::from_str("[1, 2, 3]").unwrap();
        assert!(matches!(result, NodeState::Loaded(ref v) if v == &vec![1, 2, 3]));
    }

    #[test]
    fn test_deserialize_loaded_map() {
        let result: NodeState<FxHashMap<String, i32>> =
            serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
        if let NodeState::Loaded(map) = result {
            assert_eq!(map.get("a"), Some(&1));
            assert_eq!(map.get("b"), Some(&2));
        } else {
            panic!("Expected Loaded state");
        }
    }

    #[test]
    fn test_serialize_loading() {
        let state: NodeState<String> = NodeState::Loading;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "null");
    }

    #[test]
    fn test_serialize_loaded() {
        let state = NodeState::Loaded("test".to_string());
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, r#""test""#);
    }
}
