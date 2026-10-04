# Environment and debug Tools

This reference covers the debug environment variables, log levels, and inspection tools available
for deep debugging of moon tasks.

---

## Table of contents

1. [Debug environment variables](#debug-environment-variables)
2. [Log levels](#log-levels)
3. [Inspection commands](#inspection-commands)
4. [Trace profiling](#trace-profiling)
5. [Cache file locations](#cache-file-locations)
6. [Recommended debug workflows](#recommended-debug-workflows)

---

## Debug environment variables

moon provides several environment variables that reveal internal state during task execution. Set
them before running `moon run`:

| Variable                   | What it reveals                                                                                                                                                                                                                                                                                                                                                   |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MOON_DEBUG_DAEMON`        | Adds gRPC transport (`tonic`) logs for daemon RPCs to the main process's log. The daemon server's own logs always go to `.moon/cache/daemon/server.log` (`moon daemon logs`).                                                                                                                                                                                     |
| `MOON_DEBUG_MCP`           | Debug output from MCP server interactions.                                                                                                                                                                                                                                                                                                                        |
| `MOON_DEBUG_PROCESS_ENV`   | Logs **every** variable moon explicitly sets on the command (task `env`, plugin-injected and activated variables, `MOON_*`) — by default only `MOON_*`/`PROTO_*` keys are logged. Variables inherited from the shell and `PATH` additions are never shown, and a task `env` value is shown even when a shell value overrides it. Requires `--log debug` or lower. |
| `MOON_DEBUG_PROCESS_INPUT` | Logs the full stdin in the "Running command" debug line, which otherwise truncates input over 200 bytes. The input itself is never truncated. Requires `--log debug` or lower.                                                                                                                                                                                    |
| `MOON_DEBUG_REMOTE`        | Debug output from remote caching — connection errors, sync status.                                                                                                                                                                                                                                                                                                |
| `MOON_DEBUG_WASM`          | Debug output from WASM plugins — loading, execution, memory profiles.                                                                                                                                                                                                                                                                                             |

### Usage

```bash
# Reveal env vars passed to the process (most common debug need)
MOON_DEBUG_PROCESS_ENV=true moon run <project>:<task> --log trace --force

# Full debug output for a failing task
MOON_DEBUG_PROCESS_ENV=true MOON_DEBUG_PROCESS_INPUT=true \
  moon run <project>:<task> --log trace --force

# Debug remote caching issues
MOON_DEBUG_REMOTE=true moon run <project>:<task> --log debug

# Debug toolchain installation (use --log debug; no dedicated env var exists)
moon run <project>:<task> --log debug --force
```

### Behavior-changing env vars (check for overrides!)

Separate from the debug vars above, moon reads env vars that **override configuration** — a shell
profile or CI environment exporting one of these can make behavior diverge from what the config
says. When "the config says X but moon does Y", check for these first:

```bash
env | grep '^MOON_'
```

| Variable                                            | Overrides                                                                                                                                                                |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `MOON_DAEMON`                                       | `unstable_daemon` — enables/disables the background daemon                                                                                                               |
| `MOON_EXPERIMENT_*`                                 | Any `experiments.*` flag (async graph/affected, hashing, CAS, output style)                                                                                              |
| `MOON_TOOLCHAIN_FORCE_GLOBALS`                      | Uses tools from `PATH` instead of the toolchain (`true`, or a list of tool IDs) — their paths aren't prepended, and <sup>v2.6+</sup> their environments aren't activated |
| `MOON_CACHE`                                        | The `--cache` mode (`read`, `read-write`, `write`, `off`)                                                                                                                |
| `MOON_CACHE_CAS_MAX_SIZE` <sup>v2.5+</sup>          | `cache.cas.maxSize` — CAS eviction limit                                                                                                                                 |
| `MOON_CACHE_CAS_VERIFY_INTEGRITY` <sup>v2.5+</sup>  | `cache.cas.verifyIntegrity`                                                                                                                                              |
| `MOON_CACHE_SHARED_WORKTREE_CACHE` <sup>v2.5+</sup> | `cache.unstable_sharedWorktreeCache`                                                                                                                                     |
| `MOON_BASE` / `MOON_HEAD`                           | Base/head revisions for affected detection                                                                                                                               |

### Environment variables in task config

Tasks can declare env vars that are passed to the process and included in the hash:

```yaml
tasks:
  build:
    command: 'vite build'
    env:
      NODE_ENV: 'production'
```

The **configured** values in `env` are included in the hash. If you change `NODE_ENV` from
`production` to `development` in the config, the hash changes and the cache misses.

But a task's `env` value is only applied when the variable **isn't already set** in the shell — an
exported shell value wins at runtime, while the hash still uses the configured value. And env vars
**not** declared in `env` (but present in the shell) are passed to the process without affecting the
hash. So neither way makes a different shell `NODE_ENV` cause a cache miss. To hash the **actual**
value from the environment, add it to `inputs` with the `$` prefix (`inputs: ['$NODE_ENV']`).

<sup>v2.5+</sup> A task's resolved `env` can also include variables inherited from a
**workspace-level `env`** in `.moon/tasks/**/*` files, merged beneath the project's own `env`
(project wins on conflict, unless `workspace.mergeStrategies.env` changes the strategy). If a
variable has a value that appears nowhere in the project config, check the global task files. See
`config-mistakes.md` § Workspace-inherited env.

<sup>v2.6+</sup> Toolchains can also **activate their environment** when moon builds a command (for
tasks and toolchain operations), via proto's `activate_environment` plugin function — the same one
behind `proto activate`. This may set variables (like `JAVA_HOME`) and prepend paths to `PATH`, so a
variable can appear in the process without being configured anywhere:

- Only toolchains with a configured `version` that have been setup are activated.
- Variables already configured for the command (task `env`, or those injected by toolchain plugins)
  are never overridden. When a task has multiple toolchains, the **first** one wins.
- Activated variables **do** override the same variable exported in the shell (like a `JAVA_HOME`
  set by CI), and they take precedence over `options.envFile` values for the same key (env files are
  loaded afterwards, and only fill missing keys).
- `MOON_TOOLCHAIN_FORCE_GLOBALS` disables activation (along with toolchain paths).
- `moon toolchain info <id>` lists `activate_environment` under the tier 3 APIs, marked 🟢 when the
  toolchain implements it (⚫️ otherwise).

```bash
# See the variables moon sets on the command (including activated ones)
MOON_DEBUG_PROCESS_ENV=true moon run <project>:<task> --log debug --force
```

Activated variables are not part of the task's hash, like other variables that aren't declared in
`env`.

---

## Log levels

Control verbosity with the `--log` global option or `MOON_LOG` environment variable.

| Level     | What you see                                                                                                                          |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `off`     | Nothing                                                                                                                               |
| `error`   | Only errors                                                                                                                           |
| `warn`    | Warnings and above                                                                                                                    |
| `info`    | (default) Status messages, task output                                                                                                |
| `debug`   | Internal decisions — hash generation, cache checks, toolchain resolution                                                              |
| `trace`   | Everything from moon's own crates (moon, proto, starbase, warpgate) — network requests, child process details, file system operations |
| `verbose` | `trace` for **all** crates (including third-party dependencies), with span-name prefixes showing nesting                              |

### Recommendations

- **Start with `debug`** for most issues. As of v2.4 this is the recommended level for day-to-day
  debugging — it includes the majority of useful diagnostic information without drowning you in
  noise.
- **Escalate to `trace`** only if `debug` doesn't reveal the problem. <sup>v2.4+</sup> `trace` was
  made _significantly_ more verbose and is now primarily intended for agents and deep diagnostics —
  it may be too spammy for normal debugging, so pipe it to a file.
- **Use `verbose`** when you need logs from third-party crates, or to see which span a log line is
  nested in. It doesn't show timing — for performance, use `--dump` (trace profile) or `--otel`.

```bash
# Debug level (recommended starting point)
moon run <project>:<task> --log debug --force

# Trace level, saved to file for analysis
moon run <project>:<task> --log trace --force 2>&1 | tee moon-trace.log

# Or use the MOON_LOG env var
MOON_LOG=debug moon run <project>:<task> --force

# Write logs to a specific file
moon run <project>:<task> --log trace --log-file debug.log --force
```

---

## Inspection commands

These commands let you examine moon's internal state without running tasks.

### `moon task` — inspect resolved task config

```bash
# Human-readable output
moon task <project>:<task>

# Machine-readable JSON
moon task <project>:<task> --json
```

Shows the fully resolved task configuration after inheritance, merging, and token resolution. This
is the single most useful debugging command — always start here.

**Tip:** Running `moon task <project>:<task>` without `--json` also displays all available `PATH`s
for the resolved toolchain. <sup>v2.6+</sup> It also labels deps that don't run before the task with
their type, like `db:stop (cleanup)` or `web:serve (wait)`.

### `moon project` — inspect project metadata

```bash
# Human-readable output
moon project <project>

# Machine-readable JSON
moon project <project> --json
```

Shows project metadata: language, toolchain, stack, layer, tags, dependencies, file groups, and all
configured tasks.

### `moon task-graph` / `moon project-graph` — visualize graphs

```bash
# Visualize the task dependency graph
moon task-graph <project>:<task>
# --dot, or --json for external analysis

# Visualize the project dependency graph
moon project-graph <project>
# --dot, or --json for external analysis
```

These show task-level and project-level dependency relationships respectively, complementing the
lower-level action graph below.

### `moon action-graph` — visualize the dependency graph

```bash
# Print the graph (use these as an agent)
moon action-graph <project>:<task> --dot
moon action-graph <project>:<task> --json

# Focus on a specific target and include its dependents
moon action-graph <project>:<task> --dependents --dot

# Open an interactive visualization in the browser (humans only — starts a local web server and
# blocks until interrupted)
moon action-graph <project>:<task>
```

The action graph shows every action moon will take to run a target: toolchain setup, dependency
installation, project sync, and task execution. It's the best tool for diagnosing:

- Why a task depends on something unexpected
- Why a pipeline never finishes (a requested persistent task runs until interrupted, by design; or
  <sup>v2.6+</sup> a `wait` dep that is never stopped)
- <sup>v2.6+</sup> Where cleanups run — a `cleanup` dep is linked as a _dependent_ of the task it
  cleans up after (the edge is reversed)
- Whether tasks are running in parallel or serial

### `moon hash` — inspect and compare hashes

```bash
# Show hash manifest (all sources that generated the hash)
moon hash <hash>

# Compare two hashes
moon hash <hash1> <hash2>

# JSON output
moon hash <hash> --json
```

<sup>v2.6+</sup> Hash manifests are stored as blobs in the local content-addressable cache
(`.moon/cache/blobs/`) instead of `.moon/cache/hashes/<hash>.json`, so `moon hash` is the way to
read them. "Unable to find a hash manifest" means it was never stored, or was garbage collected: no
task manifest references a hash manifest, so any garbage collection deletes hash manifests older
than an hour (regardless of `--lifetime`). GC runs on `moon clean`, and after a successful pipeline
when the daemon is connected and `pipeline.autoCleanCache` is enabled (the default). Re-running the
task regenerates the manifest.

> For interpreting hash diffs in cache investigations, see `cache-issues.md`.

### `moon query` — query project and task information

```bash
# Find all projects matching criteria
moon query projects --language typescript
moon query projects --stack frontend

# Find all tasks across projects
moon query tasks
moon query tasks --project <project>

# v2.3+: filter tasks by tag
moon query tasks --tags quality
```

The MQL query is a **positional** argument — `moon query` has no `--query` flag (only `moon run` and
`moon exec` do).

**MQL `tag` vs `taskTag`** (v2.3+): MQL's `tag` field is a legacy alias for `projectTag` (project
tags), while `taskTag` matches task tags. On task queries, only `taskTag` works — `projectTag`/`tag`
(like other project fields, except `project`) silently match **nothing**. To filter by project tags,
query projects instead, or use `--project <regex>` on task queries.

```bash
moon query tasks --tags quality              # by task tag (regex flag)
moon query tasks "taskTag=quality"           # by task tag (MQL)
moon query projects "projectTag=frontend"    # by project tag
moon query projects "taskTag=quality"        # projects containing a task tagged quality
```

> In v2.3–v2.4, no tag field worked on task queries — they silently matched nothing. v2.5 fixed
> `taskTag` only; use the `--tags` flag on older versions.

---

## Trace profiling

For performance issues that need microsecond-level analysis:

```bash
# Generate a trace profile
moon run <project>:<task> --dump --force
```

This creates a JSON trace file in the current working directory. Open it in:

- **Chrome DevTools:** Navigate to `chrome://tracing` and load the file
- **Perfetto:** Upload to `ui.perfetto.dev`

The trace shows:

- Toolchain setup time
- Dependency installation time
- Hash generation time (including file system operations)
- Process execution time
- Cache read/write time

This is the most granular debugging tool. Use it when you know something is slow but can't tell what
from the logs alone.

### OpenTelemetry export <sup>v2.5+</sup>

The same span data can be exported over OTLP to an observability backend (Grafana, Honeycomb,
Jaeger, etc), which is useful for comparing runs over time or debugging CI performance where you
can't open a local trace file:

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4318"
moon --otel run <project>:<task>

# Also export log events as OTLP logs (respects --log level)
moon --otel --otel-logs run <project>:<task> --log debug
```

<sup>v2.6+</sup> `--otel` also exports **metrics**, which are useful for spotting trends across many
runs rather than debugging a single one:

- `moon.task.runs` / `moon.task.duration` — labeled with `target`, `project`, `task`, `toolchains`,
  `status` (`passed`, `failed`, `cached`, `cached-from-remote`, `skipped`), and `flaky` (passed only
  after a retry). Group by `target` to find slow or flaky tasks, or compare cached statuses against
  the total for a cache hit rate.
- `moon.action.*` and `moon.operation.*` — per action (sync, setup, install, run) and per operation
  within an action (hash generation, hydration, execution), for finding where time goes.

Persistent tasks are not recorded, as they never complete. `OTEL_METRICS_EXPORTER=none` disables
metrics while keeping traces.

See the [OpenTelemetry docs](https://moonrepo.dev/docs/commands/overview#opentelemetry) for
transports, the full env var list, and all metric attributes.

---

## Cache file locations

Quick reference for where moon stores internal state:

```
.moon/cache/
  blobs/<ab>/<cdef…>              # First 2 hash chars / rest of the hash; hash manifests (v2.6+)
                                  # and CAS outputs (when enabled)
  daemon/                         # Daemon server state and logs
  hashes/<hash>.json              # Hash manifest — before v2.6 only (now in blobs/)
  outputs/<hash>.tar.gz           # Archived task outputs (legacy / default)
  states/<project>/
    snapshot.json                 # Project snapshot (resolved tasks, config)
    <task>/
      lastRun.json                # Last run metadata (exit code, hash)
      stdout.log                  # Captured stdout from last run
      stderr.log                  # Captured stderr from last run
```

All paths are relative to the workspace root. The `.moon/cache/` directory should be git-ignored.

> When `experiments.casOutputsCache` is enabled (v2.3+), new task outputs are stored in a
> content-addressable store at `.moon/cache/manifests/` and `.moon/cache/blobs/` (prefix-sharded by
> hash; renamed in v2.4 from `ac/` and `cas/`) — per-hash `.tar.gz` files stop being created in
> `outputs/`. See `cache-issues.md` § Experimental caching layers. <sup>v2.6+</sup> The local CAS is
> always enabled, as it stores hash manifests, so `blobs/` exists even with the experiment off.

> <sup>v2.5+</sup> Two additions change where to look. With `cache.sharedWorktreeCache` (alias
> `unstable_sharedWorktreeCache`) enabled, and moon running inside a git worktree, `blobs/` and
> `manifests/` move to the **base checkout's** `.moon/cache` (or `~/.moon/cache/shared` when the
> repository root has no moon config) — the worktree's own copies may be empty. In v2.6+, this
> includes hash manifests, since they're blobs. And with the daemon enabled (and outputs going to
> the local CAS or a remote cache), archive/hydrate storage errors are recorded only in
> `.moon/cache/daemon/server.log` (`moon daemon logs`), not in the main process output — a failed
> hydrate surfaces as a plain cache miss (the task re-runs).

---

## Recommended debug workflows

### "My task fails and I don't know why"

```bash
# 1. Check the config first
moon task <project>:<task> --json

# 2. Run with debug logging
moon run <project>:<task> --log debug --force

# 3. If the error is about env vars or missing input
MOON_DEBUG_PROCESS_ENV=true moon run <project>:<task> --log trace --force

# 4. Check stderr from last run
cat .moon/cache/states/<project>/<task>/stderr.log
```

### "My task fails, skips, or re-runs because of a check" <sup>v2.4+</sup>

`checks` run before the task and emit debug logs. Grep for them to see which check fired:

```bash
moon run <project>:<task> --log debug --force 2>&1 | grep -i "check\|condition\|requirement"

# Signals to look for:
#   "Running task check"                                       → a check executed
#   "Checking requirement" (+ non-zero exit_code)              → requirement failed → task fails
#   "Skipping task as all conditional checks have passed"      → condition skip
#   "Will continue to run the task as not all conditional…"    → conditions didn't all pass → ran
#   "Checking condition"                                       → a condition check executed
#   "Task check timed out"                                     → check hit options.timeout
#     (silently non-fatal: requirement counts as passing, fingerprint hashes nothing)
```

See `config-mistakes.md` § Task checks and `cache-issues.md` § Fingerprint checks in the hash.

### "My task is cached when it shouldn't be"

```bash
# 1. Inspect the hash
moon hash <hash>

# 2. Force a run and compare hashes
moon run <project>:<task> --force
moon hash <old-hash> <new-hash>

# 3. The diff shows what inputs are missing
```

### "My task re-runs every time"

```bash
# 1. Run twice and capture both hashes
moon run <project>:<task> --force
# note the hash
moon run <project>:<task> --force
# note the new hash

# 2. Diff to see what changed
moon hash <hash1> <hash2>

# 3. The changing field is too volatile — narrow inputs or fix outputs
```

### "My pipeline hangs"

```bash
# 1. Visualize the graph (always pass --dot or --json as an agent; without them the
#    command starts a web server and blocks)
moon action-graph <project>:<task> --dot

# 2. Is a persistent task part of the run? It runs until interrupted, by design.
#    (A non-persistent task with a required dep on one is a build error, not a hang.)
# 3. v2.6+: look for `wait` deps without a `cleanup` dep that stops them
moon task <project>:<task>   # deps labeled "(wait)" / "(cleanup)"

# 4. Look for long-running commands that aren't marked `persistent`, and for a
#    persistent/wait task holding a `mutex` that another task needs
```

### "My cleanup dependency didn't run" <sup>v2.6+</sup>

```bash
# 1. Confirm the dep type
moon task <project>:<task>

# 2. Look for the skip reason
moon run <project>:<task> --log debug 2>&1 | grep -i "clean"
#   "Skipping cleanup job, as there's nothing to clean up"
#     → the task was hydrated from the cache or skipped, and didn't start a wait dep
#   "Pipeline was aborted, running cleanup jobs for the jobs that have ran"
#     → a failure aborted the pipeline; cleanups still run for tasks that ran

# 3. Interrupted with Ctrl+C? Cleanups don't run on signals.
# 4. In CI? A cleanup with runInCI disabled doesn't run there.
```

See `config-mistakes.md` § Task dep `type`.
