# Changelog

## Unreleased

#### 💥 Breaking

View the [migration guide](https://moonrepo.dev/docs/migrate/2.0) for a full list of breaking
changes and how to easily migrate!

- Renamed "touched files" to "changed files".
- **CLI**
  - Removed commands: `moon node`, `moon migrate from-package-json`, `moon query hash`,
    `moon query hash-diff`
  - Renamed all options and flags to kebab-case instead of camelCase.
  - Reworked many commands and their arguments. Refer to the migration guide for details.
  - Reworked console output handling. Updated `--summary` with different levels.
- **Configuration**
  - Renamed, removed, or changed _many_ settings. Refer to the migration guide for details.
  - Renamed `.moon/toolchain.yml` to `.moon/toolchains.yml` (plural).
- **Projects**
  - Reworked how the `language` is detected.
  - Flattened `project` metadata structure.
- **Tasks**
  - Reworked `.env` handling.
  - Reworked env var merge order, interpolation, and more.
- **Toolchain**
  - Removed the old platform system, and replaced it with the new WASM plugin system.
    - All old "stable" toolchains have been replaced with the new "unstable" toolchains.
- **WASM API**
  - Removed the `/cwd` virtual path.
  - Renamed `ProjectFragment.alias` to `ProjectFragment.aliases` and changed its type from
    `Option<String>` to `Vec<String>`.
  - Removed `RegisterExtensionOutput.config_schema` field. Use the new `define_extension_config`
    plugin function instead.

#### 🚀 Updates

View the [announcement blog post](https://moonrepo.dev/blog/moon-v2.0) for all updates, new
features, improvements, and much more!

- **Action pipeline**
  - Will now always generate a hash for a task, even if caching is disabled.
- **CLI**
  - New commands: `moon exec`, `moon extension`, `moon hash`, `moon projects`, `moon tasks`,
    `moon query affected`, `moon template`
  - Stabilized the `moonx` binary (which uses `moon exec` under the hood).
  - Added support for `.config/moon` instead of `.moon`.
- **Configuration**
  - Added support for more formats: JSON, TOML, and HCL.
  - `.moon/extensions.*`
    - New file for configuring extensions (formerly in `workspace.extensions`).
  - `.moon/tasks.*`
    - Added `inheritedBy` for configuration based task inheritance.
  - `.moon/workspace.*`
    - Added `projects.globFormat` setting.
    - Added `defaultProject` setting.
    - Stabilized remote caching.
  - `moon.*`
    - Added `mergeToolchains` task option.
    - Added "utility" task `preset`.
    - Added "data" `stack`.
- **Docker**
  - Better toolchain integration.
  - Added `--no-setup` and `--template` support to `moon docker file`.
  - Updated project configs to override workspace configs.
- **Extensions**
  - Added a new extension, `unpack`, for unpacking archive files.
  - Added `.moon/extensions.*` configuration file.
  - Added support for new plugin APIs: `define_extension_config`, `extend_command`,
    `extend_project_graph`, `extend_task_command`, `extend_task_script`, `sync_project`, and
    `sync_workspace`.
- **Projects**
  - Added a default project concept.
  - Added path based IDs instead of dir name IDs.
  - Updated projects to support multiple aliases (one from each applicable toolchain).
- **Tasks**
  - Added `inheritedBy` task option for configuration based task inheritance.
  - Added deep merging support to task inheritance.
  - Updated `.env` loading to occur before task execution, instead of creation.
- **Toolchains**
  - Integrated the new WASM plugin system.
  - Improved how toolchains extend env vars and paths for commands and scripts.
- **Tokens**
  - Added new tokens: `$projectTitle`, `$projectAliases`, `$taskToolchains`
- **VCS**
  - Replaced the old v1 Git implementation with a new v2 implementation.
  - Improved support for worktrees, submodules, and more.
  - Rewrote the hooks layer for better interoperability.
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
- Fixed an issue where ctrl+c wouldn't exit when a prompt was waiting for input.
- Fixed an issue where `project` based task inputs would not be reflected internally in the input
  files/globs list.

#### ⚙️ Internal

- Updated proto to [v0.54.1](https://github.com/moonrepo/proto/releases/tag/v0.54.0) (from 0.53.2).
- Updated wasmtime to v37.
- Updated Rust to v1.92.0.
- Updated dependencies.
