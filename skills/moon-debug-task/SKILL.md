---
name: moon-debug-task
description: >-
  Diagnose and fix moon tasks that are broken, misconfigured, or behaving
  unexpectedly. Use this skill when a moon task is failing, not running,
  skipped, hanging, producing stale or wrong output, cached when it shouldn't
  be, re-running every time when it should be cached, or when outputs are
  empty or missing after a cache hit. Also covers pipeline hangs and freezes,
  tasks that only work in CI but not locally (or vice versa), tasks skipped
  by --affected, and task inheritance not applying to a project. Activate on
  any mention of "moon run" or "moon task" combined with a problem — errors,
  stale cache, missing outputs, wrong results, "nothing to do", or unexpected
  behavior. Also use for task options like persistent, runInCI, allowFailure,
  affectedFiles, mutex, timeout, or cacheLifetime. This skill is for
  diagnosing existing tasks, not for creating new tasks, setting up
  workspaces, configuring toolchains, or learning moon concepts.
license: MIT
allowed-tools: Bash(moon:*) Read
compatibility: >-
  Requires moon >= 2.0.0 CLI installed and a configured moon workspace.
metadata:
  moon-version-min: "2.0.0"
  moon-version-tested: "2.1.1"
  category: "debugging"
  ecosystem: "moonrepo"
---

# Moon Task Debugger

A workflow-oriented diagnostic skill for troubleshooting moon tasks. This is not
a reference manual — it guides you through a structured debugging flow so you
can isolate the problem quickly.

For conceptual background, see the [moon documentation](https://moonrepo.dev/docs).

---

## Quick-start: 5-step diagnostic flow

Work through these steps in order. Most issues resolve by step 3.

### Step 1: Inspect the resolved task configuration

The first thing to check is whether the task is configured the way the user
expects. Moon merges configuration from multiple sources (global tasks, project
config, inheritance), so the resolved result can surprise people.

```bash
# Show the fully resolved task config (with inheritance applied)
moon task <project>:<task>

# Machine-readable version for programmatic inspection
moon task <project>:<task> --json
```

**What to verify:**
- `command` vs `script` — if the command contains pipes (`|`), redirects (`>`),
  or chained commands (`&&`), it must use `script`, not `command`
- `inputs` — are they too broad (`**/*` captures everything) or too narrow
  (missing source files)? Check `state.default_inputs` (true = using default
  `**/*`) and `state.empty_inputs` (true = explicitly set to `[]`)
- `outputs` — are they declared for build tasks? Missing outputs means the cache
  can never hydrate artifacts
- `toolchain` — is the correct toolchain assigned? An incorrect toolchain means
  wrong tool versions
- `deps` — are task dependencies correct and complete?
- `options` — check `persistent`, `runInCI`, `cache`, `affectedFiles`, `mutex`,
  `timeout`, `retryCount`, `allowFailure`, `os`
- `type` — `build` (has outputs), `test` (default), or `run` (persistent)
- `preset` — `server` or `utility` apply multiple option defaults at once

**Red flags:**
- `command: 'eslint . && prettier --check .'` — pipes/chains in `command` fail
  silently or error. Use `script` instead
- Empty `outputs` on a build task — cache will never restore artifacts
- `inputs: ['**/*']` — too broad, cache invalidates on every change
- A `persistent` task in a `deps` chain — downstream tasks hang forever because
  the persistent process never "completes"
- `command: 'noop'` or `nop` / `no-op` — the task is intentionally a no-op and
  does nothing. Moon treats these specially
- `runInCI: 'only'` — task runs in CI but NOT locally (common surprise)
- `runInCI: 'skip'` — task is skipped in CI but relationships remain valid
- `os` set to a platform the user isn't on — task silently skips
- `allowFailure: true` — task errors are swallowed, can mask real problems

### Step 2: Run with maximum verbosity

If the config looks right, run the task with debug logging to see what moon is
actually doing under the hood.

```bash
# Debug-level logging with cache bypass
moon run <project>:<task> --log debug --force

# Deep debugging: reveal env vars and stdin passed to the process
MOON_DEBUG_PROCESS_ENV=true MOON_DEBUG_PROCESS_INPUT=true \
  moon run <project>:<task> --log trace --force
```

**What to look for in the logs:**
- Toolchain resolution — is the right version of node/deno/bun/etc being used?
- Hash generation — what sources are being hashed?
- Affected status — is the task being skipped because it's "not affected"?
- Process execution — what command is actually being spawned?

**Visualize the execution graph** to spot dependency issues:

```bash
# Interactive graph visualization (opens in browser)
moon action-graph <project>:<task>

# Export to DOT format for external tools
moon action-graph <project>:<task> --dot > graph.dot

# JSON format for programmatic analysis
moon action-graph <project>:<task> --json
```

Look for:
- Circular dependencies
- Missing dependencies (task runs before its prerequisite)
- A persistent task node that downstream tasks depend on (they'll hang)

### Step 3: Inspect cache state

If the task runs but produces wrong results, or runs when it shouldn't (or
doesn't run when it should), the cache is the likely culprit.

```bash
# Inspect a hash manifest to see what inputs were hashed
moon hash <hash>

# Compare two hashes to see what changed between runs
moon hash <hash1> <hash2>

# Short-form hashes work too
moon hash 0b55b234 2388552f
```

**Key cache locations:**
- `.moon/cache/hashes/<hash>.json` — hash manifest (all sources used to generate the hash)
- `.moon/cache/outputs/<hash>.tar.gz` — archived task outputs
- `.moon/cache/states/<project>/snapshot.json` — project snapshot with resolved tasks
- `.moon/cache/states/<project>/<task>/lastRun.json` — last run metadata (exit code, hash)
- `.moon/cache/states/<project>/<task>/stdout.log` — captured stdout
- `.moon/cache/states/<project>/<task>/stderr.log` — captured stderr

> For deeper cache diagnosis, read `references/cache-issues.md`.

### Step 4: Diagnose the problem type

Use this table to jump to the right reference:

| Symptom | Likely cause | Quick check | Reference |
|---------|-------------|-------------|-----------|
| Task doesn't exist | Inheritance not applied — check `inheritedBy` conditions in `.moon/tasks/**/*` against project's `toolchain`, `stack`, `layer`, `tags` via `moon project <name> --json` | `moon task <target> --json` | `references/config-mistakes.md` |
| "Nothing to do" | `--affected` + no changes, `runInCI: false`, or `inheritedBy` mismatch (global task not inherited) | Check flags, `options.runInCI`, and `inheritedBy` | `references/decision-tree.md` |
| Task errors on execution | Wrong `command`/`script`, bad toolchain | `moon run <target> --log debug` | `references/config-mistakes.md` |
| Stale cache (cached when it shouldn't be) | Inputs too narrow, missing `env` vars | `moon hash <hash>` | `references/cache-issues.md` |
| Cache miss (re-runs every time) | Inputs too broad, volatile outputs | `moon hash <h1> <h2>` | `references/cache-issues.md` |
| Outputs not restored after cache hit | `outputs` misconfigured | Check `.moon/cache/outputs/` | `references/cache-issues.md` |
| Task hangs / pipeline stuck | Persistent task in `deps` chain | `moon action-graph <target>` | `references/config-mistakes.md` |
| Task is slow | Dep chain bottleneck, no parallelism | `moon action-graph <target>` | `references/decision-tree.md` |
| Task does nothing (no-op) | Command is `noop`/`nop`/`no-op` | `moon task <target> --json` | `references/config-mistakes.md` |
| Task fails silently | `allowFailure: true` hiding errors | Check `options.allowFailure` | `references/config-mistakes.md` |
| Task skipped locally | `runInCI: 'only'` set | Check `options.runInCI` | `references/config-mistakes.md` |
| Task skipped in CI | `runInCI: false` or `'skip'` | Check `options.runInCI` | `references/config-mistakes.md` |
| Mutex contention / deadlock | Two tasks share same `mutex` | Check `options.mutex` | `references/config-mistakes.md` |
| Task times out | `timeout` option set too low | Check `options.timeout` | `references/config-mistakes.md` |

### Step 5: Validate the fix

After making changes, verify the fix actually worked:

```bash
# Bypass cache to force a fresh run
moon run <project>:<task> --force

# Disable cache entirely (no reads OR writes)
moon run <project>:<task> --cache off

# Verify the resolved config reflects your changes
moon task <project>:<task> --json
```

**`--force` vs `--cache off`:**
- `--force` ignores existing cache but **writes** new cache after execution
- `--cache off` disables caching entirely — no reads, no writes

---

## Common anti-patterns

These are the mistakes that come up most often. If the user's problem matches
one of these, you can skip the diagnostic flow and go straight to the fix.

### Pipes or chains in `command`

```yaml
# WRONG: command only accepts a single binary
tasks:
  lint:
    command: 'eslint . && prettier --check .'

# RIGHT: use script for pipes, redirects, or chained commands
tasks:
  lint:
    script: 'eslint . && prettier --check .'
```

`command` is for a single binary with arguments. It supports inheritance merge
strategies (append, prepend, replace). `script` supports shell syntax but does
not support merge strategies.

### Missing outputs on build tasks

```yaml
# WRONG: no outputs declared — cache never hydrates artifacts
tasks:
  build:
    command: 'vite build'

# RIGHT: declare what the build produces
tasks:
  build:
    command: 'vite build'
    outputs:
      - 'dist'
```

Without `outputs`, moon has nothing to archive or restore on cache hit.

### Overly broad inputs

```yaml
# WRONG: **/* captures node_modules, .moon/cache, everything
tasks:
  test:
    command: 'vitest'
    inputs:
      - '**/*'

# RIGHT: be specific about what affects the task
tasks:
  test:
    command: 'vitest'
    inputs:
      - 'src/**/*'
      - 'tests/**/*'
      - 'vitest.config.*'
```

Broad inputs mean the hash changes on every run, defeating the cache.

### Volatile outputs that change every run

If outputs include files with timestamps, absolute paths, or random content
(like sourcemaps with absolute paths), the cache will always appear "stale."
Exclude volatile files from `outputs` or normalize them.

### Persistent task blocking the pipeline

```yaml
# WRONG: dev-server is persistent and blocks downstream tasks
tasks:
  dev-server:
    command: 'vite dev'
    preset: 'server'  # marks as persistent
  integration-test:
    command: 'cypress run'
    deps:
      - '~:dev-server'  # hangs forever waiting for dev-server to "finish"

# RIGHT: persistent tasks should be leaf nodes, not dependencies
# Or restructure so integration-test doesn't depend on dev-server
```

A persistent task (or one with `preset: 'server'`) never completes. If another
task lists it in `deps`, the pipeline hangs. Diagnose with
`moon action-graph <target>` and look for persistent nodes with dependents.

Note: tasks named `dev`, `start`, or `serve` are automatically marked as
`server` preset.

### Confusing `--affected` with `--force`

These are opposites:
- `--affected` **restricts** execution to tasks whose inputs changed
- `--force` **bypasses** cache and runs everything regardless

Using `--affected` when you want to force a run (or vice versa) is a common
source of "why didn't my task run?" confusion.

### `allowFailure` masking real errors

```yaml
# DANGEROUS: task errors are silently swallowed
tasks:
  lint:
    command: 'eslint src/'
    options:
      allowFailure: true
```

If a task has `allowFailure: true`, it will report success even when the
underlying command fails. This is intentional for advisory tasks, but if the
user doesn't realize it's set (e.g., inherited from a global task), real errors
go unnoticed. Check `moon task <target> --json` and inspect `options.allowFailure`.
To see the actual error output, check the captured stderr:

```bash
cat .moon/cache/states/<project>/<task>/stderr.log
```

### Mutex deadlocks between tasks

```yaml
# PROBLEM: two tasks share a mutex and depend on each other
tasks:
  build-a:
    command: 'build-a'
    options:
      mutex: 'build-lock'
  build-b:
    command: 'build-b'
    options:
      mutex: 'build-lock'
    deps: ['~:build-a']  # safe — sequential via deps
```

The `mutex` option ensures only one task with that mutex name runs at a time.
This is useful for tasks that write to shared resources. But if combined
incorrectly with `deps`, it can cause unexpected serialization or deadlocks.

### Wrong `runInCI` variant

```yaml
tasks:
  deploy:
    command: 'deploy.sh'
    options:
      runInCI: 'only'    # ONLY runs in CI, skipped locally
  e2e:
    command: 'playwright test'
    options:
      runInCI: 'skip'    # Skipped in CI but runs locally, deps stay valid
```

The `runInCI` option accepts: `true`/`'affected'` (default — run if affected),
`false` (never in CI), `'always'` (always in CI even if not affected),
`'only'` (CI only, skip local), `'skip'` (skip CI, run local).

---

## When to load references

Each reference file covers a specific problem domain in depth. Load them only
when the diagnostic flow points you there — don't load everything upfront.

| Reference | When to load |
|-----------|-------------|
| `references/decision-tree.md` | When the symptom doesn't match the quick table above, or you need a systematic walk-through of all possibilities |
| `references/cache-issues.md` | When the problem is clearly cache-related: unexpected hits, unexpected misses, outputs not restoring |
| `references/config-mistakes.md` | When the task config is wrong: command vs script, inheritance bugs, presets, persistent tasks, affectedFiles, mutex, timeout, retries, runInCI variants, allowFailure, os |
| `references/environment-debug.md` | When you need to go deeper with env vars, log levels, trace profiles, or inspection tools |
