pub mod extension;

#[macro_export]
macro_rules! config_struct {
    ($struct:item) => {
        #[derive(Debug, serde::Deserialize)]
        #[serde(default, deny_unknown_fields, rename_all = "camelCase")]
        $struct
    };
}
