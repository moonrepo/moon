# Changelog

## Unreleased

#### 🚀 Updates

- Temporarily disabled shallow checkouts triggering a hard error in CI until we can implement a
  better solution. This means that if you have a shallow checkout, you may see incorrect affected
  results, or Git commands may fail.
- Added more logs to `moon docker prune` to help debug edge cases.
- Added `MOON_INCLUDE_RELATIONS` environment variable support for the `--include-relations` CLI
  option.
- Added `.env` and `.env.*` as defaults to the `hasher.ignoreMissingPatterns` setting.

#### 🐞 Fixes

- Fixed an issue where the graph visualizers would not render correctly in the VS Code extension.
- Fixed an issue where a task with `shell: false` would be force enabled when a glob/env was
  detected. We now respect the configured value.
- Fixed an issue where "run" type based tasks would not run in CI.

## 2.0.1

#### 🚀 Updates

- Updated `moon upgrade` to upgrade via proto if we detect that moon is managed by proto. This will
  run `proto install moon latest`.

#### 🐞 Fixes

- Fixed some WASM serialization errors.
- Fixed the `moon upgrade` command not handling the new v2 distribution format correctly. If you are
  on moon v2.0.0, the upgrade command will still be broken until you upgrade to this patch.

## 2.0.0

#### 💥 Breaking

View the [migration guide](https://moonrepo.dev/docs/migrate/2.0) for a full list of breaking
changes and how to easily migrate!

- Renamed "touched files" to "changed files".

- **CLI**
  - Removed canary and nightly releases.
  - Removed commands: `moon node`, `moon migrate from-package-json`, `moon query hash`,
    `moon query hash-diff`
  - Renamed all options and flags to kebab-case instead of camelCase.
  - Reworked many commands and their arguments. Refer to the migration guide for details.
  - Reworked console output handling. Updated `--summary` with different levels.
  - Reworked release distribution to use archives instead of direct executables.
- **Configuration**
  - Renamed, removed, or changed _many_ settings. Refer to the migration guide for details.
  - Renamed `.moon/toolchain.yml` to `.moon/toolchains.yml` (plural).
- **MCP**
  - Updated protocol version to 2025-11-25.
  - Updated `get_projects` and `get_tasks` to return fragments, to reduce the payload size.
- **Projects**
  - Reworked how the `language` is detected.
  - Flattened `project` metadata structure.
- **Tasks**
  - Task inheritance now deep merges instead of shallow merges when dealing with extends and
    multi-global.
  - Task `command` and `args` only support simple commands now. Use `script` for compound commands
    (pipes, redirects, multiple commands, etc).
  - Removed "watcher" task `preset`.
  - Reworked env var merge order, substitution, and more. Refer to the migration guide for details.
  - Reworked `.env` handling.
    - Moved loading to occur before task execution, instead of creation.
    - Can _no longer_ reference task `env` vars for substitution.
- **Toolchain**
  - Removed the old platform system, and replaced it with the new WASM plugin system.
    - All old "stable" toolchains have been replaced with the new "unstable" toolchains.
- **VCS**
  - Reworked the hooks layer for better interoperability.
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
  - Applies "transitive reduction" to the graph, removing unnecessary edges for better performance.
  - Improved console output, logging, and error handling.
  - Improved parallelism when running tasks.
    - Now resolves and expands targets _before_ partitioning.
    - Now partitions _after_ filtering based on affected state.
- **CLI**
  - New commands: `moon exec`, `moon extension`, `moon hash`, `moon projects`, `moon tasks`,
    `moon query affected`, `moon template`
  - Updated commands `moon check`, `moon ci`, and `moon run`:
    - Now uses `moon exec` under the hood.
    - Added levels to `--summary`.
  - Updated commands that require an identifier to prompt for it if not provided.
  - Stabilized the `moonx` binary (which uses `moon exec` under the hood).
  - Added support for `.config/moon` instead of `.moon`.
  - Added support for `...` in task targets, which is an alias for `**/*`. This is similar to how
    Bazel targets work.
  - Improved stack memory usage by pushing thread data to the heap. This resolves spurious stack
    overflow issues.
- **Configuration**
  - Added support for more formats: JSON, TOML, and HCL.
  - Improved error messages for union based settings.
  - `.moon/extensions.*`
    - New file for configuring extensions (formerly in `workspace.extensions`).
  - `.moon/tasks.*`
    - Added `inheritedBy` setting for configuration based task inheritance.
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
- **MCP**
  - Added a `generate` tool for running the code generator.
- **Projects**
  - Added a default project concept.
  - Added path based IDs instead of dir name IDs.
  - Updated projects to support multiple aliases (one from each applicable toolchain).
- **Remote cache**
  - Stabilized all settings.
  - Enabled gzip/zstd compression for HTTP requests.
- **Tasks**
  - Added deep merging support for task inheritance.
  - Updated `command` and `args` with better syntax parsing and error handling.
    - Better handling of quotes, escapes, and spaces.
    - Extracts env vars into the task.
  - Updated `env` values to support `null`, which would remove an inherited system env var.
  - Updated `envFile` option to support token/var substitution.
  - Improved `.env` handling:
    - Updated the parser to support more syntax.
    - Updated loading to occur before task execution, instead of creation.
    - Can now reference system/moon/task env vars for substitution.
- **Toolchains**
  - Stabilized the new WASM plugin system.
  - Improved how toolchains extend env vars and paths for commands and scripts.
- **Tokens**
  - Added new tokens: `$projectTitle`, `$projectAliases`, `$taskToolchains`
- **VCS**
  - Replaced the old v1 Git implementation with a new v2 implementation.
  - Improved support for worktrees, submodules, and more.
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

- Fixed local executables in `@moonrepo` packages not being detected correctly.
- Fixed task job parallelism to partition _after_ tasks have been filtered based on affected state.
- Fixed an issue where env var substitution would not process in the order they were defined.
- Fixed an issue where ctrl+c wouldn't exit when a prompt was waiting for input.
- Fixed an issue where `project` based task inputs would not be reflected internally in the input
  files/globs list.
- Fixed an issue where running a task that triggers a system/moon error wouldn't output the error
  message. This also aborts the action pipeline correctly now.
- Fixed an issue where errors during project graph building would not be reported correctly.
- Fixed an issue where a negated glob in a file group would not expand properly when used as an
  argument.

#### ⚙️ Internal

- Updated proto to [v0.55.2](https://github.com/moonrepo/proto/releases/tag/v0.55.0) from 0.53.2
  (view [v0.54](https://github.com/moonrepo/proto/releases/tag/v0.54.0) changes).
- Updated wasmtime to v37.
- Updated Rust to v1.93.0.
- Updated dependencies.
