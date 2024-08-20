use moon_common::cacheable;
use schematic::schema::UnionType;
use schematic::{Schema, SchemaBuilder, Schematic};

cacheable!(
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[serde(untagged, expecting = "expected a single value, or a list of values")]
    pub enum OneOrMany<T: Schematic> {
        One(T),
        Many(Vec<T>),
    }
);

impl<T: Schematic + Clone> OneOrMany<T> {
    pub fn to_list(&self) -> Vec<T> {
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
