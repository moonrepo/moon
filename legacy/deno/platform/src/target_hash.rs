use moon_hash::hash_content;
use std::collections::BTreeMap;
use typescript_tsconfig_json::CompilerOptions;

hash_content!(
    pub struct DenoTargetHash {
        // Deno version
        deno_version: String,

        // All the dependencies (and their integrity hashes) of the project
        dependencies: BTreeMap<String, Vec<String>>,
    }
);

impl DenoTargetHash {
    pub fn new(deno_version: Option<String>) -> Self {
        DenoTargetHash {
            deno_version: deno_version.unwrap_or_else(|| "unknown".into()),
            dependencies: BTreeMap::new(),
        }
    }

    pub fn hash_deps(&mut self, dependencies: BTreeMap<String, Vec<String>>) {
        self.dependencies = dependencies;
    }
}

hash_content!(
    #[derive(Default)]
    pub struct TypeScriptTargetHash {
        // `tsconfig.json` `compilerOptions`
        compiler_options: BTreeMap<String, String>,
    }
);

impl TypeScriptTargetHash {
    /// Hash compiler options that may alter compiled/generated output.
    pub fn hash_compiler_options(&mut self, compiler_options: &CompilerOptions) {
        if let Some(jsx) = &compiler_options.jsx {
            self.compiler_options
                .insert("jsx".to_owned(), format!("{jsx:?}"));
        }

        if let Some(jsx_factory) = &compiler_options.jsx_factory {
            self.compiler_options
                .insert("jsxFactory".to_owned(), format!("{jsx_factory:?}"));
        }

        if let Some(jsx_fragment_factory) = &compiler_options.jsx_fragment_factory {
            self.compiler_options.insert(
                "jsxFragmentFactory".to_owned(),
                format!("{jsx_fragment_factory:?}"),
            );
        }

        if let Some(jsx_import_source) = &compiler_options.jsx_import_source {
            self.compiler_options.insert(
                "jsxImportSource".to_owned(),
                format!("{jsx_import_source:?}"),
            );
        }

        if let Some(module) = &compiler_options.module {
            self.compiler_options
                .insert("module".to_owned(), format!("{module:?}"));
        }

        if let Some(module_resolution) = &compiler_options.module_resolution {
            self.compiler_options.insert(
                "moduleResolution".to_owned(),
                format!("{module_resolution:?}"),
            );
        }

        if let Some(target) = &compiler_options.target {
            self.compiler_options
                .insert("target".to_owned(), format!("{target:?}"));
        }
    }
}
