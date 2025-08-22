/// Apply default attributes for configuration based structs.
/// Will assume that all keys are in camel case.
#[macro_export]
macro_rules! config_struct {
    ($struct:item) => {
        #[derive(Debug, serde::Deserialize)]
        #[serde(default, rename_all = "camelCase")]
        $struct
    };
}

#[macro_export]
macro_rules! shared_config {
    ($container:ident, $model:ident) => {
        #[derive(Default)]
        pub struct $container {
            pub data: $model,
            pub dirty: Vec<String>,
            pub path: moon_pdk_api::VirtualPath,
        }

        impl std::ops::Deref for $container {
            type Target = $model;

            fn deref(&self) -> &Self::Target {
                &self.data
            }
        }

        impl std::ops::DerefMut for $container {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.data
            }
        }

        impl $container {
            pub fn new(path: moon_pdk_api::VirtualPath) -> Self {
                Self {
                    data: Default::default(),
                    dirty: vec![],
                    path,
                }
            }

            pub fn is_dirty(&self) -> bool {
                !self.dirty.is_empty()
            }
        }
    };
}

#[macro_export]
macro_rules! json_config {
    ($container:ident, $model:ident) => {
        moon_pdk_api::shared_config!($container, $model);

        impl $container {
            pub fn load(path: moon_pdk_api::VirtualPath) -> AnyResult<Self> {
                if path.exists() {
                    Ok(Self {
                        data: starbase_utils::json::read_file(path.any_path())?,
                        dirty: vec![],
                        path,
                    })
                } else {
                    Ok(Self::new(path))
                }
            }

            pub fn save(self) -> AnyResult<Option<moon_pdk_api::VirtualPath>> {
                if self.dirty.is_empty() {
                    return Ok(None);
                }

                use starbase_utils::json;

                let mut data: json::JsonValue = json::read_file(self.path.any_path())?;

                for field in &self.dirty {
                    self.save_field(field, &mut data)?;
                }

                #[cfg(feature = "wasm")]
                {
                    host_log!(
                        "Saving <path>{}</path> with changed fields {}",
                        self.path,
                        self.dirty
                            .into_iter()
                            .map(|dirty| format!("<property>{dirty}</property>"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }

                json::write_file_with_config(&self.path, &data, true)?;

                Ok(Some(self.path))
            }

            pub fn save_model(self) -> AnyResult<moon_pdk_api::VirtualPath> {
                use starbase_utils::json;

                #[cfg(feature = "wasm")]
                {
                    host_log!("Saving <path>{}</path>", self.path);
                }

                json::write_file_with_config(&self.path, &self.data, true)?;

                Ok(self.path)
            }
        }
    };
}

#[macro_export]
macro_rules! toml_config {
    ($container:ident, $model:ident) => {
        moon_pdk_api::shared_config!($container, $model);

        impl $container {
            pub fn load(path: moon_pdk_api::VirtualPath) -> AnyResult<Self> {
                if path.exists() {
                    Ok(Self {
                        data: starbase_utils::toml::read_file(path.any_path())?,
                        dirty: vec![],
                        path,
                    })
                } else {
                    Ok(Self::new(path))
                }
            }

            pub fn save(self) -> AnyResult<Option<moon_pdk_api::VirtualPath>> {
                if self.dirty.is_empty() {
                    return Ok(None);
                }

                use starbase_utils::toml;

                let mut data: toml::TomlValue = toml::read_file(self.path.any_path())?;

                for field in &self.dirty {
                    self.save_field(field, &mut data)?;
                }

                #[cfg(feature = "wasm")]
                {
                    host_log!(
                        "Saving <path>{}</path> with changed fields {}",
                        self.path,
                        self.dirty
                            .into_iter()
                            .map(|dirty| format!("<property>{dirty}</property>"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }

                toml::write_file(&self.path, &data, true)?;

                Ok(Some(self.path))
            }

            pub fn save_model(self) -> AnyResult<moon_pdk_api::VirtualPath> {
                use starbase_utils::toml;

                #[cfg(feature = "wasm")]
                {
                    host_log!("Saving <path>{}</path>", self.path);
                }

                toml::write_file(&self.path, &self.data, true)?;

                Ok(self.path)
            }
        }
    };
}
