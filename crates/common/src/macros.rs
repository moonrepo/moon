/// Derive `Serialize`/`Deserialize` with `camelCase` field renaming.
/// Use for structs that are persisted to cache or serialized to JSON.
#[macro_export]
macro_rules! cacheable {
    ($impl:item) => {
        #[derive(serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        $impl
    };
}

/// Derive `Serialize`/`Deserialize` with `kebab-case` variant renaming.
/// Use for enums that are persisted to cache or serialized to JSON.
#[macro_export]
macro_rules! cacheable_enum {
    ($impl:item) => {
        #[derive(serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "kebab-case")]
        $impl
    };
}
