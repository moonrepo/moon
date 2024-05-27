#[macro_export]
macro_rules! cacheable {
    ($impl:item) => {
        #[derive(serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        $impl
    };
}

#[macro_export]
macro_rules! cacheable_enum {
    ($impl:item) => {
        #[derive(serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "kebab-case")]
        $impl
    };
}
