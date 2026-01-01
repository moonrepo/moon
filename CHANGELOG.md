# Changelog

## Unreleased

#### 💥 Breaking

View the [migration guide](https://moonrepo.dev/docs/migrate/2.0) for a full list of breaking
changes and how to easily migrate!

- **WASM API**
  - Renamed `ProjectFragment.alias` to `ProjectFragment.aliases` and changed its type from
    `Option<String>` to `Vec<String>`.
  - Removed `RegisterExtensionOutput.config_schema` field. Use the new `define_extension_config`
    plugin function instead.

#### 🚀 Updates

View the [announcement blog post](https://moonrepo.dev/blog/moon-v2.0) for all updates, new
features, improvements, and much more!

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

#### 🧩 Extensions

- **Migrate Nx**
  - Added support for the following `project.json` fields: `targets.*.continuous`
- **Migrate Turborepo**
  - Added support for the following `turbo.json` fields: `tags`, `tasks.*.env` (wildcards and
    negation)
- **Unpack**
  - Updated to use `unzip` and `tar` commands.

#### 🧰 Toolchains

- **JavaScript**
  - Added support for Yarn v4.10 catalogs.
  - Fixed an issue where implicit dependencies would sometimes not resolve.

#### 🐞 Fixes

- Fixed task job parallelism to partition _after_ tasks have been filtered based on affected state.
- Fixed an issue where env var substitution would not process in the order they were defined.

#### ⚙️ Internal

- Updated proto to [v0.54.1](https://github.com/moonrepo/proto/releases/tag/v0.54.0) (from 0.53.2).
- Updated wasmtime to v37.
- Updated Rust to v1.92.0.
- Updated dependencies.
