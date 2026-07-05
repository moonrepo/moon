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
17. [Task checks](#task-checks) — v2.4+
18. [Project-level `taskOptions`](#project-level-taskoptions) — v2.4+
19. [Task builder validation errors](#task-builder-validation-errors)

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

In v2 this is a **parse error** — moon rejects the configuration at runtime with an error.

### How to detect

```bash
moon task <project>:<task> --json
```

If the `command` field contains pipes, redirects, expressions, etc., it should be `script` instead.

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

Both conditions must be met. If the project has `toolchain: 'node'` but `stack: 'backend'`, it won't
inherit this task.

```bash
# See the project's metadata
moon project <project> --json
```

Compare `toolchains`, `stack`, `layer`, `language`, and `tags` against the `inheritedBy` conditions.

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
`mergeToolchains`.

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

### Diagnosis

```bash
# See which config files contributed to the task
cat .moon/cache/states/<project>/snapshot.json
```

The snapshot's `inherited.layers` shows which global config files were loaded and in what order.

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

---

## Persistent tasks blocking the pipeline

A persistent task (`options.persistent: true` or `preset: 'server'`) is one that runs continuously —
a dev server, a file watcher, a background process. moon handles persistent tasks specially: they
run **last** and **in parallel**, after all non-persistent dependencies complete.

### The problem

If a non-persistent task lists a persistent task in `deps`, moon produces a **hard error**. moon
validates dep chains and rejects this configuration before execution starts.

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
moon action-graph <project>:<task>

# Look for a persistent task node with edges pointing to it from other tasks
```

### How to fix

**Option 1: Remove the dependency.** Run the server and tests separately:

```bash
# In one terminal
moon run app:dev-server

# In another terminal
moon run app:integration-test
```

**Option 2: Use a script that manages both.** Create a script that starts the server, waits for it
to be ready, runs tests, then kills the server:

```yaml
tasks:
  integration-test:
    script: 'start-server-and-test "vite dev" http://localhost:3000 "cypress run"'
```

**Option 3: Restructure so persistent tasks are leaf nodes.** Persistent tasks should not have
dependents. They should be the last thing that runs.

---

## `affectedFiles` misconfiguration

The `affectedFiles` option passes affected file paths to the task's command as arguments (and/or as
the `MOON_AFFECTED_FILES` env var). This only works when `--affected` is passed to `moon run` or
`moon exec`.

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

By default, when no files are affected, `.` (current directory) is passed as the argument. Set
`passInputsWhenNoMatch: true` to pass the task's `inputs` list instead.

> **Note:** The v1 option `affectedPassInputs` was removed in v2. Use
> `affectedFiles.passInputsWhenNoMatch` instead.

### Key point

`affectedFiles` does nothing unless `--affected` is passed on the command line. If you set it in
config but always run `moon run <target>` without `--affected`, the setting has no effect.

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

**Circular extension:** Task A extends B, B extends A. moon should catch this, but it's worth
checking if you see strange behavior.

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

| Value                           | Local       | CI (affected) | CI (not affected) |
| ------------------------------- | ----------- | ------------- | ----------------- |
| `true` / `'affected'` (default) | Runs        | Runs          | Skipped           |
| `false`                         | Runs        | Skipped       | Skipped           |
| `'always'`                      | Runs        | Runs          | Runs              |
| `'only'`                        | **Skipped** | Runs          | Skipped           |
| `'skip'`                        | Runs        | **Skipped**   | Skipped           |

### Common surprises

**`'only'`** — the task is CI-only. Running `moon run app:deploy` locally does nothing. This trips
people up when they try to test a CI task locally.

**`'skip'`** — the task is skipped in CI but task relationships (deps) remain valid. Unlike `false`,
downstream tasks that depend on a `'skip'` task won't break in CI.

**`'always'`** — the task always runs in CI regardless of affected status. Useful for tasks like
`deploy` that should run on every merge to main, even if no inputs changed.

### How to detect

```bash
moon task <project>:<task> --json | grep -i runci
# Also check state.setRunInCi — if true, it was explicitly configured
```

---

## `allowFailure` hiding errors

When `options.allowFailure` is `true`, the task reports success even when the underlying command
exits with a non-zero code. The pipeline continues as if nothing went wrong.

```yaml
tasks:
  advisory-lint:
    command: 'eslint src/'
    options:
      allowFailure: true # Lint failures are warnings, not blockers
```

This is intentional for advisory tasks. But if it's inherited from a global task and the user
doesn't realize it's set, real errors go unnoticed.

**Gotcha with deps:** If task A has `allowFailure: true` and task B depends on A, B will execute
even if A's command failed. moon's task builder validates that `allowFailure` deps are acceptable,
but the runtime behavior can still surprise.

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
B acquires the mutex, completes, then A acquires it. This is fine. But if you have a cycle in deps +
shared mutex, the pipeline can deadlock.

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
slower machines, you may need a longer timeout.

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
retries.

---

## `os` platform filtering

The `os` option restricts a task to specific operating systems. If the current platform doesn't
match, the task is silently skipped.

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

- **Last run time:** if the previous run's timestamp exceeds the lifetime, the cached result is
  skipped and the task re-executes.
- **Archive file:** if the `.tar.gz` archive in `.moon/cache/outputs/` is older than the lifetime,
  hydration is rejected and the task re-executes.

Additionally, `moon clean --lifetime` uses this value to remove stale archives from disk.

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

# Run quality-tagged tasks in upstream projects
moon run '^:#quality'

# Run quality-tagged tasks in a specific project
moon run 'app:#quality'
```

> The `#` is a shell comment marker, so `#tag` targets must be quoted (or escaped with `\#`) on the
> command line.

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

**Tag vs project tag confusion** <sup>MQL rename</sup>

The MQL `tag` field was renamed to `projectTag` in v2.3. A new `taskTag` field queries by task tag.
Stale queries using the old `tag=...` syntax now error.

```bash
# Pre-v2.3
moon query tasks --query "tag=quality"

# v2.3+
moon query tasks --query "projectTag=quality"  # project tag
moon query tasks --query "taskTag=quality"     # task tag
moon query tasks --tags quality                # convenience flag
```

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
        hash: 'stdout' # true | 'exit-code' | 'stdout' | 'stderr'
```

### Behavior by type

| Type          | Script passes (exit 0)             | Script fails (non-zero)                                  |
| ------------- | ---------------------------------- | -------------------------------------------------------- |
| `requirement` | Task continues                     | Task **fails** — `RequirementCheckFailed`, does not run  |
| `condition`   | Counts toward skipping (see below) | Task runs as normal                                      |
| `fingerprint` | Output mixed into hash             | Task **fails** — `FingerprintCheckFailed` before hashing |

**Conditions skip, they don't gate.** The task is skipped **only when _all_ `condition` checks
pass**. If any condition fails, the task runs as normal. This is the inverse of a requirement, and a
common source of "my task never runs" confusion.

### Common surprises

**"My task fails with a requirement/fingerprint check error"**

```
Task app:deploy is unable to run as the requirement check `command -v aws` failed.
```

The named script exited non-zero (or, for fingerprints, crashed). The diagnostic codes are
`task_runner::requirement_check_failed` and `task_runner::hash_check_failed`. Run the script
manually to see why it fails.

**"My task is skipped even though inputs changed and it's not a cache hit"**

All `condition` checks passed, so moon skipped the task on purpose. The target ends in a `Skipped` /
`SkippedConditional` state. Confirm with debug logs:

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

1. Global `.moon/tasks/*` `taskOptions` (workspace-wide defaults, since v1.20).
2. Project `moon.*` `taskOptions` <sup>v2.4+</sup> (project-wide defaults).
3. Per-task `options` (most specific, wins).

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

**`PersistentDepRequirement`** — a non-persistent task depends on a persistent task. This is always
a configuration error because the persistent task never finishes. Fix: remove the dependency or
restructure the task graph.

**`AllowFailureDepRequirement`** — a task depends on a task with `allowFailure: true`. moon warns
about this because a failing dependency will still let the dependent task run, which may produce
incorrect results.

**`RunInCiDepRequirement`** — a task that runs in CI depends on a task that doesn't run in CI
(`runInCI: false`). The dependency won't execute in CI, so the dependent task may fail or produce
incorrect results.

**`InvalidCommandSyntax` / `UnsupportedCommandSyntax`** — the `command` field contains shell syntax
(pipes, redirects, `&&`) that should use `script` instead.

**`UnknownExtendsSource`** — the `extends` field references a task that doesn't exist in the current
project or global scope.

**`UnknownDepTarget`** — a `deps` entry references a target that doesn't exist. Check for typos in
the project or task name.

**"a shell script is required for a task check"** <sup>v2.4+</sup> — a `checks` entry has an empty
or whitespace-only `script`. Every check must define a non-empty shell script.
