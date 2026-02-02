use schematic::schema::UnionType;
use schematic::{Schema, SchemaBuilder, Schematic};
use serde::{Deserialize, Deserializer, Serialize};

/// Represents a single value, or a list of multiple values.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum OneOrMany<T: Schematic> {
    One(T),
    Many(Vec<T>),
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

impl<'de, T: Deserialize<'de> + Schematic> Deserialize<'de> for OneOrMany<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;

        // Buffer the content so we can try deserializing it multiple ways
        let content = deserializer.deserialize_any(schematic::serde_content::ValueVisitor)?;
        let mut errors: Vec<(&str, String)> = Vec::new();

        // Try deserializing all variants
        match T::deserialize(
            schematic::serde_content::Deserializer::new(content.clone())
                .coerce_numbers()
                .human_readable(),
        ) {
            Ok(value) => return Ok(Self::One(value)),
            Err(error) => errors.push(("One", error.to_string())),
        };

        match Vec::<T>::deserialize(
            schematic::serde_content::Deserializer::new(content.clone())
                .coerce_numbers()
                .human_readable(),
        ) {
            Ok(value) => return Ok(Self::Many(value)),
            Err(error) => errors.push(("Many", error.to_string())),
        };

        // All variants failed, build the combined error message
        let mut error_msg =
            format!("failed to parse as a single value, or a list of multiple values:");

        for (variant_name, error) in &errors {
            error_msg.push_str(&format!("\n- {}: {}", variant_name, error));
        }

        Err(D::Error::custom(error_msg))
    }
}
