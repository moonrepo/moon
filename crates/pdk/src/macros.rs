#[macro_export]
macro_rules! shared_config {
    ($container:ident, $model:ident) => {
        pub struct $container {
            pub path: moon_pdk::VirtualPath,
            data: $model,
            dirty: Vec<String>,
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

            pub fn is_dirty(&self) -> bool {
                !self.dirty.is_empty()
            }
        }
    };
}

#[macro_export]
macro_rules! json_config {
    ($container:ident, $model:ident) => {
        moon_pdk::shared_config!($container, $model);

        impl $container {
            pub fn load(path: moon_pdk::VirtualPath) -> AnyResult<Self> {
                Ok(Self {
                    data: starbase_utils::json::read_file(path.any_path())?,
                    dirty: vec![],
                    path,
                })
            }

            pub fn save(self) -> AnyResult<Option<moon_pdk::VirtualPath>> {
                if self.dirty.is_empty() {
                    return Ok(None);
                }

                use starbase_utils::json;

                let mut data: json::JsonValue = json::read_file(self.path.any_path())?;

                for field in &self.dirty {
                    match self.save_field(field, data.get(field))? {
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

                host_log!(
                    "Saving <path>{}</path> with changed fields {}",
                    self.path.display(),
                    self.dirty
                        .into_iter()
                        .map(|dirty| format!("<property>{dirty}</property>"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                json::write_file_with_config(self.path.any_path(), &data, true)?;

                Ok(Some(self.path))
            }

            pub fn save_model(self) -> AnyResult<moon_pdk::VirtualPath> {
                use starbase_utils::json;

                host_log!("Saving <path>{}</path>", self.path.display());

                json::write_file_with_config(self.path.any_path(), &self.data, true)?;

                Ok(self.path)
            }
        }
    };
}
