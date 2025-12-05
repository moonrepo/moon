# Changelog

## Unreleased

#### 💥 Breaking

- **WASM API**
  - Renamed `ProjectFragment.alias` to `ProjectFragment.aliases` and changed its type from
    `Option<String>` to `Vec<String>`.
  - Removed `RegisterExtensionOutput.config_schema` field. Use the new `define_extension_config`
    plugin function instead.

#### 🚀 Updates

- **WASM API**
  - Added a `load_extension_config_by_id` host function.
  - Added `define_extension_config`, `initialize_extension`, and `extend_command` plugin functions.
  - Added `load_extension_config`, `parse_extension_config` and `parse_extension_config_schema`
    utility functions.
  - Added `DefineExtensionConfigOutput`, `InitializeExtensionInput`, `InitializeExtensionOutput`,
    `ExtendCommandInput`, and `ExtendCommandOutput` types.
  - Added `ExtendProjectGraphInput.extension_config`, `ExtendTaskCommandInput.extension_config`,
    `ExtendTaskScriptInput.extension_config`, `SyncProjectInput.extension_config`, and
    `SyncWorkspaceInput.extension_config` fields.
  - Added `RegisterToolchainOutput.language` field.

#### ⚙️ Internal

- Updated proto to [v0.54.0](https://github.com/moonrepo/proto/releases/tag/v0.54.0) (from 0.53.2).
- Updated wasmtime to v37.
- Updated Rust to v1.91.0.
- Updated dependencies.
