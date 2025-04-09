use crate::config_enum;
use schematic::schema::UnionType;
use schematic::{Schema, SchemaBuilder, Schematic};

config_enum!(
    #[serde(untagged, expecting = "expected a single value, or a list of values")]
    pub enum OneOrMany<T: Schematic> {
        One(T),
        Many(Vec<T>),
    }
);

impl<T: Schematic> Default for OneOrMany<T> {
    fn default() -> Self {
        Self::Many(vec![])
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
