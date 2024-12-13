#[macro_export]
macro_rules! generate_switch {
    ($name:ident, [ $($value:literal),* ]) => {
        impl $name {
            pub fn is_enabled(&self) -> bool {
                !matches!(self, Self::Enabled(false))
            }
        }

        impl From<bool> for $name {
            fn from(value: bool) -> Self {
                Self::Enabled(value)
            }
        }

        impl Schematic for $name {
            fn build_schema(mut schema: SchemaBuilder) -> Schema {
                schema.union(UnionType::new_any([
                    schema.infer::<bool>(),
                    schema.nest().string(StringType {
                        enum_values: Some(Vec::from_iter([
                            $(
                                $value.into()
                            ),*
                        ])),
                        ..Default::default()
                    }),
                ]))
            }
        }
    };
}
