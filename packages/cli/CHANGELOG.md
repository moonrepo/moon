# Changelog

## Unreleased

#### 🚀 Updates

- Rewrote the actions pipeline from the ground-up:
  - Increased performance.
  - Better concurrency handling and scheduling.
  - More accurately monitors signals (ctrl+c) and shutdowns.
  - Tasks can now be configured with a timeout.

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
