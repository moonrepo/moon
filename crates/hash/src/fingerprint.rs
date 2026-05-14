#[macro_export]
macro_rules! fingerprint {
    ($impl:item) => {
        #[derive(serde::Serialize)]
        #[serde(default, rename_all = "camelCase")]
        $impl
    };
}
