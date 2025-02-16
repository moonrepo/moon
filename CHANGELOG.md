# Changelog

## Unreleased

#### ⚙️ Internal

- Updated proto to [v0.46.0](https://github.com/moonrepo/proto/releases/tag/v0.46.0) (from 0.45.2).

## 1.32.5

#### 🚀 Updates

- Updated Python uv with new features:
  - Project aliases are inferred from `project.name` in `pyproject.toml`.
  - Implicit project dependencies are inferred from `project.dependencies` in `pyproject.toml`.
    - Requires a bare identifier with no version/url/origin markers.
    - Currently does not support `tool.uv` or workspaces.

#### 🐞 Fixes

- Fixed an issue where no touched files would be found when pushing commits on the default branch.
- Fixed an issue where `pyproject.toml` or `uv.lock` would sometimes not be parsed.

## 1.32.4

#### 🐞 Fixes

- Fixed an issue where a project could be inserted into the project graph multiple times.
- Fixed an issue where touched files would sometimes not be available outside of affected scopes.

## 1.32.3

#### 🐞 Fixes

- Fixed an issue where `moon setup` wouldn't load configuration.
- Fixed an issue where `moon docker scaffold` would unexpectedly install the toolchain languages.
- Fixed an issue where `moon docker scaffold` would not copy `uv.toml` and `uv.lock` files.
- Fixed an issue where changing `python.version` wouldn't regenerate the Python virtual environment.

## 1.32.2

#### 🐞 Fixes

- Fixed an issue where `NO_COLOR` or `FORCE_COLOR` would be overwritten for task child processes.

#### ⚙️ Internal

- Updated proto to [v0.45.2](https://github.com/moonrepo/proto/releases/tag/v0.45.2) (from 0.45.1).
- Updated dependencies.

## 1.32.1

#### 🚀 Updates

- Added a `runner.killProcessThreshold` setting to control the threshold in which to force kill
  child processes when a signal is received. Defaults to 2 seconds.

#### 🐞 Fixes

- Fixed an issue where `bash` and `batch` project language's would not inherit tasks.
- Fixed some `bun.lock` parsing issues.

## 1.32.0

#### 🚀 Updates

- Added unstable support for multiple package workspaces when installing dependencies and resolving
  lockfiles.
  - For example, you can now have multiple `yarn.lock` files throughout the repository, allowing for
    multiple Yarn/Node workspaces.
- Updated Pkl (`.pkl`) based configuration to always be enabled, and is no longer hidden behind an
  experimental flag.
  - Added official documentation: https://moonrepo.dev/docs/guides/pkl-config
- Updated our unstable remote service (Bazel RE API) with new functionality:
  - You can now use `http(s)` protocols for gRPC servers, instead of just `grpc(s)`.
  - Added an `unstable_remote.api` setting, which can be used to inform the server's API format.
    Defaults to `grpc`.
  - Added an `unstable_remote.auth` setting, which can be used for HTTP Bearer/token Authorization
    based endpoints. Can also be used to set headers for all requests.
  - Added support for Depot cloud-based caching: https://depot.dev/docs/cache/overview
  - Added support for the HTTP protocol: https://bazel.build/remote/caching#http-caching
  - Added support for the ByteStream API.
  - Added timeout and concurrency limit to clients.
  - Added a `MOON_DEBUG_REMOTE` environment variable, which can be used to debug internal errors for
    diagnosing connection/integration issues.
  - Improved handling of TLS/mTLS connections.
- Improved the Bun toolchain.
  - Added support for the new v1.2 `bun.lock` file. We do our best to detect this, but we suggest
    configuring `--save-text-lockfile`.
- Improved the Deno toolchain.
  - Added support for Deno v2 and `deno install`. Will now run this command instead of `deno cache`.
  - Added support for v3 and v4 lockfiles (we now use the `deno_lockfile` crate).
  - Added basic support for workspaces.
  - Added `deno.installArgs` setting.
- Improved the Python toolchain.
  - Added unstable uv support. Can be enabled with the new `python.packageManager` and `python.uv`
    settings.
    - Right now, has basic toolchain support, including dependency installs and virtual
      environments.
  - Renamed `python.rootRequirementsOnly` to `python.rootVenvOnly` (old name still works).
  - Will now inherit versions from the root `.prototools`.
- Improved the Rust toolchain.
  - The root-level project is now properly taken into account when detecting if a project is within
    the Cargo workspace.
  - Project dependencies (`dependsOn`) are now automatically inferred from `Cargo.toml`
    dependencies.
- Reworked child process handling to better handle signals and shutdown accordingly. Additionally,
  when the pipeline receives a signal, we now display the status that shutdown the pipeline in the
  summary.
- Reworked the new task `inferInputs` option to not infer environment variables from popular CI/CD
  providers, as those values constantly change, causing tasks to always be affected. If you would
  like to reference these values, add them as an explicit inputs.

#### 🐞 Fixes

- Fixed lots of issues around remote caching. Should be "stable" now.

#### ⚙️ Internal

- Updated Rust to v1.84.1.

## 1.31.3

#### 🐞 Fixes

- This patch upgrades proto to the latest version to fix some bin locating issues that you may have
  encountered if you have proto v0.45 installed on your system, which conflicts with the proto v0.44
  that moon installs.

#### ⚙️ Internal

- Updated proto to [v0.45.1](https://github.com/moonrepo/proto/releases/tag/v0.45.1) (from 0.44.4).

## 1.31.2

#### 🚀 Updates

- Added file locks for certain operations to avoid race collisions when multiple `moon` commands are
  ran in parallel.

#### 🐞 Fixes

- Fixed an issue where inferred inputs would include directories that would log a warning and fail
  to be hashed.

#### ⚙️ Internal

- Updated Rust to v1.84.

## 1.31.1

#### 🐞 Fixes

- Fixed `NO_COLOR` and `FORCE_COLOR` being unexpectedly set in some situations.
- Fixed an issue where a custom project `language` would no longer inherit global tasks of the same
  name.

#### ⚙️ Internal

- Updated proto to [v0.44.4](https://github.com/moonrepo/proto/releases/tag/v0.44.4) (from 0.44.2).

## 1.31.0

#### 💥 Breaking

- We've updated the task inheritance order to better reflect specificity. The biggest changes are
  that deno/node/bun are now a higher priority than javascript/typescript, and stack is the lowest
  priority. This should only affect users with very complex inheritance chains.

#### 🚀 Updates

- We are deprecating the concept of the "platform", as this is required for the next step in
  supporting WASM based toolchain plugins. Going forward, any reference to platform is now a
  toolchain. The following changes have been made:
  - Deprecated the top-level `platform` setting in `moon.yml`, use `toolchain.default` instead.
    - Additionally, the toolchain can now be inferred from the top-level `language` setting and any
      config files in the project/workspace root. This pattern is preferred when possible.
  - Deprecated the `platform` task setting, use `toolchain` instead.
  - Deprecated the `taskPlatform` query field, use `taskToolchain` instead.
  - Deprecated the `--platform` option for `moon query tasks`, use `--toolchain` instead.
  - Deprecated the `$taskPlatform` token, use `$taskToolchain` instead.
- Added the ability to run targets in `moon run` and `moon ci` using a glob-like syntax.
  - For example: `:build-*`, `app-*:build`, `#tag-{foo,bar}:build`, etc.
- Added a `--no-bail` flag to `moon run`, that will continue running tasks even when a task fails.
- Added glob support (and `glob://`) to `generator.templates` in `.moon/workspace.yml`, allowing you
  to glob for your codegen template locations.
- Added a task option `inferInputs`, that will automatically infer inputs based on file groups and
  substituted environment variables used within the task.
- Updated task option `runInCI` to support the values "always" (always run) and "affected" (only run
  if affected, same as `true`).
- Updated task inputs to automatically be inherited from file group token functions used within the
  task's `script`, `command`, `args`, or `env`.
- Updated the `extends` setting in `.moon/workspace.yml`, `toolchain.yml`, and `tasks.yml`, to
  support a list of files/URLs to extend.
- Updated our unstable remote service (Bazel RE API) with new functionality:
  - Added an `unstable_remote.cache.compression` setting, which can be set to `zstd` to enable
    zstandard compression on output blobs.
  - Symlink based outputs will now be created as symlinks on Windows, where previously they were
    copies. Do note that symlinks require
    [privileged access on Windows](https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-10/security/threat-protection/security-policy-settings/create-symbolic-links)
    to function correctly.
- Updated the `moon templates` command:
  - Templates are now sorted by identifier.
  - Added a `--filter` option to filter the templates list by ID.
- Updated toolchain dependency installs to retry up to 3 attempts if the install command fails.
- Improved the task output prefixing logic.

#### 🐞 Fixes

- Fixed a panic that could occur during command argument parsing.
- Fixed an issue where remote cached blobs would sometimes fail to be created locally.
- Fixed an issue when `ctrl+c`ing multiple persistent tasks would sometimes leave them running in
  the background.

#### ⚙️ Internal

- Updated proto to v0.44.2 (from 0.44.1).
- Updated dependencies.

## 1.30.6

#### 🐞 Fixes

- Fixed an issue where `python venv` would fail to find an applicable Python version.
- Fixed an issue with PowerShell Git hooks not bubbling up exit codes of failed commands.
- Fixed an issue where Git submodules/worktrees would point to the wrong hooks folder.

#### ⚙️ Internal

- Updated proto to v0.44.1 (from 0.43.1).

## 1.30.5

#### 🐞 Fixes

- Fixed Python virtual env bin path not being available for tasks when `python.version` is not
  defined.

#### ⚙️ Internal

- Updated proto to v0.43.1 (from 0.43.0).
- Updated dependencies.

## 1.30.4

#### 🐞 Fixes

- Fixed `moon ci` showing incorrect job related logs.
- Fixed some issues with the Python toolchain:
  - pip is no longer required to be enabled to activate a virtual environment.
  - Changed `python.rootRequirementsOnly` to `false` by default.
  - The venv root is now the location of a found `requirements.txt`, otherwise the package root, or
    workspace root if `python.rootRequirementsOnly` is enabled.
  - Tasks will now inherit the correct venv paths in `PATH`.

## 1.30.3

#### 🐞 Fixes

- Fixed an issue where a task with explicit no inputs (`inputs: []`) would always be marked as
  affected.

#### ⚙️ Internal

- Updated proto to v0.43.0 (from 0.42.2).
- Updated wasmtime to v26 (from v23).
- Updated Rust to v1.83.

## 1.30.2

#### 🐞 Fixes

- Fixed an issue where dependencies/dependents of an affected task would be skipped in the action
  graph if they were also not affected.
- Fixed a potential cycle (stack overflow) that may occur in the affected tracker.

#### ⚙️ Internal

- Improved task dependent resolution in the action graph.

## 1.30.1

#### 🐞 Fixes

- Fixed a Git "fatal: bad object" error when submodules are in being used.
- Fixed an issue where `moon ci` would trigger a shallow checkout error, even when a base revision
  is provided.
- Fixed an issue were changes in a non-direct dependency would not mark a deep dependent as
  affected.

## 1.30.0

#### 💥 Breaking

- If you renamed a project using the `id` setting in `moon.yml`, you can no longer reference that
  project in dependencies and targets using its original ID.
- Refactored the `moon query tasks` command.
  - CLI options have been replaced with new task based options, instead of being project based.
  - Now utilizes the new task graph and affected tracker.
- The `moon project-graph --json` output no longer includes task data (the `tasks` field is an empty
  map).
  - Use `moon task-graph` to access this data, and piece things together using the project
    `taskTargets` field.
  - The `moon project --json` output pieces everything together automatically.

#### 🚀 Updates

- Added _unstable_ support for self-hosted remote caches, powered by the
  [Bazel Remote Execution API](https://github.com/bazelbuild/remote-apis).
  - Allows for 3rd-party implementations like
    [`bazel-remote`](https://github.com/buchgr/bazel-remote) to be used.
  - Currently supports the gRPC protocol, and will support HTTP in a later release.
  - Our moonbase product will be sunset in the future.
- Added Python tier 2 and 3 support.
  - Will download and install Python into the toolchain when a `version` is configured.
  - Will parse the `requirements.txt` to resolve and install dependencies.
  - Added a `python` setting to `.moon/toolchain.yml`.
  - Added a `toolchain.python` setting to `moon.yml`.
  - Updated `moon bin` commands to support Python.
- Added a new task graph, that enables new granular based functionality for task related features.
  - Added a new `moon task-graph` command.
  - Can now control the depth of upstream (dependencies) and downstream (dependents).
  - Affected information now tracks based on dependent graph connections.
  - Added `--upstream` and `--downstream` options to `moon query tasks`.
- Added basic support for Git submodules, and will now extract touched files from all submodules.
- Added 7 new token variables: `$arch`, `$os`, `$osFamily`, `$vcsBranch`, `$vcsRepository`,
  `$vcsRevision`, `$workingDir`
- Added a `rust.binstallVersion` setting to `.moon/toolchain.yml`.
- Updated Pkl configurations to support `read()` for environment variables.
- Resolved the `strictProjectIds` experiment and you can no longer reference the original ID.
- Resolved the `disallowRunInCiMismatch` experiment and you can no longer have a CI based task
  depend on a non-CI based task.

#### 🐞 Fixes

- Fixed `moon project-graph <id>` not including all dependencies/dependents. It was only showing
  direct relationships.
- Fixed an issue where touched file paths would include Git submodule directories and trigger hasher
  warnings.

#### ⚙️ Internal

- Updated dependencies.
- Updated proto to v0.42.2 (from 0.42.0).

## 1.29.4

#### 🚀 Updates

- Added an `experiments.strictProjectIds` setting to `.moon/workspace.yml`. When enabled, will
  disallow using original IDs for renamed projects (via the `id` setting) when building the project
  graph.
- Updated codegen/template `destination` to be relative from the workspace root if prefixed with
  `/`, otherwise the current working directory.

#### 🐞 Fixes

- Fixed an issue where moon would attempt to execute a folder if it has the same name as the current
  shell.
- Fixed an issue where `[working_dir]` and `[workspace_root]` variables were not working in the
  `template.yml` `destination` setting.

#### ⚙️ Internal

- Updated dependencies.
- Updated proto to v0.42.0 (from 0.41.3).

## 1.29.3

#### 🐞 Fixes

- Fixed an issue running `npx` commands (primarily through lockfile deduping).

## 1.29.2

#### 🚀 Updates

- Removed the warning around `.env` files not existing in certain environments.

#### 🐞 Fixes

- Fixed an issue where the task option `timeout` would apply to the overall run, and not for each
  attempt when using the `retryCount` option.

#### ⚙️ Internal

- Updated Rust to v1.82.

## 1.29.1

#### 🚀 Updates

- Added a new task option `merge`, that defines the same strategy for all applicable fields. Can be
  overridden with `mergeX` task options.
- Added a `moon` setting to `.moon/toolchain.yml`, which can be used to customize the update check
  process.

#### 🐞 Fixes

- Fixed `moon query projects` including dependency projects by default. Can be controlled with
  `--upstream`.
- Fixed `moon query projects` regex patterns not working when used in conjunction with affected.
- Fixed Bash-based hooks being generated with the wrong path separators on Windows.
- Fixed an issue where an inherited task with merge strategy "replace" will accidentally remove task
  args, deps, and env in inheriting tasks.
- Fixed an issue where token variables and environment variables used within task args together
  would not be replaced correctly.

## 1.29.0

#### 🚀 Updates

- Implemented a new affected project tracker that is more accurate and more powerful.
  - Can now control the depth of upstream (dependencies) and downstream (dependents).
  - Affected information now logs more information and is included in reports/JSON.
  - Added `--upstream` and `--downstream` options to `moon query projects`.
  - Coming soon for affected tasks as well!
- Implemented experimental support for a new configuration language, [Pkl](https://pkl-lang.org)!
  - Pkl is programmable, allowing for variables, loops, conditionals, and more.
- Added a new task option, `cacheLifetime`, that controls how long a task will be cached for.
- Added a new task merge strategy, `preserve`, that preserves the original inherited value.
- Added a new setting `vcs.hookFormat` to `.moon/workspace.yml`, that can customize the shell/file
  format for hooks.
- Added "sync workspace action" support to toolchain plugins. This is our first step in supporting
  toolchains via WASM plugins.
- Updated task `outputs` to support token and environment variables.
- Updated `moon query projects` to include the project description as a trailing value.
- Updated `moon query tasks` to include the task type and platform, and the task description as a
  trailing value.

#### 🐞 Fixes

- Fixed an issue where a root project in a polyrepo would not default to `**/*` inputs for tasks.
- Potential fix for an issue that occurs when multiple toolchains of the same type (2 different
  Node.js versions for example) would fail in weird ways when installing in parallel.

#### ⚙️ Internal

- Updated identifiers and targets to use [compact strings](https://crates.io/crates/compact_str).
- Updated proto to v0.41.3 (from 0.40.4).
- Updated wasmtime to v23 (from v17).

## 1.28.3

#### 🐞 Fixes

- Fixed an issue where VCS hooks were not being generated in Docker (requires `.git` to exist).
- Fixed an issue where projects whose `id` was renamed in `moon.yml` wasn't being reflected when
  inheriting project dependencies through the toolchain/platform integration.
- Fixed `optionalDependencies` not being considered a project relationship for Bun/Node implicit
  `package.json` detection.

## 1.28.2

#### 🚀 Updates

- Updated cached task outputs to now be prefixed with the task target when printing to the console.
- Updated Bun/Yarn lockfile parsing to temporarily log a warning on parse failure instead of exiting
  with an error. This change was made as there are currently no actionable or human-readable error
  messages.

#### 🐞 Fixes

- Fixed an issue where empty lines were being trimmed while streaming task console output.
- Fixed an issue where tasks that errored _before_ the command is executed, would swallow the error
  in `moon ci`.
  - It would also put the task in a non-failed state depending on the order of operations. This
    should be correct now.

## 1.28.1

#### 🚀 Updates

- Added caching around `bun bun.lockb` commands, instead of running them for every task.
- Updated and loosened identifier naming restrictions.
- Updated environment variable substitution to support different outputs when a variable is missing,
  based on a trailing flag syntax.
  - `$FOO` or `${FOO}` - If variable is missing, keeps the original syntax (current default).
  - `$FOO?` or `${FOO?}` - If variable is missing, replaces with an empty string.
  - `$FOO!` or `${FOO!}` - Ignores variable substitution and preserves the syntax (without !).

#### 🐞 Fixes

- Fixed an issue where an affected task with files that contain non-standard characters would fail
  to run because Bash expansion fails. We now quote file paths that contain `*`, `$`, `+`, and `[]`.

#### ⚙️ Internal

- Updated Rust to v1.81.

## 1.28.0

#### 💥 Breaking

- Extensions (WASM plugins) must implement the `register_extension` API. All built-in extensions
  have been updated.

#### 🚀 Updates

- Resolved the `actionPipelineV2` experiment and the new pipeline is now always enabled. The old
  pipeline code has been deleted. If you run into any problems, please report an issue!
- Resolved the `strictProjectAliases` experiment and enabled its functionality going forward.
- Added a `preset` task setting, that enables a collection of task options.
  - Will be replacing the `local` setting, which is now deprecated.
- Added an `os` task option, which allows a specific operating system to be targeted for the task.
- Added a `@meta` token function that can access project metadata.
- Added `$projectName`, `$projectChannel`, and `$projectOwner` token variables.
- Added `codeowners.requiredApprovals` setting to `.moon/workspace.yml`, that will apply
  requirements to all paths.
- Added a new experiment `disallowRunInCiMismatch`, that errors when a task that is `runInCI`
  enabled depends on a task that is `runInCI` disabled. These kinds of relationships cause a handful
  of issues in the pipeline.
  - Currently this only applies to `moon ci` and may also apply to `moon run` in the future.
- Updated the terminal checkpoint (4 squares) colors to better reflect their state.
  - Cached tasks continue to be blue, but non-cached but successful tasks are now green.
  - Announcements are now yellow.
- Updated `MOON_TOOLCHAIN_FORCE_GLOBALS` to support filtering by tool IDs, instead of applying to
  everything.

#### ⚙️ Internal

- We've sunset the CI insights feature in moonbase. We will no longer be tracking CI run history.
  This is a retroactive change that applies to all moon versions.
- Updated proto to v0.40.4 (from v0.39.7).

## 1.27.10

#### 🐞 Fixes

- Fixed an issue around running dependents when a dependency is affected in CI.

## 1.27.9

#### 🐞 Fixes

- Fixed a regression where the "primary target" detection would include far too many targets. This
  would result in passthrough arguments being unintentionally passed deeper.
- Fixed a regression where dependent tasks would always run in CI.

## 1.27.8

#### 🚀 Updates

- We now log messages when inheriting values from the root `.prototools`.

#### 🐞 Fixes

- Fixed a panic that occurs in the action graph builder.
- Fixed an issue where the root `.prototools` wasn't always loaded.

## 1.27.7

#### 🐞 Fixes

- Fixed an issue where codeowners `requiredAppprovals` wouldn't allow `1`.
- Fixed an issue where a task that depends on another task that has `runInCI: false`, would not run
  as affected in `moon ci` because the dependency task was skipped.

## 1.27.6

#### 📢 Notice

There have been reports of the `moon.exe` binary being flagged as a Trojan on Windows. This looks to
be a common false positive (`Wacatac.B!ml`) that targets Rust built binaries (as seen in the
[Tauri project as well](https://github.com/tauri-apps/tauri/issues/2486)).

While this isn't a simple fix on our end, we're publishing a new binary to replace the old one. You
may also have to unquarantine moon on your system. And lastly, we also suggest
[adding an exclusion for `moon.exe` to Windows Defender](https://support.microsoft.com/en-gb/windows/add-an-exclusion-to-windows-security-811816c0-4dfd-af4a-47e4-c301afe13b26).

#### ⚙️ Internal

- Updated proto to v0.39.7 (from v0.38.3).
- Updated Rust to v1.80.
- Updated dependencies.

## 1.27.5

#### 🐞 Fixes

- Fixed an issue where token expansion would clobber variable replacement when multiple variables
  are used.
- Fixed internal tasks being displayed in `moon query` results.

## 1.27.4

#### 🐞 Fixes

- Fixed an issue where `moon docker scaffold` would not respect `.gitignore` files.
- Fixed an issue where task hashing would potentially hash too many files when extracting touched
  files from `git status`.
- Fixed an issue where the automatic dependency installation wasn't triggering when switching
  branches.

## 1.27.3

#### 🚀 Updates

- Added `project.metadata` to `moon.yml` so that custom fields can be defined.
- Improved some task related error messages to include more information.

#### 🐞 Fixes

- Fixed an issue where the new action pipeline wouldn't wait for in-flight moonbase (remote caching)
  requests to finish. The old pipeline was not affected.
- Fixed `@in` and `@out` tokens not being able to reference other token functions.

## 1.27.2

#### 🐞 Fixes

- Fixed an issue where file groups expanded with task `script` or `env` weren't resolving relative
  to the project or workspace root (based on options).

#### ⚙️ Internal

- Updated proto to v0.38.3 (from v0.38.2).

## 1.27.1

#### 🚀 Updates

- Removed the concurrency limit when running persistent tasks.

#### 🐞 Fixes

- Fixed an issue where task scripts would error with "command not found" (caused by over
  escaping/quoting).
- Fixed an issue where a persistent task may not run or error with a missing dependency hash, if ran
  alongside many other persistent tasks.
- Fixed an issue where "build" dependencies were being synced as project references.
- Fixed an issue where the install dependencies action wouldn't run if the vendor directory
  (node_modules) was missing, but our operation was previously cached.
- Fixed an issue where token variables were not being replaced in task scripts.

## 1.27.0

#### 🚀 Updates

- Enabled the `experiments.actionPipelineV2` setting in `.moon/workspace.yml` by default.
  - If you run into any issues running tasks, disable this setting and report an issue.
- Added a new task field `script`, as an alternative to `command` + `args`.
  - Scripts support multiple commands (`&&`), pipes, and redirects.
  - Scripts do not support task inheritance argument merging.
  - Scripts are always ran in a shell.
- Added a new command, `moon docker file`, that can be used to generate an initial multi-stage
  `Dockerfile` for a project.
- Added new `docker` settings to both `.moon/workspace.yml` and `moon.yml`, allowing it to be
  configured at the workspace and project levels.
- Added support for [murex](https://murex.rocks/) shells.
  - Updated both `unixShell` and `windowsShell` task options.
- Improved the "automatically install dependencies if a manifest/lockfile has changed" flow. This
  should trigger less than before.
- We now generate JSON schemas for our configuration files to `.moon/cache/schemas`, so that they
  can be dynamically created based on the current moon version and environment.
  - Is automatically generated when running a task.
  - Added a `moon sync config-schemas` command to manually run this operation.
- When writing JSON and YAML files, we attempt to write back to the file with its original
  indentation.

#### 🐞 Fixes

- Fixed action operations not being present in the run report when a task fails to run.
- Fixed stdout/stderr not being included in the summary review of failed tasks.

#### 🧰 Toolchain

- Node
  - Added a `node.syncPackageManagerField` setting, to control the `packageManager` field in
    `package.json`.
- Yarn
  - Will no longer call `yarn set version` and instead rely entirely on proto's toolchain.

#### ⚙️ Internal

- Updated proto to v0.38.2 (from v0.37.2).

## 1.26.7

#### 🚀 Updates

- Updated `bun.installArgs` and `node.bun.installArgs` to stay in sync.

#### 🐞 Fixes

- Fixed `moon run --affected` not auto-installing dependencies in CI.
- Fixed an issue where the project graph cache would not invalidate based on certain environment
  variables.
- Fixed an issue where our console would be prematurely dropped, causing output flushing issues.

## 1.26.6

#### 🐞 Fixes

- Fixed a regression where `pwsh.exe` would not fallback to `powershell.exe` when the former does
  not exist.
- Respect `CARGO_HOME` during automatic rustup installation.

## 1.26.5

#### 🚀 Updates

- Updated home-based environment variables, like `CARGO_HOME`, to support relative paths.

#### 🐞 Fixes

- Fixed an issue where globs wouldn't match when leading with `./`.

#### ⚙️ Internal

- Updated proto to v0.37.2 (from v0.37.1).

## 1.26.4

#### 🐞 Fixes

- Fixed an issue where the shell could not be detected, and would potentially hang.

## 1.26.3

#### 🐞 Fixes

- Potential fix for a deadlock that occurs when running `moon docker scaffold`.
- Reverted the default shell back to `sh` instead of `bash` when one could not be detected.

## 1.26.2

#### 🐞 Fixes

- Fixed the `ciReport.json` file not being created with the experimental pipeline.
- Fixed the wrong version being displayed in `--version` and in logs.

#### ⚙️ Internal

- Downgraded WASM runtime to fix potential issues.

## 1.26.1

#### 🐞 Fixes

- Re-released because of an npm version mismatch issue.

## 1.26.0

#### 💥 Breaking

- Reworked webhooks to more closely align with our current data structures.
  - Renamed `target.*` events to `task.*`, and `*.finished` to `*.completed`.
  - Removed the `pipeline.aborted` event. Instead, an `aborted` field is now passed to
    `pipeline.completed`.
  - Replaced the `action` field with a new `node` field within `task.*` events.
  - Reworked `pipeline.completed` entirely. Instead of calculating values for you, we now pass all
    results allowing you to calculate them yourself.

#### 🚀 Updates

- Rewrote the actions/tasks pipeline from the ground-up. Is currently experimental and must be
  enabled with the `experiments.actionPipelineV2` setting in `.moon/workspace.yml`.
  - Increased performance.
  - Better concurrency handling and scheduling.
  - More accurately monitors signals (ctrl+c) and shutdowns.
  - Tasks can now be configured with a timeout (`options.timeout` setting).
  - Some operations within actions are now ran in parallel.
  - We renamed many of the action labels (`SyncNodeProject(app)` -> `SyncProject(node, app)`).
- Added a global `--dump` flag, that will dump a trace profile that can be inspected in
  `chrome://tracing`.
- Updated `moon completions` command to support Nushell.
- Updated task option `unixShell` with new options: ion, nu (nushell), pwsh (powershell), xonsh.
- Updated task option `windowsShell` with new options: elvish, fish, nu (nushell), xonsh.
- Updated CLI command execution to be more performant, and to reduce our usage of concurrent locks.
  - Internal components (like cache engine, or project graph) are now lazy-loaded when required,
    instead of created upfront.

#### ⚙️ Internal

- Updated proto to v0.37.1 (from v0.36.2).
- Updated Rust to v1.79.

## 1.25.6

#### 🐞 Fixes

- Fixed a potential deadlock when installing tools.

#### ⚙️ Internal

- Updated proto to v0.36.2 (from v0.36.0).

## 1.25.5

#### 🐞 Fixes

- Fixed an issue where multiple Bun tools would try to install and collide.
- Fixed an issue where the `package.json` `packageManager` field would be set with an invalid
  version specifier.

## 1.25.4

#### 🚀 Updates

- Updated `bun.version` and `node.bun.version` to stay in sync when one is defined and the other
  isn't. This helps to avoid tool discrepancies.

#### 🐞 Fixes

- Fixed an issue where persistent tasks depending on each other would sometimes error with
  "Encountered a missing hash".
- Fixed nightly and canary releases not showing the correct version in `moon --version`.

## 1.25.3

#### 🚀 Updates

- Improved error messages around git version and worktree parsing.

#### 🐞 Fixes

- Fixed `git` version parsing when the version contains invalid semver parts.

#### 🔋 Extensions

- Updated `download` to v0.0.5.
- Updated `migrate-nx` to v0.0.5.
- Updated `migrate-turborepo` to v0.1.2.
  - Added Turborepo v2 support.

## 1.25.2

#### 🚀 Updates

- Added a check to `moon docker scaffold` that ensures that `.moon/cache` is ignored in a root
  `.dockerignore` file. This helps to avoid interoperability issues.
- Added more logs to `moon docker` commands to help uncover future issues.

#### 🐞 Fixes

- Fixed an issue where `noop` tasks would not cache / invalidate cache. This is a regression from
  the recent task runner changes.

#### ⚙️ Internal

- Updated proto to v0.36.0 (from v0.35.4).

## 1.25.1

#### 🚀 Updates

- Rewrote process failure error messages to include exit status information. This should help
  uncover processes killed by signals, and help debug the -1 exit code issues.

## 1.25.0

#### 💥 Breaking

- Removed the following webhook events associated with task outputs: `target-output.archiving`,
  `target-output.archived`, `target-output.hydrating`, `target-output.hydrated`,
  `target-output.cache-check`.

#### 🚀 Updates

- Rewrote the task runner from the ground up:
  - Improved handling and reliability of output archiving and hydration.
  - Streamlined the task execution (child process) flow.
  - Now tracks metrics for individual operations, like hash generation, output hydration, task
    execution, and more. Can be inspected in the run report.
- Added a `--summary` flag to `moon run` and `moon check` that will include a summary of all actions
  that were processed/failed within the pipeline. This is the same output used in `moon ci`.
- Added a new console reporting layer that handles the rendering of output in the terminal.
  - This enables us to support additional reporters in the future, each with unique UIs.
  - Slightly tweaked our current UI rendering. You may notice some differences.
- Updated external configuration files (via https extends) to be cached for 24 hours.
  - This will fix issues with offline mode.
- Greatly reduced the amount of concurrent locks being held during task execution. May see slight
  performance improvements.

#### 🐞 Fixes

- Fixed an issue where actions within the run report were not reflecting the correct status of their
  last execution attempt.
- Fixed an issue where "have outputs been created" checks would fail if outputs only contained
  negated globs, coupled with literal paths.
- Fixed an issue where `.prototools` in the workspace root was not being respected when running moon
  commands in a sub-directory.
- Fixed `PROTO_*_VERSION` environment variables being set to `*`, resulting in unexpected versions
  being resolved.

#### ⚙️ Internal

- Updated proto to v0.35.4 (from v0.34.4).
- Updated macOS binaries to be built on macos-12 instead of macos-11.

## 1.24.6

#### 🐞 Fixes

- Reworked the binary provided by `@moonrepo/cli` to work better on Windows.

## 1.24.5

#### 🐞 Fixes

- Fixed an issue where proto managed tools may error with "Failed to detect an applicable version".

## 1.24.4

#### 🐞 Fixes

- Fixed a regression where `runInCI` was being overzealously applied to `moon run` commands.
- Fixed generated VCS hooks not containing a trailing newline.

## 1.24.3

#### 🐞 Fixes

- Fixed an issue where internal tasks would still run when running a task using "closest project"
  detection.
- Fixed an issue where task's with `runInCI` weren't always being filtered properly.

## 1.24.2

#### 🐞 Fixes

- Fixed task `deps.env` not supporting variable substitution.
- Fixed an issue where Git hooks would overwrite non-local hooks. The `core.hooksPath` setting is
  now only used if the path is within the current repository.

## 1.24.1

#### 🐞 Fixes

- Fixed an issue where versions in `.prototools` weren't being respected.
- Fixed task `deps.args` and `deps.env` not expanding tokens correctly.

## 1.24.0

#### 🚀 Updates

- Added an experimental `moon templates` command, that lists all available codegen templates.
- Added a `--dependents` flag to `moon project-graph <id>` and `moon query projects`, to include
  downstream dependents of a focused/affected project.
- Added a `mutex` task option, allowing for exclusivity and to ensure only 1 task is running at a
  time for the same mutex.
- Added a `runner.autoCleanCache` setting to `.moon/workspace.yml`, allowing the post-run clean
  mechanism to be controlled.
- Updated `moon ci` to automatically determine base/head revisions based on your current CI provider
  (when applicable).
- Updated `moon generate`:
  - When passing variables as command line arguments, they are now entirely modeled after the
    template configuration.
    - Booleans and negated booleans now work better.
    - Numbers now support negative values.
    - Multiple values can now be passed for enums when `multiple` is enabled.
  - If a variable value is passed as an argument, we no longer prompt for it.
  - Internal variables will now error when passed as an argument.
- Updated action graph and project graph visualization:
  - Slightly tweaked the colors to be easier to read.
  - Updated edges to use chevron arrows.
  - Added a new layout system to organize node/edges, controlled by the `?layout=` query parameter.
  - Supported layout options: `dagre` (default), `klay`, `grid`, `breadthfirst`
- Updated root-level tasks to have no inputs by default, instead of `**/*`. This is typically what
  users want, to avoid greedy tasks.

#### 🐞 Fixes

- Fixed YAML schema validation not allowing custom languages for the project `language` field.
- Fixed an issue where Bun and Node would both attempt to install dependencies, resulting in
  collisions.
  - To resolve this issue, we currently prioritize Node over Bun if both tools are enabled.
  - If you have both and want to use Bun, set Node's package manager to
    `node.packageManager: 'bun'`.
- Attempted fix for "too many open files" when moon is cleaning cached artifacts.

#### ⚙️ Internal

- Updated proto to v0.34.4 (from v0.32.2).

## 1.23.4

#### 🐞 Fixes

- Fixed an issue where leading `./` in input/output globs would cause matching failures.
- Fixed an issue where root-level projects were not being marked as affected in `moon query`.
- Fixed an issue where `moon docker scaffold` would copy all sources when a project depends on a
  root-level project.

## 1.23.3

#### 🧩 Plugins

- Updated `bun_plugin` to v0.11.
  - Added Windows support.
  - Will now use the baseline build on x64 Linux when available.

#### ⚙️ Internal

- Updated proto to v0.32.2 (from v0.32.1).

## 1.23.2

#### 🐞 Fixes

- Fixed an issue where input environment variables weren't always being taken into account for task
  hashes.

## 1.23.1

#### 🚀 Updates

- Added more CI/CD platforms to check for.

#### 🐞 Fixes

- Fixed an issue where `moon clean` wasn't removing nested files.
- Fixed an issue where `package.json` syncing would create incorrect `link:`s for Bun.
- Fixed an issue where `moon ext` would trigger a "No such file or directory" error.

#### 🔋 Extensions

- Updated `migrate-nx` to v0.0.3.
  - Fixed invalid IDs when converting `package.json` names.

## 1.23.0

#### 🚀 Updates

- Added `git:` and `npm:` locators to the `generator.templates` setting in `.moon/workspace.yml`.
  - This allows templates to be packaged and managed outside of the workspace.
  - Locations will be cloned/downloaded on-demand.
- Added an `id` setting to `template.yml`, so that templates can customize their name (instead of
  using the folder name).
- Added a `variables()` function for templates that returns an object of all variables available.
- Added new functionality for template variables in `template.yml`:
  - New `order` setting to control the order in which they are prompted for.
  - New `internal` setting that ignores values passed on the command line.
  - Updated enum `default` settings to support an array of values.
- Added an `internal` task option, which marks tasks as internal only.
- Updated task inheritance to support stack-based configuration, such as
  `.moon/tasks/node-frontend.yml` or `.moon/tasks/bun-backend-application.yml`.
- Updated `moon project` and `moon task` to include the configuration files that tasks inherit from.
- Updated `moon task` to include the modes it belongs to.

#### 🐞 Fixes

- Fixed an issue where a project's `platform` was being detected as `node` (when not enabled), and
  should have been `bun`. If you're using both `bun` and `node` in the same workspace, moon has a
  hard time detecting which should be used for what project. If you run into issues, explicitly set
  the `platform` in the project's `moon.yml`.
- Fixed an issue where template files couldn't import/include/extends files from extended templates.
- Fixed template enum variable default values being able to use a non-supported value.

#### ⚙️ Internal

- Configuration JSON schemas are now included within each GitHub release.
- Updated proto to v0.32.1 (from v0.31.4).
- Updated Rust to v1.77.

## 1.22.10

#### ⚙️ Internal

- Added more logging around our WASM plugins.
- Added a `MOON_DEBUG_WASM` environment variable, for including additional logging output, and
  optionally dumping memory/core profiles.

## 1.22.9

#### 🐞 Fixes

- Fixed an issue with `moon docker scaffold` where Rust projects in the workspace skeleton would
  fail to compile as they were missing a lib/main entry point.
- Fixed an issue with `moon docker prune` where an unknown project type would trigger toolchain
  errors.

## 1.22.8

#### 🐞 Fixes

- Fixed an issue where task hashing would attempt to hash invalid file paths, when moon is located
  within a nested git repository.

## 1.22.7

#### 🐞 Fixes

- Fixed an issue where environment variable substitution would trigger recursively when referencing
  itself.

## 1.22.6

#### 🚀 Updates

- We now include the exit code of a failing task in the logs for easier debugging.

#### 🐞 Fixes

- Fixed an issue where the wrong path was being displayed for the task message "in ...".

#### ⚙️ Internal

- Updated proto to v0.31.4 (from v0.31.2).

## 1.22.5

#### 🐞 Fixes

- Fixed `env` variable substitution not being able to reference values from an `.env` file.
- Fixed an issue where moon would move an existing proto binary when installing proto, triggering
  permission issues.

## 1.22.4

#### 🐞 Fixes

- Fixed an issue where deleted but uncommitted files would log a hashing warning.
- Fixed an issue where parsing `bun.lockb` would fail if using `github:` protocols.

#### ⚙️ Internal

- Updated proto to v0.31.2 (from v0.30.2).

## 1.22.3

#### 🚀 Updates

- Updated our project constraint enforcement to take the new `stack` setting into account. For
  example, frontend applications can now depend on backend applications, where as previously they
  could not.

## 1.22.2

#### 🐞 Fixes

- Fixed an issue where VCS hooks were being created in Docker, triggering cache issues.

## 1.22.1

#### 🚀 Updates

In v1.22, we [made a change](https://github.com/moonrepo/moon/issues/1329) to affected tasks that
pass all `inputs` as arguments, instead of passing `.`. This change was made to not overzealously
pass files to the task that it doesn't care about, but it ended up causing problems for certain
commands.

We didn't want to revert the change, but it also wasn't easy to fix without causing other issues, so
as a compromise, we opted to introduce a new task option, `affectedPassInputs` to handle this
functionality.

## 1.22.0

#### 🚀 Updates

- Added `configuration` and `scaffolding` variants to the project `type` setting in `moon.yml`.
  - Updated project constraints to support these new variants.
- Added a `stack` setting to `moon.yml`, for categorizing which tech stack it belongs to.
  - Supports `frontend`, `backend`, `infrastructure`, and `systems`.
  - Added a `projectStack` field to the query language (MQL).
  - Added a `$projectStack` token variable for tasks.
  - Updated the `moon query projects` command to support a `--stack` option, and include the stack
    in the output.
  - Updated the `moon project` command to include the stack in the output.
- Added a `description` setting for tasks, for providing human-readable information.
  - Updated the `moon project` and `moon task` commands to include the description in the output.
- Added an `installArgs` setting for bun/npm/pnpm/yarn in `.moon/toolchain.yml`, to customize the
  args used when installing dependencies.
- Added a new built-in extension, `migrate-nx`, for migrating from Nx to moon.
  - Will convert all `nx.json`, `workspace.json`, and `project.json` files.
- Updated task input environment variables to support a wildcard match using `*`, for example
  `$VITE_*`.
  - This will include all environment variables in the current process that starts with `VITE_`.
- Updated the `envFile` task option to support a list of file paths.
- Updated the `migrate-turborepo` extension.
  - Removed the requirement of moon's project graph. Will now scan for turbo.jsons instead.
- Updated affected tasks to use `inputs` as the list of files provided, instead of `.`.

#### 🐞 Fixes

- Fixed an issue where `bun install` was not running with `--production` in Docker prune.
- Fixed an issue where invalid IDs passed to certain commands would trigger a panic.
- Fixed an issue where `$PWD` in a task was pointing to the wrong directory.

#### 🧩 Plugins

- Updated `deno_plugin` to v0.9.1.
  - Added Linux ARM64 support (requires Deno >= v1.41).
- Updated `rust_plugin` to v0.8.1.
  - Uses the full triple target when installing and uninstalling toolchains.

#### ⚙️ Internal

- Updated Rust to v1.76.
- Updated proto to v0.30.2 (from v0.29.1).

## 1.21.4

#### 🐞 Fixes

- Fixed VCS hooks on Windows generating invalid PowerShell commands.

## 1.21.3

#### 🐞 Fixes

- Fixed a panic that would occur when running an action and path stripping would fail.

## 1.21.2

#### 🐞 Fixes

- Attempted fix for `liblzma.5.dylib` issues on macOS arm64.

## 1.21.1

#### 🚀 Updates

- Added shallow checkout detection to help avoid failing Git commands.
  - If detected in `moon ci`, is a hard failure.
  - If detected in `moon run`, will disable affected checks.

## 1.21.0

#### 🚀 Updates

- Added Deno tier 3 support.
  - Will download and install Deno into the toolchain when a `version` is configured.
  - Will parse the `deno.lock` lockfile to extract and resolve dependencies.
  - Will hash manifests and inputs for Deno specific caching.
  - Added a `deno.version` setting to `.moon/toolchain.yml`.
  - Added a `toolchain.deno` setting to `moon.yml`.
  - Updated `moon bin` and `moon docker` commands to support Deno.
- Added a new built-in extension, `migrate-turborepo`, with new functionality.
  - Replaces the previous `moon migrate from-turborepo` command.
  - Added Bun support behind a new `--bun` flag.
  - Added support for `globalDotEnv`, `dotEnv`, and `outputMode`.
  - Scripts now run through a package manager, instead of `moon node run-script`.
  - Root-level tasks will now create a root `moon.yml`, instead of warning.
- Added `unixShell` and `windowsShell` task options, so that the underlying shell can be configured
  per task.
- Added `bun.inferTasksFromScripts` setting to `.moon/toolchain.yml`, for compatibility with
  Node.js.
- Added environment variable support to `fileGroups`.
- Added a `@envs(group)` token function for referencing environment variables.
- Added a `--quiet` global argument, for hiding non-critical moon output.
- Deprecated the `moon node run-script` command. Run the task through a package manager instead,
  like `npm run` or `yarn run`.
- Updated tasks with glob-like arguments to automatically enabled the `shell` option, so that glob
  expansion works correctly.
- Updated interactive tasks to not be shutdown when receiving a CTRL+C signal, and instead allow
  them to handle it themselves, and cleanup if necessary.
- Implemented a new console layer for writing to stdout/stderr.
  - Logs are now buffered and written periodically.
  - Previously they were written immediately, which required locking std each call.
  - Should see some minor performance improvements.

#### 🐞 Fixes

- Fixed an issue where the action graph would create incorrect nodes when a tool utilizes dependency
  workspaces, and a project is not within the workspace.
- Fixed an issue where glob based arguments were overlay escaped.
- Fixed console checkpoints (the 4 squares) showing the wrong working directory.

#### ⚙️ Internal

- Updated proto to v0.30.0 (from v0.29.1).

## 1.20.1

#### 🚀 Updates

- Removed the maximum concurrency limit from persistent tasks.

#### 🐞 Fixes

- Fixed `moon docker scaffold` not copying the project specific `moon.yml` file, resulting in a
  skewed project graph.

## 1.20.0

#### 🚀 Updates

- Added a new extension plugin system.
  - An extension is a WASM plugin that is not built into moon's core:
    https://github.com/moonrepo/moon-extensions
  - Extensions can be executed with the new `moon ext` command.
  - The community can build and publish their own extensions!
- Added a `taskOptions` setting to `.moon` task configs, allowing default task options to be
  defined.
  - These options will be merged and inherited as part of the configuration chain.
- Added an `optional` field to task `deps`, allowing the dependency to be optional during
  inheritance.
- Added a "Tags" view to the VSCode extension.
- Updated proto installation to trigger for all applicable commands, not just `moon run`,
  `moon check`, and `moon ci`.
  - Will also use the global proto version if available when there's no internet connection, and the
    moon required proto version has not been installed.

#### 🐞 Fixes

- Fixed Git version parsing when using VFSForGit.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.9.
- Updated `node_plugin` and `node_depman_plugin` to v0.9.
  - Changed the `bundled-npm` and `intercept-globals` settings to be `false` by default (instead of
    `true`).
- Updated `rust_plugin` to v0.8.

#### ⚙️ Internal

- Updated proto to v0.29.1 (from v0.26.4).

## 1.19.3

This fixes a bad 1.19.2 release.

## 1.19.2

#### 🐞 Fixes

- Fixed another location where the `proto` binary was not available.

## 1.19.1

#### 🐞 Fixes

- Fixed `proto` binary not being available in a Dockerfile when running `moon docker` commands.
- Fixed our `@moonrepo/cli` postinstall script not working correctly for Bun Arm64.

## 1.19.0

#### 💥 Breaking

- Removed the `experiments.interweavedTaskInheritance` setting from `.moon/workspace.yml`.
  Interweaved inheritance is now always enabled (was previously true by default).
- Removed the `experiments.taskOutputBoundaries` setting from `.moon/workspace.yml`. We opted to
  remove boundaries entirely, as they caused more problems than solved. Task outputs may now overlap
  without issue.

#### 🚀 Updates

- Updated `implicitDeps` in `.moon/tasks.yml` and task `deps` in `moon.yml` to support arguments and
  environment variables for the dependency target.
- Updated the action graph and pipeline to _not_ run the same target (but with different arguments
  and environment variable variations) in parallel, to avoid unexpected collisions.
- Updated VS Code extension to support multiple VS Code workspace folders.
- Improved code generation and templates:
  - Added a `destination` field to `template.yml`, to customize a default location.
  - Added a `extends` field to `template.yml`, allowing templates to extend and inherit other
    templates.
  - Updated `[var]` syntax to support filters: `[var | camel_case]`.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.7.
- Updated `node_plugin` and `node_depman_plugin` to v0.7.
- Updated `rust_plugin` to v0.6.

#### ⚙️ Internal

- Updated Rust to v1.75.
- Updated proto to v0.26.4 (from v0.25).

## 1.18.5

#### 🚀 Updates

- Reworked duplicate project ID/alias detection to be more accurate. Will also now error instead of
  warn when a duplicate is detected.
  - For aliases, the error can be disabled with the new `experiments.strictProjectAliases` setting
    in `.moon/workspace.yml`.
  - For project IDs, the error can not be disabled, as conflicting IDs will cause issues with the
    project graph.

#### 🐞 Fixes

- Fixed glob based project locating to not log warnings when a file is found and it starts with `.`
  (ignore dotfiles).
- Fixed project aliases potentially overwriting a project with the same name.

## 1.18.4

#### 🚀 Updates

- Updated the proto installation step to download, unpack, and install using Rust, instead of
  relying on our Bash/PowerShell scripts. This should remove the requirement that openssl, tar, and
  other environment tools must exist.

#### ⚙️ Internal

- Updated proto to v0.25.3.

## 1.18.3

#### 🐞 Fixes

- Fixed more issues in relation to custom project IDs not resolving correctly.

#### ⚙️ Internal

- Improved some error messages with more information.

## 1.18.2

#### 🚀 Updates

- Silenced proto migration warnings when ran in the context of moon.

#### 🐞 Fixes

- Fixed an issue where `@dirs` and `@files` tokens didn't always work correctly in `outputs`.
- Fixed the `@moonrepo/cli` package pulling in different `@moonrepo/core-*` versions

#### ⚙️ Internal

- Updated proto to v0.25.2.

## 1.18.1

#### 🐞 Fixes

- Fixed an issue where we would install `proto` even when not required.
- Fixed an issue where implicit dependencies were not resolving correctly when projects were
  renamed.

## 1.18.0

#### 🚀 Updates

- Rewrote toolchain based task running to use a path based approach.
  - Instead of manually locating an executable, we now rely on `PATH` to locate the executable.
  - Non-system tasks can now be wrapped in a shell using the `shell` option.
  - This approach will now benefit from proto shims and binaries.
  - We'll also download and install the `proto` binary if it does not exist.
- Reworked the `moon init` command.
  - Will no longer scaffold the toolchain configuration by default.
  - The tool to scaffold into a toolchain can be passed as an argument.
  - The path to initialize in is now behined the `--to` option.
  - Added support for the `bun` tool.
  - Simplified the workflow overall.
- Updated `moon.yml` to support customizing the project name using the `id` field.
  - Can be used to override the project name derived in `.moon/workspace.yml`.
- Added a `MOON_INSTALL_DIR` environment variable, to control where the `moon` binary is installed
  to.

#### 🐞 Fixes

- Fixed `moon upgrade` failing when not ran in a moon workspace.
- Fixed `CODEOWNERS` being written with double trailing newlines.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.6.
- Updated `node_plugin` and `node_depman_plugin` to v0.6.1.
- Updated `rust_plugin` to v0.5.

#### ⚙️ Internal

- Improved string allocation and performance for queries, task tokens, and process commands.
- Improved remote caching flow and handling.
- Updated proto to v0.25.

## 1.17.4

#### 🐞 Fixes

- Fixed an issue where executing moon (and indirectly proto) would run into privilege access issues
  on Windows.
- Fixed `typescript.includeProjectReferenceSources` and `typescript.syncProjectReferencesToPaths`
  settings not including project references that were manually added (not auto-synced).
- Fixed the "a project already exists with alias" warnings when using Bun and Node together.

#### ⚙️ Internal

- Added canary release support.
- Enabled wasmtime caching, which should improve performance of WASM plugins by 10-20%.
- Updated proto to v0.23.7.

## 1.17.3

#### 🐞 Fixes

- Fixed an issue where we would fail to find Cargo binaries on Windows.

#### ⚙️ Internal

- Updated proto to v0.23.3.

## 1.17.2

#### 🐞 Fixes

- Fixed an issue where `cargo-binstall` would error when trying to install it and it already exists.

## 1.17.1

#### 🐞 Fixes

- Fixed the wrong version being reported by the CLI.

## 1.17.0

#### 🚀 Updates

- Integrated full Bun support (tier 1-3).
  - Will download and install Bun into the toolchain when a `version` is configured.
  - Will parse the `bun.lockb` lockfile to extract and resolve dependencies.
  - Will hash manifests and inputs for Bun specific caching.
  - Added a `bun` setting to `.moon/toolchain.yml`.
  - Added a `toolchain.bun` setting to `moon.yml`.
  - Updated `moon bin` and `moon docker` commands to support Bun.
  - Updated task `platform` to support "bun".
- Improved TypeScript support.
  - Added a `typescript.root` setting to denote the TypeScript root.
  - Added a `typescript.includeSharedTypes` setting, for syncing a shared types path to all
    project's `include`.
  - Added a `typescript.includeProjectReferenceSources` setting, for syncing project reference files
    to all project's `include`.
  - Updated `typescript.syncProjectReferencesToPaths` setting to always include the wildcard, and
    not require an index file.
  - Improved project reference syncing and edge case handling.
- Improved JavaScript support.
  - Added `bun.rootPackageOnly` and `node.rootPackageOnly` settings to support the "one version
    rule" pattern.
  - Updated automatic dependency linking to use the `build` scope instead of `peer` scope. This
    should alleviate some of the pain points with `package.json` syncing.

## 1.16.5

#### 🐞 Fixes

- Fixed an issue where codegen would merge JSON/YAML files with the incorrect source.
- Updated file traversal to not walk outside of the workspace root.

#### ⚙️ Internal

- Updated Rust to v1.74.
- Updated proto to v0.23.0.
- Updated dependencies.
- Updated logs to now include nanoseconds.

## 1.16.4

#### 🚀 Updates

- Update project graph hashing to include git ignored `moon.yml` files.

#### 🐞 Fixes

- Fixed Yarn v1.22.x download not unpacking correctly.

#### 🧩 Plugins

- Updated Yarn `node_depman_plugin` to v0.5.1.

## 1.16.2/3

#### 🚀 Updates

- Updated `projects` globs to support ending in `moon.yml`.
- Updated `node.dependencyVersionFormat` to fallback to a supported format when the chosen
  `node.packageManager` does not support the configured (or default) version format.
- Updated to proto v0.22.0.

#### 🐞 Fixes

- Fixed an issue where dependencies were being injected into the root `package.json`, when a
  root-level project was dependending on non-root project tasks.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.5.
- Updated `deno_plugin` to v0.5.
- Updated `go_plugin` to v0.5.
- Updated `node_plugin` and `node_depman_plugin` to v0.5.
- Updated `python_plugin` to v0.2.
- Updated `rust_plugin` to v0.4.
- Updated `schema_plugin` (TOML) to v0.5.

## 1.16.1

#### 🐞 Fixes

- Fixed `moon ci` not treating dependents as "CI only" when running locally.
- Fixed the MQL parser failing on projects that contain a `.`.
- Fixed JSON comment stripping not handling docblock styled comments (`/** **/`).

## 1.16.0

#### 🚀 Updates

- Added Bun as a supported Node.js package manager: `node.packageManager: 'bun'`.
- Added components and targets support for the Rust toolchain.
  - Added `rust.components` and `rust.targets` settings to `.moon/toolchain.yml`.
  - Will automatically be installed with `rustup` when the pipeline is ran.
- Added a `MOON_TOOLCHAIN_FORCE_GLOBALS` environment variable, that will force all toolchain tools
  to use the global binary available on `PATH`, instead of downloading and installing.
- Added an improved task inheritance chain resolver.
  - Global and local tasks are now interweaved within the chain, where as previously global was
    built first, then local.
  - To fallback to the previous behavior, set `experiments.interweavedTaskInheritance: false` in
    `.moon/workspace.yml`.
- Added a new project type `automation`, for projects like E2E and integration testing.
- Updated action graph cycle detection to list all nodes in the cycle (when detectable).
- Updated all npx calls to use a package manager equivalent. For example: `yarn dlx`, `pnpm dlx`,
  `bunx`.
- Updated to support Yarn v4.

#### 🐞 Fixes

- Fixed an issue where `moon ci` and `git` would fail if there's only 1 commit on the base branch.
- Fixed an issue where `runInCI` was not respected when a task is a dependency of an affected task.
- Fixed an issue where the task `replace` merge strategy would not apply for empty values.

#### ⚙️ Internal

- Updated dependencies.
- Updated to proto v0.21.0.
- Pinned proto plugins to a fixed version instead of using latest.

## 1.15.4

#### 🐞 Fixes

- Fixed an issue where pnpm would fail to dedupe when its toolchain version is not using a
  fully-qualified version.
- Fixed an issue where `PROTO_OFFLINE` wouldn't use global binaries when available.

#### ⚙️ Internal

- Updated to proto v0.20.3.

## 1.15.3

#### 🐞 Fixes

- Fixed an issue where interactive/persistent flags weren't always bubbled up the the task runner.

#### ⚙️ Internal

- Updated to proto v0.20.

## 1.15.2

#### 🚀 Updates

- Updated `moon run --interactive` to allow more than 1 target.

#### 🐞 Fixes

- Fixed an issue where "raw" codegen files were sometimes being rendered, and failing with invalid
  syntax.
- Fixed an issue where task dependents for the non-primary targets were being included in the action
  graph.
- Fixed an issue with the project graph that would create duplicate nodes for deeply nested cycles.
- Fixed an issue where a cycle would be created in the action graph for the `SyncProject` action
  type.

## 1.15.1

#### 🚀 Updates

- Based on feedback, we've updated the automatic dependency linking to _not apply_ when the target
  is the root-level project. This should alleviate all unwanted cycles.

#### 🐞 Fixes

- Fixed an issue where Node.js dependency syncing would fail on `build` dependencies, and be over
  zealous with root-level projects.
- Improved detection of Rust `cargo-binstall` package.

## 1.15.0

#### 💥 Breaking

- Tasks that depend (via `deps`) on other tasks from arbitrary projects (the parent project doesn't
  implicitly or explicitly depend on the other project) will now automatically mark that other
  project as a "peer" dependency. For example, "b" becomes a peer dependency for "a".

#### 🎉 Release

- Rewrote the dependency graph from the ground-up:
  - Now known as the action graph.
  - All actions now depend on the `SyncWorkspace` action, instead of this action running
    arbitrarily.
  - Cleaned up dependency chains between actions, greatly reducing the number of nodes in the graph.
  - Renamed `RunTarget` to `RunTask`, including interactive and persistent variants.
- Updated the action graph to process using a topological queue, which executes actions on-demand in
  the thread pool when they are ready (dependencies have been met). Previously, we would sort
  topologically _into batches_, which worked, but resulted in many threads uselessly waiting for an
  action to run, which was blocked waiting for the current batch to complete.
  - For large graphs, this should result in a significant performance improvement.
  - Persistent tasks will still be ran as a batch, but since it's the last operation, it's fine.
- Released a new GitHub action,
  [`moonrepo/setup-toolchain`](https://github.com/marketplace/actions/setup-proto-and-moon-toolchains),
  that replaces both `setup-moon-action` and `setup-proto`.

#### 🚀 Updates

- Added a `moon action-graph` command.
- Added a `--dependents` argument to `moon action-graph`.
- Added the ability to skip non-`RunTask` actions using environment variables.
- Deprecated the `moon dep-graph` command.

#### 🐞 Fixes

- Fixed an issue where task dependents (via `moon ci` or `moon run --dependents`) wouldn't always
  locate all downstream tasks.

#### ⚙️ Internal

- Added in-memory caching to project graph file system lookup operations.
- Updated Rust to v1.72.

## 1.14.5

#### 🐞 Fixes

- Temporarily fixed the "A dependency cycle has been detected for (unknown)" issue.
- Fixed an issue where Git hooks were not created properly when using Git worktrees.
- Fixed a panic when attempting to clean/parse a JSON string.

## 1.14.4

#### 🐞 Fixes

- Fixed an issue where `moon docker scaffold` was too greedy and would copy files it shouldn't.
- Fixed some `PATH` inconsistencies when executing npm/pnpm/yarn binaries.
- Fixed codegen `lower_case` and `upper_case` stripping characters.

## 1.14.3

#### 🚀 Updates

- Updated `moon dep-graph` to support a task in closest project, similar to `moon run`.
- Updated to proto v0.19.

#### 🐞 Fixes

- Fixed an issue where local tasks could not extend global tasks using the `extends` setting.

## 1.14.2

#### 🐞 Fixes

- Fixed an issue where non-YAML files in `.moon/tasks` would be parsed as YAML configs.
- Fixed an issue where arguments were not passed to generated Git hooks.
- Fixed an issue where moonbase would fail to sign in in CI.
- Fixed an issue where a root project with aliases, that has self referential tasks, would trigger a
  stack overflow error.

## 1.14.1

#### 🐞 Fixes

- Fixed an issue when using a global version of npm/pnpm/yarn, and the wrong arguments were being
  passed to commands.
- Fixed the "running for 0s" message constantly logging for interactive tasks.

## 1.14.0

#### 🚀 Updates

- Added a `moon run` shorthand, where "run" can be omitted. For example, `moon run app:build` can be
  written as `moon app:build`.
  - This only works for targets that contain a `:`.
- Updated `moon ci` to support running an explicit list of targets, instead of running everything.
- Updated `node.version`, `npm.version`, `pnpm.version`, `yarn.version`, and `rust.version` to
  support partial versions and requirements/ranges like `1.2`, `1`, or `^1.2`.
- Updated `.moon/tasks` to support nested folders, for better organization of task files.

#### ⚙️ Internal

- Improved handling of certificates and proxies.
- Updated to proto v0.18.

## 1.13.5

#### 🐞 Fixes

- Fixed an issue where the `projectName` query would not work correctly.

## 1.13.4

#### ⚙️ Internal

- Updated to proto v0.17.

## 1.13.3

#### 🐞 Fixes

- Fixed an issue where tool globals directory was not being correctly located.
- Fixed a panic when using the `rust` toolchain and attempting to install `bins`.

## 1.13.2

#### 🐞 Fixes

- Fixed an issue where `pnpm` or `yarn` would panic based on configuration combination.

## 1.13.1

#### 🐞 Fixes

- Fixed an issue where tasks depending on arbitrary project tasks would fail to build a partial
  project graph.
- Fixed an issue where task `deps` within global tasks would not apply exclude/rename overrides.
- Fixed an issue with asset/binary files not working correctly with codegen templates.
- Fixed an issue where `moon upgrade` would require a workspace.

## 1.13.0

#### 🚀 Updates

- Added an `allowFailure` task option, allowing tasks to fail without bailing the entire run.
  - Tasks allowed to fail cannot be depended on.
- Added colors to command line `--help` menus.
- Updated `runner.archivableTargets` to support tag scoped targets.
- Updated `moon query tasks` to filter out projects with no tasks.
- Updated `moon query tasks --affected` to filter based on affected task, instead of affected
  project.
- Updated proto integration from v0.12 to v0.16:
  - proto tools are now powered by WASM plugins, which will be downloaded by moon on-demand.
  - Yarn v2+ will now download the requested version, and not downgrade to latest v1.
  - Please report any issues or degradations from this migration.
  - View entire [proto changelog](https://github.com/moonrepo/proto/blob/master/CHANGELOG.md#0160).

#### 🐞 Fixes

- Fixed `moon init` not using the remote default branch when scaffolding.

#### ⚙️ Internal

- Cleaned up moonbase and launchpad implementations.
- Updated Rust to v1.72.

## 1.12.1

#### 🐞 Fixes

- Improved failed task error messages by including information about the failing task.
- Fixed an issue where failed tasks would be cached.
- Fixed an issue where errors lost information while bubbling up the stack.

## 1.12.0

#### 🚀 Updates

- Added [git worktree](https://git-scm.com/docs/git-worktree) support (experimental).
- Added an `interactive` field to task options. This marks tasks as interactive, so they can use
  stdin.
- Added an `extends` field to task configurations. This allows tasks to extend and inherit settings
  from sibling tasks.
- Updated task `env` values to support token functions and variables.
- Updated task `outputs` to support negated globs.
- Will now log a warning to the console if a configuration file uses the `.yaml` extension.

#### 🐞 Fixes

- Fixed an issue where `moon ci` would no run affected targets based on touched files.

#### ⚙️ Internal

- Improved caching and hashing layers.

## 1.11.1

#### 🐞 Fixes

- Fixed an issue where tasks using output globs would not always hydrate from the cache.

## 1.11.0

#### 💥 Breaking

- To support the new project graph, the order and priority in which environment variables are
  resolved has changed. Previously it was task-level > .env file > project-level. Now it's
  task-level > project-level > .env file.

#### 🚀 Updates

- Rewrote the project graph from the ground-up:
  - Lazily built using a multi-pass approach.
  - Graph edges now indicate the type of relationship: development, production, build, peer.
  - Updated `moon project-graph --json` to include the fully expanded graph data.
- Identifiers (project names, file groups, etc) can now be prefixed with underscores (`_`).
- Added Poetry detection support for Python projects.
- Added an `experiments` setting to `.moon/workspace.yml`.
- **Tasks**
  - Environment variables in `command` and `args` are now substituted.
  - Task `deps` can now depend on tag targets (`#tag:task`).
  - Task `env` are now used when substituting values, alongside system-level.
  - Task `outputs` can now use token variables.
- **Codegen**
  - Templates can be used as-is without rendering with [Tera](https://tera.netlify.app) by appending
    a `.raw` extension.
- **Query language**
  - Updated `project` to query both project name AND alias.
  - Added `projectName` for only querying by name.

#### 🐞 Fixes

- Fixed an issue where newer moonbase secret keys would fail to sign in.
- Fixed an issue where `@files` token would not invalidate the project graph cache.
- Fixed an issue where changing `.env` would not invalidate the project graph cache.

#### ⚙️ Internal

- Updated to proto v0.13.
- Updated Rust to v1.71.

## 1.10.1

#### 🐞 Fixes

- Fixed an issue where `.gitignore` patterns weren't always applied correctly.
- Fixed an issue where `git hash-object` commands would fail if moon was setup in a sub-directory.
- Fixed an issue where our "upgrade moon" message would print when requesting JSON output
  (`--json`), resulting in JSON parsing errors.

## 1.10.0

#### 💥 Breaking

> These changes are fixing edge cases that should not have been allowed, but may break existing
> repos. If these changes become troublesome, we'll revert.

- Tasks that configure the same outputs will now error. This change was made as multiple tasks
  writing to the same output location will cause caching and hydration issues.
- If a dependency of a task failed to run or was skipped, then the parent task will now be skipped.

#### 🚀 Updates

- Added support for `MOON_BASE` and `MOON_HEAD` environment variables.
  - Will be used when diffing across branches or commits.
  - Works for both `moon ci` and `moon run`.
- Added `deno.bins` setting to `.moon/toolchain.yml`.
- Added `hasher.ignorePatterns` and `hasher.ignoreMissingPatterns` settings to
  `.moon/workspace.yml`.
- Updated `moon ci` to include a summary of all failed actions.
- Updated `moon run` to compare against the previous commit when running on the default branch and
  using `--remote`.
- Updated `rust.bins` in `.moon/toolchain.yml` to support an object for each bin entry.
  - Can denote bins as CI or local only.
  - Can force install bins.
- Updated the run report to include stderr/stdout for all attempts.

#### 🐞 Fixes

- Fixed an issue where failed target run attempts would not appear in the run report.

#### 📚 Documentation

- Added a new in-depth "Debugging a task" guide.

#### ⚙️ Internal

- Updated to proto v0.12.
- Modernized the code generator and project constraints implementation.
- Renamed runfile to snapshot throughout.

## 1.9.2

#### 🐞 Fixes

- Fixed a panic when attempting to execute an npm package who's binary is purely Bash.

## 1.9.1

#### 🐞 Fixes

- Fixed a panic when parsing the output of `git --version`.

## 1.9.0

#### 🚀 Updates

- Added VCS hooks management support.
  - Added `vcs.hooks` and `vcs.syncHooks` settings to `.moon/workspace.yml`.
  - Added `moon sync hooks` command.
- Added `--clean` and `--force` flags to `moon sync codeowners` command.
- Updated `moon init` to:
  - Detect an applicable VCS provider and set the `vcs.provider` setting.
  - Convert a detected tool version to a fully-qualified semantic version.
- **Node.js**
  - Moved syncing logic from `InstallNodeDeps` action to `SetupNodeTool` action. This includes
    syncing `packageManager`, `engines`, and version files. This should feel more natural.

#### 🐞 Fixes

- Fixed an issue where task hashes would be different between Windows and Unix machines.
  - Root cause is that arguments would use different path separators.
- Fixed an issue where `dev`, `start`, or `serve` tasks would not always be marked as `local`.
- Fixed an issue where inherited tasks parameters (inputs, deps, etc) would sometimes be lost based
  on the merge strategy.
- Fixed an issue with dependency graph cycle detection.

#### ⚙️ Internal

- Updated to proto v0.11.
- Dropped SVN support since it was never finished and doesn't work.
- Improved VCS file handling, caching, and performance.

## 1.8.3

#### 🐞 Fixes

- Fixed an issue where command line arguments were incorrectly escaped in Bash shells.

## 1.8.2

#### 🐞 Fixes

- Updated `CODEOWNERS` to take `.editorconfig` into account when generating.
- Fixed an issue where `git` branch commands would fail on <= v2.22.
- Fixed an issue where disabling moon's cache would not disable proto's cache.

## 1.8.1

#### 🐞 Fixes

- Fixed an issue where failed processes would not bubble up the original error.
- Fixed TypeScript type issues in `@moonrepo/types`.
- Fixed JSON schema issues.

#### ⚙️ Internal

- Updated to proto v0.10.5.

## 1.8.0

#### 🚀 Updates

- Added code owners (`CODEOWNERS`) support.
  - Added `owners` setting to `moon.yml`.
  - Added `codeowners` setting to `.moon/workspace.yml`.
  - Added `moon sync codeowners` command.
- Added `vcs.provider` setting to `.moon/workspace.yml`.
- Added a new action to the graph, `SyncWorkspace`, that'll be used for workspace-level checks.
- Added `workspace.syncing` and `workspace.synced` webhooks.
- Added `MOON_OUTPUT_STYLE` and `MOON_RETRY_COUNT` environment variables.
- **Rust**
  - Improved Cargo workspace root and members detection.

#### ⚙️ Internal

- Deprecated the `moon sync` command, use `moon sync projects` instead.
- Refactored task inputs, outputs, and file groups to be more accurate.
- Updated Rust to v1.70.

## 1.7.3

#### 🐞 Fixes

- Fixed an issue where glob task outputs were not invalidating a previous build.
- Fixed an issue where changing inputs would not mark a task as affected, when a moon workspace is
  nested within a repository.
- Improved handling of ctrl+c signal detection and shutting down processes.

## 1.7.2

#### 🐞 Fixes

- Node.js
  - Fixed an issue where some workers/packages would fail while inheriting parent args.
- Rust
  - Fixed an issue where `cargo generate-lockfile` would run in the wrong directory.

## 1.7.1

#### 🐞 Fixes

- Fixed some configuration bugs.
- Fixed initial bootstrap log messages not being logged.
- Fixed an issue where hydrated caches would be partially written.

## 1.7.0

#### 🚀 Updates

- Rewrote configuration from the ground-up:
  - Strict parsing to bubble up typos, invalid nesting, and more.
  - Recursive merging and validation.
  - And many more improvements.
- Rewrote error handling and rendering.
  - Improved error messages.
  - Added custom error messages for certain situations.
- Added support for npm lockfile v3 format.

#### 🐞 Fixes

- Fixed an issue where colors were not being forced when passing `--color`.
- Fixed an issue where `--log` or `MOON_LOG` would error when running nested `moon` commands.

#### ⚙️ Internal

- Updated to proto v0.10.
- Updated Cargo dependencies.

## 1.6.1

#### 🐞 Fixes

- Fixed poor argument parsing of command line operators like `;`, `&&`, etc.

## 1.6.0

#### 🚀 Updates

- Added support for persistent tasks.
  - Added `persistent` task option to `moon.yml` (is also set via `local`).
  - Persistent tasks _run last_ in the dependency graph.
- Updated long running processes to log a checkpoint indicating it's still running.
- Updated task `platform` detection to only use the platform if the toolchain language is enabled.
- Started migration to a newer/better logging implementation.

#### 🐞 Fixes

- Fixed an issue where a task would panic for missing outputs.

#### ⚙️ Internal

- Reworked file groups to use workspace relative paths, instead of project relative.
- Reworked processes to better handle command line arguments, shells, and piped stdin input.

## 1.5.1

#### 🐞 Fixes

- Fixed an issue where tasks would run in CI even though `runInCI` was false.
- Fixed an issue where npm, pnpm, and yarn shims were not being used from proto.

## 1.5.0

#### 🚀 Updates

- Added Rust tier 2 and 3 language support!
  - Added `rust` as a supported `platform` variant.
  - Added `rust` setting to `.moon/toolchain.yml`.
  - Added `toolchain.rust` setting to `moon.yml`.
  - Added support for `rust` setting in `.prototools`.
  - Updated `moon init` and `moon bin` commands to support Rust.
  - Updated `moon docker scaffold` command to scaffold Cargo files.
  - Updated `moon docker prune` command to delete the `target` directory.

#### 🐞 Fixes

- Fixed an issue where task type was `run` when it should be `test`.

#### ⚙️ Internal

- Reworked `moon init --yes` to not enable all tools, and instead enable based on file detection.
- Cleaned up `moon init` templates. Will no longer scaffold `.moon/tasks.yml`.

## 1.4.0

#### 🚀 Updates

- Added a new target scope for tags, `#tag:task`, which will run a task for all projects with the
  given tag.
- Updated `moon query projects` and `moon query tasks` to support MQL for filtering results.
- Deprecated `node.aliasPackageNames` setting. Aliases will always be loaded now.

#### ⚙️ Internal

- Upgraded to proto v0.8.
- Updated JSON schemas with missing fields.
- Rewrote ID handling for future features.

## 1.3.2

#### 🐞 Fixes

- Fixed an issue where a `pnpm-lock.yaml` with no packages would fail to parse.

## 1.3.1

#### 🐞 Fixes

- Fixed a few issues during input hashing:
  - Would attempt to include deleted files from `git status`, which would log a warning.
  - Would attempt to hash directories for root-level projects, which would log a warning.

#### ⚙️ Internal

- Upgraded to proto v0.7.2.

## 1.3.0

#### 🚀 Updates

- Introducing MQL, a custom query language for running advanced filters on the project graph.
- Added a `--query` option to the `moon run` command, allowing for advanced targeting.
- Updated config loading to be strict and error on unknown fields for non-root fields.

#### 🐞 Fixes

- Fixed an issue where proto would fail to parse `manifest.json`.

#### ⚙️ Internal

- Updated Rust to v1.69.
- Upgraded to proto v0.7.
- Improved accuracy of our globbing utilities, especially around dotfiles/dotfolders.

## 1.2.2

#### 🚀 Updates

- Added `node_modules/.bin/moon` as another lookup location for the `moon` binary when running
  globally.

#### 🐞 Fixes

- Fixed an issue where running tasks were not killed, resulting in background zombie processes.
- Fixed a few version comparisons between Yarn legacy and berry.
- Updated dependency deduping to not run if the manager version is unknown.

## 1.2.1

#### 🐞 Fixes

- Fixed an issue where `$projectAlias` token was not substituting correctly.

## 1.2.0

#### 🚀 Updates

- Added task inheritance based on tags in the form of `.moon/tasks/tag-<name>.yml`.

#### 🐞 Fixes

- Fixed an issue where setting `MOON_COLOR` would fail validation.

#### ⚙️ Internal

- Upgraded to proto v0.6.
- Improvements to file system operations.
- Minor improvements to performance.

## 1.1.1

#### 🐞 Fixes

- Fixed an issue where token function resolving would cause massive performance degradation.

## 1.1.0

#### 🚀 Updates

- Added token variable substitution support for task `command`s.
- Added a `moon task` command, for viewing resolved information about a task.
- Updated `moon run` to be able to run tasks in the closest project based on current working
  directory.
- Updated `noop` tasks to be cacheable, so that they can be used for cache hit early returns.

#### ⚙️ Internal

- Upgraded to proto v0.5.
- Support pnpm v8's new lockfile format.
- Better handling for task's that execute the `moon` binary.

## 1.0.3

#### 🚀 Updates

- Added `hasher.batchSize` to control the number of files to be hashed per batch.
- Updated new version checks to include an optional message.

#### 🐞 Fixes

- Fixed an issue where non-input matching files were being passed to `git hash-object` during the
  inputs collection process. For large projects, you'll see improved performance.
- Fixed an issue where root-level input globs were not matching correctly when `hasher.walkStrategy`
  was "vcs".
- Fixed a deadlock where some concurrent tasks via a parent `noop` task would not start or run in
  parallel.

#### ⚙️ Internal

- Upgraded to proto v0.4.
- Switched to a semaphore for restricting task concurrency.

## 1.0.2

#### 🐞 Fixes

- Fixed an issue where `moon run` or `moon check` would hang when not running in a workspace.
- Fixed an issue where workspace root finding will locate `~/.moon`.

## 1.0.1

#### 🐞 Fixes

- Updated `envFile` to log a warning instead of triggering an error when `.env.` is missing.
- Updated `envFile` to support workspace relative paths when prefixed with `/`.
- Fixed an issue where `.moon/tasks/*.yml` were not scaffolded into `Dockerfile`s.
- Fixed an issue where a CI environment wasn't detected for some CI providers.
- Fixed a project cache issue when running tasks inside and outside of a container.

## 1.0.0

#### 💥 Breaking

- Updated the installer scripts and the `moon upgrade` command to install the `moon` binary to
  `~/.moon/bin`.
- Removed Homebrew support.

#### 🚀 Updates

- Added a `constraints` setting to `.moon/workspace.yml`, allowing for project/dep relationships to
  be enforced.
- Added a `hasher.warnOnMissingInputs` setting to `.moon/workspace.yml`.
- Added a `shell` task option to `moon.yml` that will wrap system tasks in a shell.
- Added a `tags` setting to `moon.yml` for project categorization.
- Added a `--tags` option to the `moon query projects` command.
- Added a `telemetry` setting to `.moon/workspace.yml`.
- Added 5 new token variables: `$projectAlias`, `$date`, `$time`, `$datetime`, and `$timestamp`.
- Updated task `env` and `.env` files to support variable substitution using `${VAR_NAME}` syntax.
- Updated system tasks to now execute within a shell.

#### 🐞 Fixes

- Reworked how task inputs are resolved when empty `[]` is configured, and all `**/*` is inherited.

#### ⚙️ Internal

- Updated the new version check to only run on the `check`, `ci`, `run`, and `sync` commands.
- Will now detect 16 additional CI environments: Agola, AppCenter, Appcircle, Azure, Bamboo,
  Bitrise, Buddy, Cirrus, Codemagic, Heroku, Jenkins, Jenkins X, Netlify, TeamCity, Vela,
  Woodpecker.
- Will now attempt to detect CD environments for more accurate metrics.
- We now create a [cache directory tag](https://bford.info/cachedir) in `.moon/cache`.
