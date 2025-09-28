#[macro_export]
macro_rules! config_struct {
    ($impl:item) => {
        #[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        $impl
    };
}

#[macro_export]
macro_rules! config_enum {
    ($impl:item) => {
        #[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "kebab-case")]
        $impl
    };
}

#[macro_export]
macro_rules! config_unit_enum {
    ($impl:item) => {
        $crate::config_enum!(
            #[derive(Copy, Default)]
            $impl
        );
    };
}

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

#[doc(hidden)]
#[macro_export]
macro_rules! generate_io_file_methods {
    ($name:ident) => {
        impl $name {
            pub fn get_path(&self) -> String {
                let path = self.file.as_str();

                if self.is_workspace_relative() {
                    path[1..].into()
                } else {
                    path.into()
                }
            }

            pub fn is_workspace_relative(&self) -> bool {
                self.file.as_str().starts_with('/')
            }

            pub fn to_workspace_relative(
                &self,
                project_source: impl AsRef<str>,
            ) -> WorkspaceRelativePathBuf {
                expand_to_workspace_relative(
                    if self.is_workspace_relative() {
                        RelativeFrom::Workspace
                    } else {
                        RelativeFrom::Project(project_source.as_ref())
                    },
                    self.get_path(),
                )
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! generate_io_glob_methods {
    ($name:ident) => {
        impl $name {
            pub fn get_path(&self) -> String {
                let path = self.glob.as_str();

                if self.is_workspace_relative() {
                    if self.is_negated() {
                        format!("!{}", &path[2..])
                    } else {
                        path[1..].into()
                    }
                } else {
                    path.into()
                }
            }

            pub fn is_negated(&self) -> bool {
                self.glob.as_str().starts_with('!')
            }

            pub fn is_workspace_relative(&self) -> bool {
                let path = self.glob.as_str();

                path.starts_with('/') || path.starts_with("!/")
            }

            pub fn to_workspace_relative(
                &self,
                project_source: impl AsRef<str>,
            ) -> WorkspaceRelativePathBuf {
                expand_to_workspace_relative(
                    if self.is_workspace_relative() {
                        RelativeFrom::Workspace
                    } else {
                        RelativeFrom::Project(project_source.as_ref())
                    },
                    self.get_path(),
                )
            }
        }
    };
}
