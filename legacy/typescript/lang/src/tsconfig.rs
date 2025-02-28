// tsconfig.json

use cached::proc_macro::cached;
use moon_lang::config_cache_model;
use moon_utils::path::to_relative_virtual_string;
use starbase_utils::json::{self, JsonValue, read_file as read_json};
use std::path::{Path, PathBuf};

pub use typescript_tsconfig_json::*;

config_cache_model!(
    TsConfigJsonCache,
    TsConfigJson,
    "tsconfig.json",
    read_json,
    write_preserved_json
);

impl TsConfigJsonCache {
    pub fn add_include<T: AsRef<str>>(&mut self, pattern: T) -> bool {
        let pattern = CompilerPath::from(pattern.as_ref());
        let mut include = match &self.data.include {
            Some(refs) => refs.clone(),
            None => Vec::<CompilerPath>::new(),
        };

        if include.iter().any(|p| p == &pattern) {
            return false;
        }

        include.push(pattern);
        include.sort();

        self.dirty.push("include".into());
        self.data.include = Some(include);

        true
    }

    pub fn add_include_path<T: AsRef<Path>>(&mut self, path: T) -> miette::Result<bool> {
        let path = to_relative_virtual_string(path.as_ref(), self.path.parent().unwrap())?;

        Ok(self.add_include(path.as_str()))
    }

    /// Add a project reference to the `references` field with the defined
    /// path and tsconfig file name, and sort the list based on path.
    /// Return true if the new value is different from the old value.
    pub fn add_project_ref<T: AsRef<Path>, C: AsRef<str>>(
        &mut self,
        base_path: T,
        tsconfig_name: C,
    ) -> miette::Result<bool> {
        let mut base_path = base_path.as_ref().to_path_buf();
        let tsconfig_name = tsconfig_name.as_ref();

        // File name is optional when using standard naming
        if tsconfig_name != "tsconfig.json" {
            base_path = base_path.join(tsconfig_name);
        };

        let path = to_relative_virtual_string(base_path, self.path.parent().unwrap())?;

        let mut references = match &self.data.references {
            Some(refs) => refs.clone(),
            None => Vec::<ProjectReference>::new(),
        };

        // Check if the reference already exists
        if references.iter().any(|r| r.path.as_str() == path) {
            return Ok(false);
        }

        // Add and sort the references
        references.push(ProjectReference {
            path: CompilerPath::from(path),
            prepend: None,
        });

        references.sort_by_key(|r| r.path.clone());

        self.dirty.push("references".into());
        self.data.references = Some(references);

        Ok(true)
    }

    pub fn update_compiler_options<F>(&mut self, updater: F) -> bool
    where
        F: FnOnce(&mut CompilerOptions) -> bool,
    {
        let updated;

        if let Some(options) = self.data.compiler_options.as_mut() {
            updated = updater(options);
        } else {
            let mut options = CompilerOptions::default();

            updated = updater(&mut options);

            if updated {
                self.data.compiler_options = Some(options);
            }
        }

        if updated {
            self.dirty.push("compilerOptions".into());
        }

        updated
    }

    pub fn update_compiler_option_paths(&mut self, paths: CompilerOptionsPathsMap) -> bool {
        self.update_compiler_options(|options| {
            let mut updated = false;

            if let Some(current_paths) = options.paths.as_mut() {
                for (path, mut patterns) in paths {
                    if let Some(current_patterns) = current_paths.get_mut(&path) {
                        patterns.sort();
                        current_patterns.sort();

                        if &patterns != current_patterns {
                            updated = true;
                            current_paths.insert(path, patterns);
                        }
                    } else {
                        updated = true;
                        current_paths.insert(path, patterns);
                    }
                }
            } else {
                updated = true;
                options.paths = Some(paths);
            }

            updated
        })
    }
}

// https://github.com/serde-rs/json/issues/858
// `serde-json` does NOT preserve original order when serializing the struct,
// so we need to hack around this by using the `json` crate and manually
// making the changes. For this to work correctly, we need to read the json
// file again and parse it with `json`, then stringify it with `json`.
#[track_caller]
fn write_preserved_json(path: &Path, tsconfig: &TsConfigJsonCache) -> miette::Result<()> {
    let mut data: JsonValue = json::read_file(path)?;

    // We only need to set fields that we modify within moon,
    // otherwise it's a ton of overhead and maintenance!
    for field in &tsconfig.dirty {
        match field.as_ref() {
            "include" => {
                if let Some(include) = &tsconfig.data.include {
                    data[field] = JsonValue::from_iter(
                        include.iter().map(|i| i.to_string()).collect::<Vec<_>>(),
                    );
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "references" => {
                if let Some(references) = &tsconfig.data.references {
                    let mut list = vec![];

                    for reference in references {
                        let mut item = json::json!({});
                        item["path"] = JsonValue::from(reference.path.as_str());

                        if let Some(prepend) = reference.prepend {
                            item["prepend"] = JsonValue::from(prepend);
                        }

                        list.push(item);
                    }

                    data[field] = JsonValue::Array(list);
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "compilerOptions" => {
                if let Some(options) = &tsconfig.data.compiler_options {
                    if (options.out_dir.is_some() || options.paths.is_some())
                        && !data[field].is_object()
                    {
                        data[field] = json::json!({});
                    }

                    if let Some(out_dir) = &options.out_dir {
                        data[field]["outDir"] = JsonValue::from(out_dir.as_str());
                    }

                    if let Some(paths) = &options.paths {
                        data[field]["paths"] =
                            JsonValue::from_iter(paths.iter().map(|(key, value)| {
                                (
                                    key.to_owned(),
                                    value.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
                                )
                            }));
                    }
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            _ => {}
        }
    }

    json::write_file_with_config(path, &data, true)?;

    Ok(())
}
