use schematic::schema::UnionType;
use schematic::{Schema, SchemaBuilder, Schematic};
use serde::de::{self, Deserialize, Deserializer, Visitor};
use std::fmt;
use std::marker::PhantomData;

/// Represents a single value, or a list of multiple values.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(untagged)]
pub enum OneOrMany<T: Schematic> {
    One(T),
    Many(Vec<T>),
}

// By default, if any of the variants of an untagged enum fail to deserialize, serde
// will return a generic error.
//
// This custom implementation of Deserialize for OneOrMany<T> tells serde to pass through the
// specific deserialization errors from T, which provides more context when deserialization fails.
impl<'de, T: Schematic + Deserialize<'de>> Deserialize<'de> for OneOrMany<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OneOrManyVisitor<T>(PhantomData<T>);

        impl<'de, T: Schematic + Deserialize<'de>> Visitor<'de> for OneOrManyVisitor<T> {
            type Value = OneOrMany<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("expected a single value, or a list of values")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::BoolDeserializer::new(v)).map(OneOrMany::One)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::I64Deserializer::new(v)).map(OneOrMany::One)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::U64Deserializer::new(v)).map(OneOrMany::One)
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::F64Deserializer::new(v)).map(OneOrMany::One)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::StrDeserializer::new(v)).map(OneOrMany::One)
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::StringDeserializer::new(v)).map(OneOrMany::One)
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(de::value::BytesDeserializer::new(v)).map(OneOrMany::One)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                Vec::<T>::deserialize(de::value::SeqAccessDeserializer::new(seq))
                    .map(OneOrMany::Many)
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                T::deserialize(de::value::MapAccessDeserializer::new(map)).map(OneOrMany::One)
            }
        }

        deserializer.deserialize_any(OneOrManyVisitor(PhantomData))
    }
}

impl<T: Schematic> Default for OneOrMany<T> {
    fn default() -> Self {
        Self::Many(vec![])
    }
}

impl<T: Schematic + Clone + PartialEq> OneOrMany<T> {
    pub fn matches(&self, value: &T) -> bool {
        match self {
            Self::One(item) => item == value,
            Self::Many(list) => list.contains(value),
        }
    }
}

impl<T: Schematic + Clone> OneOrMany<T> {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::One(_) => false,
            Self::Many(list) => list.is_empty(),
        }
    }

    pub fn to_list(&self) -> Vec<&T> {
        match self {
            Self::One(item) => vec![item],
            Self::Many(list) => list.iter().collect(),
        }
    }

    pub fn to_owned_list(&self) -> Vec<T> {
        match self {
            Self::One(item) => vec![item.to_owned()],
            Self::Many(list) => list.to_owned(),
        }
    }
}

impl<T: Schematic> Schematic for OneOrMany<T> {
    fn schema_name() -> Option<String> {
        None
    }

    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.union(UnionType::new_any([
            schema.infer::<T>(),
            schema.infer::<Vec<T>>(),
        ]))
    }
}

#[cfg(test)]
mod test {
    use super::OneOrMany;
    use schematic::Schematic;
    use serde::{
        Deserialize,
        de::{Deserializer, Error},
    };
    use serde_json;
    use std::fmt::Debug;

    pub fn custom_err<'de, D, T>(_deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        Err(D::Error::custom("fails with custom error"))
    }

    #[derive(Deserialize, Debug)]
    struct TestData<T: Debug + Deserialize<'static>> {
        #[allow(dead_code)]
        #[serde(deserialize_with = "custom_err")]
        value: T,
    }

    impl<T: Schematic + Debug + Deserialize<'static>> Schematic for TestData<T> {}

    fn run_custom_err_test<T: Debug + Deserialize<'static> + Schematic>(val_str: &'static str) {
        let s = format!(r#"{{"value": {}}}"#, val_str);
        let result: Result<OneOrMany<TestData<T>>, _> = serde_json::from_str(&s);

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
    fn test_deserialize_one_bool() {
        let result: OneOrMany<bool> = serde_json::from_str("true").unwrap();
        assert!(matches!(result, OneOrMany::One(true)));
    }

    #[test]
    fn test_deserialize_one_i64() {
        let result: OneOrMany<i64> = serde_json::from_str("42").unwrap();
        assert!(matches!(result, OneOrMany::One(42)));
    }

    #[test]
    fn test_deserialize_one_u64() {
        let result: OneOrMany<u64> = serde_json::from_str("100").unwrap();
        assert!(matches!(result, OneOrMany::One(100)));
    }

    #[test]
    fn test_deserialize_one_f64() {
        let result: OneOrMany<f64> = serde_json::from_str("5.55").unwrap();
        assert!(matches!(result, OneOrMany::One(v) if (v - 5.55).abs() < 0.001));
    }

    #[test]
    fn test_deserialize_one_string() {
        let result: OneOrMany<String> = serde_json::from_str(r#""hello""#).unwrap();
        assert!(matches!(result, OneOrMany::One(ref s) if s == "hello"));
    }

    #[test]
    fn test_deserialize_many_vec() {
        let result: OneOrMany<i32> = serde_json::from_str("[1, 2, 3]").unwrap();
        assert!(matches!(result, OneOrMany::Many(ref v) if v == &vec![1, 2, 3]));
    }

    #[test]
    fn test_deserialize_one_map() {
        let result: OneOrMany<std::collections::HashMap<String, i32>> =
            serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
        if let OneOrMany::One(map) = result {
            assert_eq!(map.get("a"), Some(&1));
            assert_eq!(map.get("b"), Some(&2));
        } else {
            panic!("Expected One state");
        }
    }

    #[test]
    fn test_serialize_one() {
        let state = OneOrMany::One("test".to_string());
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, r#""test""#);
    }

    #[test]
    fn test_serialize_many() {
        let state: OneOrMany<i32> = OneOrMany::Many(vec![1, 2, 3]);
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "[1,2,3]");
    }
}
