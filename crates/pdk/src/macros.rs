#[macro_export]
macro_rules! json_config {
    ($container:ident, $model:ident) => {
        pub struct $container {
            data: $model,
            dirty: Vec<String>,
            path: moon_pdk::VirtualPath,
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
            pub fn new(path: moon_pdk::VirtualPath) -> Self {
                Self {
                    data: Default::default(),
                    dirty: vec![],
                    path,
                }
            }

            pub fn load(path: &moon_pdk::VirtualPath) -> AnyResult<Self> {
                Ok(Self {
                    data: starbase_utils::json::read_file(path.any_path())
                        .map_err(moon_pdk::map_miette_error)?,
                    dirty: vec![],
                    path: path.to_owned(),
                })
            }

            pub fn save(self) -> AnyResult<Option<moon_pdk::VirtualPath>> {
                if self.dirty.is_empty() {
                    return Ok(None);
                }

                use starbase_utils::json;

                let mut data: json::JsonValue =
                    json::read_file(self.path.any_path()).map_err(moon_pdk::map_miette_error)?;

                for field in &self.dirty {
                    match self.save_field(field)? {
                        Some(value) => {
                            data[field] = value;
                        }
                        None => {
                            if let Some(root) = data.as_object_mut() {
                                root.remove(field);
                            }
                        }
                    };
                }

                Ok(Some(self.path))
            }
        }
    };
}
