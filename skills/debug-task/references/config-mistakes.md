# Common configuration mistakes

This reference covers the task configuration errors that cause the most confusion. Each section
describes the mistake, why it happens, how to detect it, and how to fix it.

---

## Table of contents

1. [`command` vs `script`](#command-vs-script)
2. [Task inheritance bugs](#task-inheritance-bugs)
3. [Presets and automatic behavior](#presets-and-automatic-behavior)
4. [Persistent tasks blocking the pipeline](#persistent-tasks-blocking-the-pipeline)
5. [`affectedFiles` misconfiguration](#affectedfiles-misconfiguration)
6. [`extends` not resolving](#extends-not-resolving)
7. [No-op tasks](#no-op-tasks)
8. [`runInCI` variants](#runinci-variants)
9. [`allowFailure` hiding errors](#allowfailure-hiding-errors)
10. [`mutex` contention](#mutex-contention)
11. [`timeout` and `retryCount`](#timeout-and-retrycount)
12. [`os` platform filtering](#os-platform-filtering)
13. [`outputStyle` and missing output](#outputstyle-and-missing-output)
14. [Cache lifetime and cache key](#cache-lifetime-and-cache-key)
15. [Task tags and `#tag` targets](#task-tags-and-tag-targets) — v2.3+
16. [Task dep `cacheStrategy`](#task-dep-cachestrategy) — v2.3+
17. [Task dep `type`: `cleanup` and `wait`](#task-dep-type-cleanup-and-wait) — v2.6+
18. [Task checks](#task-checks) — v2.4+
19. [Project-level `taskOptions`](#project-level-taskoptions) — v2.4+
20. [Task builder validation errors](#task-builder-validation-errors)

---

## `command` vs `script`

This is the single most common configuration mistake.

**`command`** accepts a single binary name with optional arguments — also known as a
[simple command](https://www.gnu.org/software/bash/manual/html_node/Simple-Commands.html) in shell
terminology. It supports task inheritance merge strategies.

```yaml
tasks:
  lint:
    command: 'eslint'
    args:
      - '--ext'
      - '.ts,.tsx'
      - 'src/'
```

**`script`** accepts
[pipelines, compound commands](https://www.gnu.org/software/bash/manual/html_node/Shell-Commands.html),
and full shell syntax — pipes, redirects, `&&`, `||`, subshells. It does **not** support inheritance
merging.

```yaml
tasks:
  lint:
    script: 'eslint --ext .ts,.tsx src/ && prettier --check src/'
```

### The mistake

```yaml
# WRONG: shell syntax in command
tasks:
  lint:
    command: 'eslint . && prettier --check .'
```

In v2 this is an error raised while tasks are **built** (before anything runs), so every command
that loads the task graph fails with it — including `moon task`:

- `UnsupportedCommandSyntax` (`task_builder::unsupported_command_syntax`) — the command parsed, but
  uses pipes, redirects, `&&`/`||`, multiple commands, or other shell-specific syntax.
- `InvalidCommandSyntax` (`task_builder::invalid_command_syntax`) — the command couldn't be parsed
  at all (the error includes the line:column position).

### How to detect

The error message names the task. Since `moon task <project>:<task> --json` fails with the same
error, read the `command` value straight from the project's `moon.*` file (or the global
`.moon/tasks/**/*` file it's inherited from). If it contains pipes, redirects, expressions, etc., it
should be `script` instead.

### How to fix

Move the value to `script`. If you need inheritance merging for args, split into separate tasks and
use `deps` to chain them:

```yaml
tasks:
  lint-eslint:
    command: 'eslint'
    args: ['--ext', '.ts,.tsx', 'src/']
  lint-prettier:
    command: 'prettier'
    args: ['--check', 'src/']
  lint:
    # Run both linters
    deps:
      - '~:lint-eslint'
      - '~:lint-prettier'
```

---

## Task inheritance bugs

moon's inheritance system lets you define tasks once in `.moon/tasks/**/*` and have them inherited
by matching projects. When inheritance goes wrong, the task either doesn't appear or appears with
unexpected config.

### Task not inherited

**Check the `inheritedBy` conditions** in the global task file:

```yaml
# .moon/tasks/node-lint.yml
inheritedBy:
  toolchain: 'node'
  stack: 'frontend'
```

Every defined field must match (an implicit AND across fields). If the project has
`toolchain: 'node'` but `stack: 'backend'`, it won't inherit this task. The canonical field names
are plural (`languages`, `layers`, `stacks`, `tags`, `toolchains` — the singular forms above are
aliases). There's also a `files` condition (project-relative file paths, of which at least one must
exist), while `order` only controls the order configs are inherited in, and doesn't filter. A list
of values within `languages`/`layers`/`stacks` matches with OR semantics; explicit `and`/`or`/`not`
clause objects are only supported by `tags` and `toolchains`.

```bash
# See the project's metadata
moon project <project> --json
```

Compare `toolchains`, `config.stack`, `config.layer`, `config.language`, and `config.tags` against
the `inheritedBy` conditions. Use the **configured** `config.language`, not the top-level
`language`: inheritance matches against the language set in `moon.*`, so a language that moon only
_detected_ (the top-level field) doesn't satisfy `inheritedBy.languages`. Tags only appear under
`config.tags`.

**Check for explicit exclusion:**

```yaml
# moon.{json,jsonc,hcl,pkl,toml,yaml,yml} (project level)
workspace:
  inheritedTasks:
    exclude: ['lint'] # This project opted out
```

**Check for rename:**

```yaml
workspace:
  inheritedTasks:
    rename:
      buildPackage: 'build' # Task exists but under a different name
```

### Task inherited with wrong config

When a project overrides an inherited task, moon merges the configs using strategies. The defaults
are:

| Field                     | Default merge strategy  |
| ------------------------- | ----------------------- |
| `args`                    | `append`                |
| `checks` <sup>v2.4+</sup> | `append`                |
| `deps`                    | `append`                |
| `env`                     | `append` (object merge) |
| `inputs`                  | `append`                |
| `outputs`                 | `append`                |
| `tags` <sup>v2.3+</sup>   | `append`                |
| `toolchains`              | `append`                |

The corresponding merge-strategy options are `mergeArgs`, `mergeChecks` <sup>v2.4+</sup>,
`mergeDeps`, `mergeEnv`, `mergeInputs`, `mergeOutputs`, `mergeTags` <sup>v2.3+</sup>, and
`mergeToolchains` — plus an umbrella `merge` option that sets all eight at once (the specific
options override it).

> <sup>v2.6+</sup> Before v2.6, a task's resolved `toolchains` were ordered by an internal hash set,
> so `append` and `prepend` produced the same (arbitrary) order, and `$taskToolchain` could expand
> to a required toolchain (like `npm`) instead of the configured one (like `node`). The order is now
> preserved, and the first toolchain is the primary one.

```yaml
# Global: args = ['--check']
# Project: args = ['--fix']
# Result with append: ['--check', '--fix']
# Result with replace: ['--fix']
# Result with prepend: ['--fix', '--check']
```

If the merged result isn't what you expect, explicitly set the merge strategy:

```yaml
tasks:
  lint:
    args: ['--fix']
    options:
      mergeArgs: 'replace' # Don't append to inherited args
```

### Workspace-inherited env and file groups

<sup>v2.5+</sup> Beyond tasks, `.moon/tasks/**/*` files can define a top-level `env` that is
inherited by every matching project and merged into the project's own `env` — with project-level
variables winning on conflicting keys by default.

```yaml
# .moon/tasks/node.yml — every matching project inherits this
env:
  NODE_ENV: 'production'
```

<sup>v2.5+</sup> The project controls how this merge happens (for both `env` and the inherited
`fileGroups`) via `workspace.mergeStrategies` in its `moon.*` config, using the same strategies as
task merging:

```yaml
# moon.yml
workspace:
  mergeStrategies:
    env: 'replace' # project env fully replaces inherited env
    fileGroups: 'preserve' # first (most global) definition of a group wins
```

**Debugging implications:**

- A task env var with a value that appears nowhere in the project's config may come from a
  workspace-level `env`. Inspect the resolved values with `moon task <target> --json`, then search
  `.moon/tasks/**/*` for the variable.
- `mergeStrategies.env` applies to the **entire map**: `preserve` keeps the first-defined map
  (typically the most global) and ignores the rest, while `replace` keeps only the last. Both can
  make an entire block of variables silently vanish.
- `mergeStrategies.fileGroups` applies **per group name** — a project redefining a group under
  `replace` (including with an empty list) drops the inherited inputs, which changes any task inputs
  that reference the group via tokens (`@files(...)`, `@globs(...)`), and therefore the task's hash.
  Under the default `append` (and `prepend`), an empty list keeps the inherited inputs.

### Diagnosis

```bash
# See which config files contributed to the task
cat .moon/cache/states/<project>/snapshot.json
```

The snapshot's `inherited.layers` maps each task ID to the ordered list of global config files that
contributed to it, and `inherited.configs` maps each config file path to its parsed contents.

---

## Presets and automatic behavior

moon has two built-in presets that set multiple options at once:

### `server` preset

```yaml
tasks:
  dev:
    command: 'vite dev'
    preset: 'server'
```

This sets:

- `cache` -> off
- `outputStyle` -> `stream`
- `persistent` -> on
- `priority` -> `'low'`
- `runInCI` -> off

### `utility` preset

```yaml
tasks:
  setup:
    command: 'setup-script'
    preset: 'utility'
```

This sets:

- `cache` -> off
- `interactive` -> on
- `outputStyle` -> `stream`
- `persistent` -> off
- `runInCI` -> `'skip'`

### Automatic preset assignment

Tasks named `dev`, `start`, or `serve` are **automatically** marked with the `server` preset. This
means they're persistent, non-cacheable, and won't run in CI — even if you didn't explicitly set a
preset.

This is the most surprising automatic behavior in moon. If your task is named `dev` and you're
wondering why it doesn't cache or run in CI, this is why.

### How to detect

```bash
moon task <project>:<task> --json
```

Check the `preset`, `options.persistent`, `options.cache`, and `options.runInCI` fields.

### How to override

You can override individual options even when a preset is applied:

```yaml
tasks:
  dev:
    command: 'vite dev'
    preset: 'server'
    options:
      runInCI: 'always' # Override the preset's runInCI: false
```

> <sup>v2.6+</sup> A persistent task with `runInCI` enabled is still only ran in CI for the tasks
> that depend on it, or when explicitly passed as a target — never on its own (like when `moon ci`
> detects it as affected). See [Persistent tasks](#persistent-tasks-blocking-the-pipeline).

---

## Persistent tasks blocking the pipeline

A persistent task (`options.persistent: true` or `preset: 'server'`) is one that runs continuously —
a dev server, a file watcher, a background process. moon handles persistent tasks specially, as they
never complete:

- **Before v2.6**, they were collected and ran **last** as a single batch, in parallel, once every
  other action in the pipeline had finished.
- **In v2.6+**, they run as soon as they're reached in the action graph (once their own deps have
  completed), alongside other tasks, and they never block the actions that come after them. A
  persistent task also no longer holds up the persistent tasks that depend on it.

### The problem

If a non-persistent task lists a persistent task in `deps` as a `required` dependency (the default),
moon produces a **hard error** when tasks are built, before execution starts, as the dependent would
never run. Which error you see depends on `runInCI`, as that's checked first:

- `RunInCiDepRequirement` (`task_builder::dependency::run_in_ci_mismatch`) — the usual one in v2.6+.
  Persistent tasks (and `server` preset tasks) default to `runInCI: false`, while a build/test task
  defaults to `true`, so the CI mismatch is caught before the persistence problem.
- `PersistentDepRequirement` (`task_builder::dependency::persistent_requirement`) — once `runInCI`
  is aligned (or either side uses `'skip'`). Its message suggests `type: 'wait'` (to wait for the
  dep to start), or disabling `options.cache` (if you only wanted to avoid the cache).

```yaml
# ERROR: integration-test depends on dev-server, which is persistent
tasks:
  dev-server:
    command: 'vite dev'
    preset: 'server'
  integration-test:
    command: 'cypress run'
    deps:
      - '~:dev-server' # error
```

### How to detect

```bash
# Visualize the dependency graph
moon action-graph <project>:<task> --dot

# Look for a persistent task node with edges pointing to it from other tasks
# Dependency types are labeled in the human-readable output (v2.6+)
moon task <project>:<task>
```

### How to fix

**Option 1 (v2.6+): Wait for it to start, then stop it.** A `wait` dependency only waits for the
server to _start_, and a `cleanup` dependency stops it once the tests have ran (pass or fail). See
[Task dep `type`](#task-dep-type-cleanup-and-wait).

```yaml
tasks:
  dev-server:
    command: 'vite dev'
    preset: 'server'
    options:
      runInCI: true # only ran in CI for the tasks that wait on it, never on its own
  integration-test:
    command: 'cypress run'
    deps:
      - target: '~:dev-server'
        type: 'wait'
      - target: '~:dev-server-stop'
        type: 'cleanup'
    options:
      cache: false # otherwise a cache hit starts the server, only to stop it right away
```

The CI check also applies to `wait` deps (only `cleanup` deps are exempt), so without `runInCI` on
the server, this still fails with `RunInCiDepRequirement`. Alternatively, set the server's `runInCI`
to `'skip'` (the tests run in CI without it), or disable `runInCI` on the tests.

**Option 2: Remove the dependency.** Run the server and tests separately:

```bash
# In one terminal
moon run app:dev-server

# In another terminal
moon run app:integration-test
```

**Option 3 (before v2.6): Use a script that manages both.** Create a script that starts the server,
waits for it to be ready, runs tests, then kills the server:

```yaml
tasks:
  integration-test:
    script: 'start-server-and-test "vite dev" http://localhost:3000 "cypress run"'
```

### Persistent tasks in CI and `moon check`

A task that never completes keeps a CI pipeline running until it times out, so in v2.6+:

- Persistent tasks default to `runInCI: false`, even when they define `outputs` or have a `type` of
  `build` or `test`. Before v2.6, such a task could run in CI by default and hang the pipeline.
- When `runInCI` is explicitly enabled, a persistent task is only ran in CI for the tasks that
  depend on it (like a `wait` dependency), or when explicitly passed as a target — never on its own,
  like when `moon ci` detects it as affected.
- `moon check` only runs persistent tasks when another task that it runs depends on them. If the
  checked projects have no build or test tasks, it reports that there's nothing to check and exits
  with a non-zero code (instead of prompting for tasks to run).

### Persistent deps and `runDepsInParallel: false`

<sup>v2.6+</sup> When `runDepsInParallel` is disabled, a persistent dep never completes, so it's
skipped when ordering the deps that follow it — those are ordered against the previous dep that does
complete. Only `required` deps are ordered at all; `wait` and `cleanup` deps don't complete before
the task runs.

Separately, an ordering edge that would form a cycle — because the configured order contradicts a
dependency between the deps themselves — is skipped, so the dependency wins.

---

## `affectedFiles` misconfiguration

The `affectedFiles` option passes affected file paths to the task's command as arguments (and/or as
the `MOON_AFFECTED_FILES` env var). Real file lists are only passed when affected tracking is on —
`--affected` on `moon run`/`moon check`/`moon exec`, and always under `moon ci`. Otherwise the list
is empty, and the "no results" fallbacks below apply.

### The mistake

```yaml
tasks:
  lint:
    command: 'eslint'
    args: ['.'] # Already passing '.' as an argument
    options:
      affectedFiles: true # Also tries to pass file paths as args
```

Now `eslint` receives both `.` **and** the affected file paths, which may cause it to lint
everything (`.`) regardless.

### Object form

The `affectedFiles` setting supports an object form with additional options:

```yaml
tasks:
  lint:
    command: 'eslint'
    options:
      affectedFiles:
        pass: 'args' # 'args', 'env', or true (both)
        filter:
          - '**/*.ts'
          - '**/*.tsx'
```

### `passInputsWhenNoMatch` and `passDotWhenNoResults`

Controls what happens when there are no affected files. These options are nested inside the
`affectedFiles` object:

```yaml
tasks:
  lint:
    command: 'eslint'
    options:
      affectedFiles:
        pass: 'args'
        passInputsWhenNoMatch: true # Pass task inputs instead of '.'
        passDotWhenNoResults: true # Pass '.' when no results at all
        ignoreProjectBoundary: false # Ignore project boundary for file matching
```

By default, when no files are affected, `.` (current directory) is passed as an argument (only if
`.` isn't already one), and `MOON_AFFECTED_FILES` is set to `.`. Set `passInputsWhenNoMatch: true`
to pass the task's **resolved input files** instead (globs expanded, limited to the project unless
`ignoreProjectBoundary` is set, then narrowed by `filter`) — with the default `**/*` inputs, that's
every file in the project.

> **Note:** The v1 option `affectedPassInputs` was removed in v2. Use
> `affectedFiles.passInputsWhenNoMatch` instead.

### Key point

Without affected tracking (`moon run <target>` with no `--affected`), `affectedFiles` doesn't pass
real file lists, but it still has an effect: the command receives `.` (or every resolved input file
with `passInputsWhenNoMatch`), and `MOON_AFFECTED_FILES=.` is set. If a task unexpectedly receives
`.` or a huge file list locally, this is why.

---

## `extends` not resolving

Tasks can extend other tasks using the `extends` field:

```yaml
tasks:
  build:
    command: 'vite build'
    inputs:
      - 'src/**/*'
  build-prod:
    extends: 'build'
    env:
      NODE_ENV: 'production'
```

### Common issues

**Base task doesn't exist:** The task being extended must exist in the same project (either defined
locally or inherited). If it's not found, it will error.

**Circular extension:** Task A extends B, B extends A. moon does **not** detect this — the chain is
resolved recursively with no cycle guard, so a circular (or self-referencing) `extends` hangs or
crashes graph building rather than producing a diagnostic. If moon dies while building tasks,
inspect `extends` chains by hand.

### How to verify

```bash
moon task <project>:<extended-task> --json
```

The resolved config should show the merged result of the base task plus the overrides from the
extending task.

---

## No-op tasks

moon treats tasks with command `noop`, `nop`, or `no-op` as intentional no-ops. These tasks execute
successfully but do nothing. They're sometimes used as aggregation points — a task that only exists
to declare `deps` on other tasks.

```yaml
tasks:
  all-checks:
    command: 'noop'
    deps:
      - '~:lint'
      - '~:test'
      - '~:typecheck'
```

If a user reports "my task runs but produces no output," check if the command is one of the no-op
values.

```bash
moon task <project>:<task> --json
# Look at the "command" field
```

---

## `runInCI` variants

The `runInCI` option controls whether a task runs in CI environments. It accepts more values than
most people realize:

| Value                                                | Local       | CI (affected) | CI (not affected) |
| ---------------------------------------------------- | ----------- | ------------- | ----------------- |
| `true` / `'affected'` (default for build/test tasks) | Runs        | Runs          | Skipped           |
| `false`                                              | Runs        | Skipped       | Skipped           |
| `'always'`                                           | Runs        | Runs          | Runs              |
| `'only'`                                             | **Skipped** | Runs          | Skipped           |
| `'skip'`                                             | Runs        | **Skipped**   | Skipped           |

### Common surprises

**`'only'`** — the task is CI-only. Running `moon run app:deploy` locally doesn't run it — with no
other targets, moon prints "No tasks found." and exits with code 1. This trips people up when they
try to test a CI task locally.

**`'skip'`** — the task is skipped in CI but task relationships (deps) remain valid. Unlike `false`,
a CI-enabled task can depend on a `'skip'` task — depending on a `false` task is rejected when tasks
are built (`RunInCiDepRequirement`), on every command and locally too, not just in CI.

**`'always'`** — the task always runs in CI regardless of affected status. Useful for tasks like
`deploy` that should run on every merge to main, even if no inputs changed.

**Persistent tasks** <sup>v2.6+</sup> — default to `false` regardless of their type, and when
enabled, only run for the tasks that depend on them, or when explicitly targeted. See
[Persistent tasks in CI](#persistent-tasks-in-ci-and-moon-check).

**A CI task depending on a non-CI task** — raises `RunInCiDepRequirement`
(`task_builder::dependency::run_in_ci_mismatch`). In v2.6+, the error lists the three fixes: enable
`runInCI` for the dependency (both run in CI), set it to `'skip'` for the dependency (the task runs
in CI without it), or disable it for the task (neither runs in CI). A persistent dependency hits
this by default, as its `runInCI` defaults to `false`.

### How to detect

```bash
moon task <project>:<task> --json | grep -i runinci
# Also check state.setRunInCi — true means runInCI was set explicitly, by a
# preset, or forced off by `interactive: true`; the key is omitted entirely when
# it defaulted from the task type (build/test → run in CI), and in v2.6+ also
# from persistence (persistent → off)
```

---

## `allowFailure` hiding errors

When `options.allowFailure` is `true`, a failing command no longer fails the pipeline: the failure
is still recorded and displayed for the task itself, but downstream work continues and moon exits
successfully, so the failure is easy to overlook.

```yaml
tasks:
  advisory-lint:
    command: 'eslint src/'
    options:
      allowFailure: true # Lint failures are warnings, not blockers
```

This is intentional for advisory tasks. But if it's inherited from a global task and the user
doesn't realize it's set, real errors go unnoticed.

**Gotcha with deps:** A task **cannot** depend on a task with `allowFailure: true` — the task
builder rejects the configuration with a hard `AllowFailureDepRequirement` error, because a failing
dependency would let the dependent task run with incorrect results. If a task suddenly errors at
graph-build time after someone added `allowFailure` to an upstream task, this is why.

### How to detect

```bash
moon task <project>:<task> --json
# Check options.allowFailure
```

---

## `mutex` contention

The `mutex` option ensures only one task with that mutex name runs at a time, even across different
projects. This prevents concurrent access to shared resources (like a database or a shared port).

```yaml
tasks:
  integration-test:
    command: 'vitest --run'
    options:
      mutex: 'database' # Only one test suite hits the DB at a time
```

### Problems

**Unexpected serialization:** If multiple tasks share a mutex, they run one at a time instead of in
parallel. This can make the pipeline much slower than expected.

**Combined with deps:** If task A (mutex: "x") depends on task B (mutex: "x"), and both need to run,
B acquires the mutex, completes, then A acquires it. This is fine. (Dependency cycles never get this
far — the action graph rejects them with a "dependency cycle has been detected" error.)

**Combined with long-running tasks:** the lock is held for the task's entire command, so a
persistent task — or <sup>v2.6+</sup> a `wait` dep — that shares a mutex with another task never
releases it. A task that waits on a server with the same mutex blocks forever (and the `cleanup`
that would stop the server never runs). Don't put a mutex on servers that other tasks run alongside.

### How to detect

```bash
moon task <project>:<task> --json
# Check options.mutex — see if multiple tasks share the same value
```

---

## `timeout` and `retryCount`

### Timeout

The `timeout` option (in seconds) kills the task if it exceeds the time limit.

```yaml
tasks:
  e2e:
    command: 'playwright test'
    options:
      timeout: 300 # 5 minutes
```

If a task is timing out, check whether the timeout is too aggressive for the workload. On CI with
slower machines, you may need a longer timeout. The same timeout also applies to each `checks`
script individually — with surprising outcomes: a timed-out `requirement` counts as **passing**, a
timed-out `condition` counts as not-passed (the task runs), and a timed-out `fingerprint`
contributes nothing to the hash. None of them error.

### Retry count

The `retryCount` option re-runs a failed task up to N times. This is useful for flaky tests but can
mask real failures.

```yaml
tasks:
  flaky-test:
    command: 'vitest --run'
    options:
      retryCount: 2 # Retry up to 2 times on failure
```

If a task "sometimes passes," check if `retryCount` is set — the task might be flaky but passing on
retries. Note `retryCount: 2` means up to 3 total attempts.

<sup>v2.6+</sup> A task is no longer retried once the pipeline has been aborted (because another
task failed) or interrupted (like with Ctrl+C), and a task that hadn't started its command yet (it
was still hashing, or waiting on a `mutex`) doesn't start it. Before v2.6, these could spawn
processes that were never terminated and kept running after moon exited — if a user reports orphan
processes on an older version, this is a likely cause.

---

## `os` platform filtering

The `os` option restricts a task to specific operating systems. On a non-matching platform the task
isn't removed — its command is rewritten to `noop` at build time (args, script, and outputs
cleared), so it runs as a passing no-op. `moon task <target> --json` showing `command: noop` when
you configured something else is the giveaway.

```yaml
tasks:
  build-macos:
    command: 'xcodebuild'
    options:
      os: 'macos' # Only runs on macOS
```

Supported values: `linux`, `macos`, `windows`.

If a task "doesn't run" on one platform but works on another, check the `os` option. This is
especially common in cross-platform CI pipelines.

---

## `outputStyle` and missing output

The `outputStyle` option controls how task output is displayed in the terminal:

| Value                   | Behavior                                        |
| ----------------------- | ----------------------------------------------- |
| `'buffer'`              | Capture output and display after task completes |
| `'buffer-only-failure'` | Only show output if the task fails              |
| `'hash'`                | Display the generated hash                      |
| `'none'`                | Suppress all output                             |
| `'stream'`              | Stream output in real-time                      |

If the user reports "my task runs but I see no output," check `outputStyle`. A value of `'none'` or
`'buffer-only-failure'` (with a passing task) suppresses output entirely.

The `server` and `utility` presets both set `outputStyle: 'stream'`.

### Primary vs transitive targets

<sup>v2.4.3+</sup> `outputStyle` only applies to **transitive** targets (deps of what was
requested). **Primary** targets — those explicitly passed on the command line — always display their
output, regardless of the configured style. (Before v2.4.3, the style applied to primary targets
too.) So the same task can show output in `moon run app:build` but not when it runs as a dep of
another target.

- <sup>v2.6+</sup> Enabling `experiments.explicitTaskOutputStyle` (or
  `MOON_EXPERIMENT_EXPLICIT_TASK_OUTPUT_STYLE`) applies the configured style to primary targets as
  well — check for it when a primary target's output is unexpectedly hidden.
- <sup>v2.6+</sup> The `--output-style <style>` option on `moon run`, `moon ci`, `moon check`, and
  `moon exec` overrides the style for **all** tasks in that run, including primary targets. For
  example, `moon ci --output-style buffer-only-failure` keeps passing tasks quiet. Check the CI
  script for this flag before blaming the task config.
- Interactive tasks always stream, as they must stay attached to the terminal.
- In v2.5.2 through the last v2.5 release, a primary target's `outputStyle` was incorrectly applied
  in CI, or when the task was hydrated from the cache (fixed in v2.6) — so on those versions,
  primary output going missing in CI is a known issue rather than a config mistake.

---

## Cache lifetime and cache key

### `cacheLifetime`

Controls how long cached outputs are considered valid. After this duration, the cached entry becomes
stale and will **no longer be hydrated** — even if the hash matches, the task will re-execute.

```yaml
tasks:
  build:
    command: 'vite build'
    options:
      cacheLifetime: '7 days'
```

At runtime, moon checks staleness in two places:

- **Last run time:** if the task's last run is older than the lifetime, the "outputs already exist"
  shortcut is skipped and the task re-executes. The last run time is updated on **every** run,
  including cache hits — so a task that's ran more often than its lifetime (with its outputs still
  on disk) never goes stale this way.
- **Archive file:** if the `.tar.gz` archive in `.moon/cache/outputs/` is older than the lifetime,
  it isn't hydrated from — but moon then falls through to the other storage backends (the local CAS
  when `casOutputsCache` is enabled, and the remote cache), which aren't checked against the
  lifetime. So with those enabled, a stale archive can still result in a cache hit.

`moon clean` does **not** read `cacheLifetime` — it has its own `--lifetime <duration>` option
(default `7 days`).

### `cacheKey`

An additional arbitrary string added to the hash computation. Changing this value invalidates all
existing caches for the task, even if nothing else changed.

```yaml
tasks:
  build:
    command: 'vite build'
    options:
      cacheKey: 'v2' # Bump this to force cache invalidation
```

Useful for: breaking the cache after a toolchain upgrade, config change outside moon's tracking, or
any "just bust the cache" scenario.

---

## Task tags and `#tag` targets

Available in v2.3+.

Tasks can declare `tags` for categorization. Targets can then reference tasks by tag using `#`:

```yaml
tasks:
  lint:
    command: 'eslint'
    tags: ['quality', 'ci']
```

```bash
# Run every task with the `quality` tag, in every project
moon run ':#quality'

# Run quality-tagged tasks in a specific project
moon run 'app:#quality'

# Run quality-tagged tasks in projects tagged `frontend`
moon run '#frontend:#quality'
```

> The `#` is a shell comment marker, so `#tag` targets must be quoted (or escaped with `\#`) on the
> command line. A bare `#tag` with no colon is rejected — the task scope must always be present.

Dependency-relative scopes are meant for a task's `deps` list. On the command line, `^:#tag` fails
with "Dependencies scope (^:) is not supported in run contexts." A `~:` target resolves to the
project in the current working directory, but `~:#tag` fails with "requires fully-qualified task
identifer (project:task)", as tag scopes can't be resolved that way.

### Common mistakes

**The `#tag` target matches nothing**

```bash
moon task <project>:<task> --json
# Inspect the `tags` field
```

If `tags` is missing or doesn't contain the tag you used in the target, the task won't match.

**Tags lost during inheritance**

By default `tags` merges with `append`, so global tasks contribute their tags and projects can add
more. If `options.mergeTags: 'replace'` is set, the project's tags replace the global ones — which
can silently drop tags you expected to inherit. Check `options.mergeTags` in
`moon task <target> --json`.

**Tag vs project tag confusion** <sup>MQL</sup>

MQL has two tag fields: `projectTag` (with `tag` as a legacy alias) matches **project** tags, while
`taskTag` <sup>v2.3+</sup> matches **task** tags. On **task** queries, only `taskTag` works —
`projectTag`/`tag` (like every other project field) silently match **nothing**, so
`moon query tasks "tag=quality"` returns no tasks at all. To filter tasks by their project's tags,
query the projects instead, or use `--project <regex>`.

The MQL query is a **positional** argument — there's no `--query` flag on `moon query`.

```bash
moon query tasks --tags quality                 # task tags (regex flag)
moon query tasks "taskTag=quality"              # task tags (MQL)
moon query projects "projectTag=frontend"       # projects by project tag
moon query projects "taskTag=quality"           # projects containing a task tagged quality
```

> In v2.3–v2.4, **no** tag field worked on task queries — the task matcher silently dropped them
> all, matching nothing (this also broke task tag glob targets like `:#tag-*`). v2.5 fixed `taskTag`
> only. On older versions, use the `--tags` flag.

---

## Task dep `cacheStrategy`

Available in v2.3+.

Each entry in `deps` can declare a `cacheStrategy` that controls whether the dep contributes to the
current task's hash. The full breakdown is in `cache-issues.md` —
[Dependency cache strategies](./cache-issues.md#dependency-cache-strategies). The summary:

| Strategy    | This task's cache invalidates when…                   |
| ----------- | ----------------------------------------------------- |
| `'hash'`    | …the dep's hash changes (inputs, command, args, env). |
| `'ignored'` | …never. Dep is a sequencing edge only.                |
| `'outputs'` | …the dep's output files change.                       |

### The default changed in v2.3

When `cacheStrategy` is omitted, the default is now chosen based on whether the dep declares
outputs:

- Dep **with** outputs → `'hash'` (same as before).
- Dep **without** outputs → `'ignored'` (was `'hash'` before).

If you upgraded from v2.2 and downstream tasks stop invalidating when an upstream `lint` / `test` /
`typecheck` (no outputs) changes, this is why. Set `cacheStrategy: 'hash'` explicitly to restore the
old behavior:

```yaml
tasks:
  build:
    deps:
      - target: '~:lint'
        cacheStrategy: 'hash'
```

### How to inspect

```bash
moon task <project>:<task> --json
# Each `deps` entry shows its resolved cacheStrategy
```

---

## Task dep `type`: `cleanup` and `wait`

Available in v2.6+.

Each entry in `deps` has a `type` that controls **when** the dep runs in relation to the task:

| Type                 | Runs                               | Task waits for it to…                  |
| -------------------- | ---------------------------------- | -------------------------------------- |
| `required` (default) | Before the task                    | Complete successfully                  |
| `cleanup`            | After the task has ran its command | (runs after, even when the task fails) |
| `wait`               | Before the task, then alongside it | _Start_ running (not complete)         |

```yaml
tasks:
  e2e:
    command: 'playwright test'
    deps:
      - 'db:start' # required
      - target: 'web:serve'
        type: 'wait'
      - target: 'web:stop'
        type: 'cleanup'
      - target: 'db:stop'
        type: 'cleanup'
    options:
      cache: false
```

Both `cleanup` and `wait` deps never contribute to the task's hash, and always run their command
instead of being hydrated from the cache, as the task relies on them running.

### Cleanup dep didn't run

Cleanups are skipped when there's **nothing to clean up** — the task didn't run its command because
it was skipped, or hydrated from the cache. The exceptions: the task already started a `wait` dep
(which the cleanup typically stops), the cleanup was explicitly passed as a target, or another task
depends on it. Look for this in the logs:

```bash
moon run <project>:<task> --log debug 2>&1 | grep -i "clean"
# "Skipping cleanup job, as there's nothing to clean up"                  → expected skip
# "Pipeline was aborted, running cleanup jobs for the jobs that have ran" → abort path
```

Other reasons a cleanup won't run:

- The pipeline was **interrupted by a signal** (Ctrl+C). Cleanups only run when a _failure_ aborts
  the pipeline, not a signal. A cleanup that was already running when another task failed is
  terminated along with every other task.
- The cleanup has `runInCI` disabled, and the pipeline is running in CI — even after the task it
  cleans up after. (Conversely, `moon ci` and `moon check` select tasks on their own, so a cleanup
  they select runs like any other task. Mark cleanup-only tasks as `internal`.)
- The cleanup's **own** deps hadn't completed when a failure aborted the pipeline, so it couldn't
  run. Cleanups should be self-contained.

Shared cleanups (same `args`/`env`) run once, after all of the tasks that depend on them. They also
still run when the task's deps don't, like with `--upstream none`.

### Wait dep problems

- **The task can't connect to the dep** — `wait` only waits for the dep to have _started_, not to be
  _ready_ (accepting connections). The task should poll a health check before sending requests.
- **The task was skipped** — a `wait` dep that already failed, or was skipped, by the time the task
  runs skips the task. If it fails _after_ the task started, the task keeps running, unless the
  failure aborts the pipeline (like with `moon run`).
- **The pipeline never finishes** — `wait` deps are not stopped when the task completes. A
  non-persistent one keeps the pipeline running until it exits; a persistent one until moon is
  exited. Pair it with a `cleanup` dep that stops it, and make sure the dep **exits successfully**
  when stopped (handle `SIGTERM`), otherwise it counts as a failed task and fails the pipeline.
- **A server starts and is immediately stopped** — `wait` deps start before the task's cache is
  checked, so on a cache hit the dep starts and its cleanup stops it right away. Disable `cache` on
  tasks that wait on long-running deps.
- `wait` deps don't count towards the pipeline's concurrency limit, so they never prevent the task
  from running.

### Inheritance

`cleanup` deps inherited through `implicitDeps` are not inherited by persistent tasks, and deps that
reference the inheriting task itself (like `~:teardown` for the `teardown` task) are dropped. A task
can't depend on the same task with two different types — if both come from inheritance, use
`mergeDeps`, or the `workspace.inheritedTasks` exclude/rename filters, to resolve it.

### How to inspect

```bash
# Human-readable: non-required deps are labeled, like "web:serve (wait)"
moon task <project>:<task>

# Machine-readable: dep entries have a `type` (omitted when `required`)
moon task <project>:<task> --json

# Cleanups are linked as dependents of the task in the graph (the edge is reversed)
moon action-graph <project>:<task> --dot
```

---

## Task checks

Available in v2.4+.

A task's `checks` field is a list of shell scripts that run **before** the task. Each check has a
type that determines what happens based on the script's exit code. A check defined as a plain string
is a `requirement` by default.

```yaml
tasks:
  deploy:
    command: './deploy.sh'
    checks:
      # requirement (string shorthand): must pass or the task fails
      - 'command -v aws'
      # condition: if all conditions pass, the task is SKIPPED
      - check: 'condition'
        script: './scripts/already-deployed.sh'
      # fingerprint: script output is folded into the task hash
      - check: 'fingerprint'
        script: 'aws --version'
        hash: 'stdout' # true (all output) | false (run, hash nothing) | 'exit-code' | 'stdout' | 'stderr'
```

### Behavior by type

| Type          | Script passes (exit 0)             | Script fails (non-zero)                                  |
| ------------- | ---------------------------------- | -------------------------------------------------------- |
| `requirement` | Task continues                     | Task **fails** — `RequirementCheckFailed`, does not run  |
| `condition`   | Counts toward skipping (see below) | Task runs as normal                                      |
| `fingerprint` | Output mixed into hash             | Task **fails** — `FingerprintCheckFailed` during hashing |

A script that fails to _spawn_ is fatal for **every** check type (conditions included), and the
process error propagates as-is rather than as the variants above. A script that hits the task's
`options.timeout` never errors: a timed-out `requirement` counts as **passing**, a timed-out
`condition` as not-passed, and a timed-out `fingerprint` contributes nothing to the hash.

**Conditions skip, they don't gate.** The task is skipped **only when _all_ `condition` checks
pass**. If any condition fails, the task runs as normal. This is the inverse of a requirement, and a
common source of "my task never runs" confusion.

**When checks actually run:**

- `fingerprint` checks run during **hash generation**, which happens on _every_ run — even when the
  result is a cache hit, and even when the task's cache is disabled.
- `requirement` and `condition` checks run just before **task execution** — so they do **not** run
  on a cache hit. A missing tool won't trip a `requirement` check while the task hydrates from
  cache; it only surfaces on the next cache miss.
- Checks of the same phase execute in **parallel**, not in declaration order — don't rely on one
  check's side effects in another.
- The task's `options.timeout` also applies to each check script individually.

### Common surprises

**"My task fails with a requirement/fingerprint check error"**

```
Task app:deploy is unable to run as the requirement check `command -v aws` failed.
```

The named script exited non-zero. The diagnostic codes are `task_runner::requirement_check_failed`
and `task_runner::hash_check_failed`. Run the script manually to see why it fails.

**"My task is skipped even though inputs changed and it's not a cache hit"**

All `condition` checks passed, so moon skipped the task on purpose. The target ends in a
`SkippedConditional` state (plain `Skipped` is a different state, used when a dependency failed).
Confirm with debug logs:

```bash
moon run <project>:<task> --log debug --force 2>&1 | grep -i "condition\|check"
# "Skipping task as all conditional checks have passed"  → condition skip
# "Will continue to run the task as not all conditional checks have passed"  → ran normally
```

**"My task re-runs every time after adding a check"**

A `fingerprint` check hashes its script output. If that output is volatile (a timestamp, PID, or
changing version), the hash changes on every run. Narrow the hashed portion with the `hash` field
(e.g. `hash: 'exit-code'`), or remove the fingerprint. See
[cache-issues.md](./cache-issues.md#fingerprint-checks-in-the-hash).

**Checks disappeared or duplicated after inheritance**

Checks merge with `append` by default. Set `options.mergeChecks` (`replace`, `prepend`, `preserve`)
to control how inherited checks combine with project-level ones.

### How to inspect

```bash
moon task <project>:<task> --json
# Inspect the `checks` array — each entry shows its type and script
```

---

## Project-level `taskOptions`

Available in v2.4+.

A project's `moon.*` config can now define a top-level `taskOptions` block that applies default
[task options](https://moonrepo.dev/docs/config/project#options) to **every task in that project**,
which each task can still override.

```yaml
# moon.yml
taskOptions:
  cache: false
  retryCount: 2

tasks:
  build:
    command: 'vite build'
    # Inherits cache: false and retryCount: 2
  lint:
    command: 'eslint .'
    options:
      cache: true # Overrides the project default
```

### Why it matters for debugging

This is a **new inheritance layer**. When a task option isn't what you expect, and it isn't set on
the task itself or in a global `.moon/tasks/*` file, check the project's `taskOptions`. The
inheritance order is:

0. The task's `preset`, if any (the base that everything below overrides).
1. Global `.moon/tasks/*` `taskOptions` (workspace-wide defaults, since v1.20).
2. Project `moon.*` `taskOptions` <sup>v2.4+</sup> (project-wide defaults).
3. Per-task `options` — first those of the inherited global task definition (`tasks:` in
   `.moon/tasks/*`), then the project's own task (most specific, wins).

Note the consequence of layer 3: an `options` value on an **inherited global task** beats the
project's `taskOptions`, so a project-level default can appear to be ignored for inherited tasks.

```bash
# See the fully resolved options after all layers merge
moon task <project>:<task> --json

# See which config files/layers contributed
cat .moon/cache/states/<project>/snapshot.json
```

If a task unexpectedly stopped caching, retries, or picked up a `mutex`/`timeout`, a project-level
`taskOptions` is a likely culprit that's easy to overlook because it lives outside the `tasks:` map.

---

## Task builder validation errors

moon's task builder validates configuration at build time and produces specific errors. If you see
one of these, here's what it means:

**`PersistentDepRequirement`** — a non-persistent task has a `required` dep on a persistent task.
This is always a configuration error because the persistent task never finishes. Fix: in v2.6+,
change the dep to `type: 'wait'`; otherwise remove the dependency or restructure the task graph. If
the task runs in CI, the CI check fires first (`RunInCiDepRequirement`), and still applies to `wait`
deps — also enable `runInCI` on the persistent dep (or set it to `'skip'`), or disable `runInCI` on
the task.

**`AllowFailureDepRequirement`** — a task depends on a task with `allowFailure: true`. This is a
hard error: moon rejects the configuration, because a failing dependency would still let the
dependent task run, producing incorrect results. <sup>v2.6+</sup> `cleanup` deps are exempt, as they
run after the task.

**`RunInCiDepRequirement`** — a task that runs in CI depends on a task that doesn't run in CI
(`runInCI: false`). The dependency won't execute in CI, so the dependent task may fail or produce
incorrect results. In v2.6+, the error lists how to resolve it (see
[`runInCI` variants](#runinci-variants)), and persistent deps hit it by default.

**Dependency type errors** <sup>v2.6+</sup> — raised for invalid `cleanup`/`wait` relationships
(diagnostic codes under `task_builder::dependency::`):

- `persistent_cleanup_dep` — a persistent task used as a `cleanup` dep (it would never complete).
- `persistent_cleanup_task` — a persistent task with a `cleanup` dep (it never completes, so the
  cleanup would run at the wrong time).
- `interactive_wait_dep` — a `wait` dep on an interactive task (interactive tasks run in isolation,
  so nothing can run alongside them).
- `self_reference` — a task with a `cleanup` or `wait` dep on itself.
- `conflicting_types` — the same dep listed with two different types, often via inheritance (use
  `mergeDeps` or the `workspace.inheritedTasks` filters).

Config validation also rejects a `cleanup`/`wait` dep with `cacheStrategy: 'hash'` or `'outputs'`
("only supported for required dependencies; use ignored instead"), and `type: 'optional'` (use the
`optional` field instead).

**`InvalidCommandSyntax` / `UnsupportedCommandSyntax`** — the `command` field contains shell syntax
(pipes, redirects, `&&`) that should use `script` instead.

**`UnknownExtendsSource`** — the `extends` field references a task that doesn't exist in the current
project or global scope.

**`UnknownDepTarget`** — a `deps` entry references a target that doesn't exist. Check for typos in
the project or task name.

**"a shell script is required for a task check"** <sup>v2.4+</sup> — a `checks` entry has an empty
or whitespace-only `script`. Every check must define a non-empty shell script.
