---
name: debug-task
description: >-
  Diagnose and fix moon tasks that are broken, misconfigured, or behaving unexpectedly. Use this
  skill when a moon task is failing, not running, skipped, hanging, producing stale or wrong output,
  cached when it shouldn't be, re-running every time, or missing outputs after a cache hit. Also
  covers pipeline hangs, tasks that only work in CI but not locally (or vice versa), tasks skipped
  by --affected, and task inheritance not applying to a project. Activate on any mention of "moon
  run" or "moon task" combined with a problem — errors, stale cache, missing outputs, wrong results,
  "nothing to do", or unexpected behavior. Also use for task options (persistent, runInCI,
  allowFailure, affectedFiles, mutex, timeout, outputStyle, cacheLifetime), task checks, cleanup and
  wait dependencies, and project-level taskOptions. Not for creating new tasks, setting up
  workspaces, configuring toolchains, or learning moon concepts.
license: MIT
allowed-tools: Bash(moon:*) Read
compatibility: >-
  Requires moon >= 2.0.0 CLI installed and a configured moon workspace.
metadata:
  moon-version-min: '2.0.0'
  moon-version-tested: '2.6.0'
  category: 'debugging'
  ecosystem: 'moonrepo'
---

# moon task debugger

A workflow-oriented diagnostic skill for troubleshooting moon tasks. This is not a reference manual
— it guides you through a structured debugging flow so you can isolate the problem quickly.

For conceptual background, see the [moon documentation](https://moonrepo.dev/docs).

**Before you start:** Ask the user for the `<project>:<task>` target to debug. If they haven't
provided a specific target, prompt them for it — the diagnostic flow requires a concrete target to
inspect.

---

## Quick-start: 5-step diagnostic flow

Work through these steps in order. Most issues resolve by step 3.

### Step 1: Inspect the resolved task configuration

The first thing to check is whether the task is configured the way the user expects. moon merges
configuration from multiple sources (global tasks, project config, inheritance), so the resolved
result can surprise people.

```bash
# Show the fully resolved task config (with inheritance applied)
moon task <project>:<task>

# Machine-readable version for programmatic inspection
moon task <project>:<task> --json
```

**What to verify:**

- `command` vs `script` — if the command contains pipes (`|`), redirects (`>`), chained commands
  (`&&`), or complex syntax, it must use `script`, not `command`.
- `inputs` — are they too broad (`**/*` captures everything) or too narrow (missing source files)?
  Check `state.defaultInputs` (true = using default `**/*`) and `state.emptyInputs` (true =
  explicitly set to `[]`, or a task of the root-level project in a monorepo that configured no
  inputs). Both keys are omitted from the JSON entirely when false, as is `state.setRunInCi`.
- `outputs` — are they declared for build tasks? Missing outputs means the cache can never hydrate
  artifacts. In v2.3+, outputs also affect the **default `cacheStrategy`** of any task that depends
  on this one (see Step 4).
- `toolchains` — is the correct toolchain(s) assigned? An incorrect toolchain means wrong tool
  versions. <sup>v2.6+</sup> The order is significant: configured toolchains come first, followed by
  related ones (those they require, and enabled toolchains that require them, like `npm` for
  `node`), and the **first** is the primary (used by `$taskToolchain`, and wins when toolchain
  environments are activated and paths are prepended to `PATH`).
- `deps` — are task dependencies correct and complete? In v2.3+, each dep entry can carry a
  `cacheStrategy` (`hash` / `ignored` / `outputs`) that controls whether the dep contributes to this
  task's cache hash. If omitted, the default depends on whether the dep declares outputs. In v2.6+,
  each dep also has a `type`: `required` (default, runs before and must pass), `cleanup` (runs
  _after_ the task, even when it fails), or `wait` (the task only waits for it to _start_). The
  human-readable `moon task` output labels the non-required ones, like `db:stop (cleanup)`.
- `options` — check `persistent`, `runInCI`, `cache`, `affectedFiles`, `mutex`, `timeout`,
  `retryCount`, `allowFailure`, `expectFailure`, `envOverride`, and `os`.
- `env` — in v2.5+, environment variables can also be inherited from a **workspace-level `env`** in
  `.moon/tasks/**/*` (merged into the project's `env`, project wins), and the project can change the
  merge behavior via `workspace.mergeStrategies.env`. A variable with a surprising value may come
  from a layer outside the task. In v2.6+, toolchains that have been setup can also **activate their
  environment** (like `JAVA_HOME`), which sets variables the task config never mentions — but never
  overrides variables the task already configures. Also note that a task's `env` value only applies
  when the variable **isn't already set in the shell** — an exported shell value wins at runtime.
- `checks` <sup>v2.4+</sup> — shell scripts that run **before** the task. Their type determines the
  outcome: a `requirement` failing makes the task **fail**, all `condition` checks passing makes the
  task **skip**, and a `fingerprint` folds script output into the task hash. A surprising fail,
  skip, or cache invalidation often traces back to a check.
- `tags` <sup>v2.3+</sup> — labels for grouping tasks. Affects targets like `:#quality` and MQL
  `taskTag` queries. If a task isn't matched by a `#tag` target you expected, check this list.
- `type` — an explicit `type` wins; otherwise `build` (has outputs), `run` (has a `preset`, or is
  persistent), else `test`. A persistent task with outputs is `build`.
- `preset` — `server` or `utility` apply multiple option defaults at once.

**Red flags:**

- `command: 'eslint . && prettier --check .'` — shell syntax in `command` is a task build error in
  v2 (`task_builder::unsupported_command_syntax`). Use `script` instead.
- Empty `outputs` on a build task — cache will never restore artifacts.
- `inputs: ['**/*']` — too broad, cache invalidates on every change.
- A non-persistent task with a `required` dep on a `persistent` task — a hard error when tasks are
  built. In v2.6+, use `type: 'wait'` to only wait for it to start — and since `wait` deps are still
  subject to the `runInCI` check (persistent and `server` preset tasks default to `runInCI: false`),
  also set `runInCI: true` on the persistent dep (or `'skip'` on it, or `false` on the dependent),
  otherwise it fails with `run_in_ci_mismatch`.
- A `wait` dep <sup>v2.6+</sup> with no `cleanup` dep that stops it — a waited-on task is never
  stopped for you, so a non-persistent one keeps the pipeline running until it exits, and a
  persistent one keeps it running until moon is exited.
- A `wait` dep on a task that has caching enabled <sup>v2.6+</sup> — waited-on deps are started
  before the cache is checked, so on a cache hit the server starts and is then immediately stopped
  by its cleanup. Disable `cache` on tasks that wait on long-running deps.
- `command: 'noop'` or `nop` / `no-op` — the task is intentionally a no-op and does nothing. moon
  treats these specially.
- `runInCI: 'only'` — task runs in CI but NOT locally (common surprise).
- `runInCI: 'skip'` — task is skipped in CI but relationships remain valid.
- `os` set to a platform the user isn't on — the task is rewritten to a passing no-op at build time
  (`moon task --json` shows `command: noop` with cleared args/outputs).
- `allowFailure: true` — the failure is still recorded and displayed, but the pipeline continues and
  moon exits successfully, so it's easy to miss.
- `expectFailure: true` <sup>v2.6+</sup> — the task passes when its command fails, and fails with
  "was expected to fail, but it passed" once the underlying problem is fixed (remove the setting).
  It's never cached, and exit codes 126/127 or a signal don't count as the expected failure.
- A `condition` check present <sup>v2.4+</sup> — the task will **skip** whenever all conditions
  pass. A task that "never runs" may have a condition that always passes.
- A `fingerprint` check present <sup>v2.4+</sup> — its script output is hashed, so volatile output
  (timestamps, versions) causes cache misses on every run.

### Step 2: Run with maximum verbosity

If the config looks right, run the task with debug logging to see what moon is actually doing under
the hood.

```bash
# Debug-level logging with cache bypass (note: --force also skips affected checks)
moon run <project>:<task> --log debug --force

# Deep debugging: reveal env vars and stdin passed to the process
MOON_DEBUG_PROCESS_ENV=true MOON_DEBUG_PROCESS_INPUT=true moon run <project>:<task> --log trace --force
```

**What to look for in the logs:**

- Toolchain resolution — is the right version of node/deno/bun/etc being used?
- Hash generation — what sources are being hashed?
- Affected status — is the task being skipped because it's "not affected"? `--force` runs tasks even
  when they're not affected, so drop it (and pass `--affected`) when diagnosing this.
- Process execution — what command is actually being spawned?

**Visualize the execution graph** to spot dependency issues:

```bash
moon action-graph <project>:<task> --dot   # DOT format
moon action-graph <project>:<task> --json  # JSON format
```

> Always pass `--dot` or `--json` as an agent. Without them, the command starts a local web server
> and opens a browser, and blocks until it's interrupted.

> For all graph commands and output formats, see `references/environment-debug.md`.

### Step 3: Inspect cache state

If the task runs but produces wrong results, or runs when it shouldn't, or doesn't run when it
should, the cache is the likely culprit.

```bash
# Inspect a hash manifest to see what inputs were hashed
moon hash <hash>

# Compare two hashes to see what changed between runs
moon hash <hash1> <hash2>

# Short-form hashes work too
moon hash 0b55b234 2388552f
```

<sup>v2.6+</sup> Hash manifests are stored as blobs in the local content-addressable cache
(`.moon/cache/blobs/`), not as `.moon/cache/hashes/<hash>.json` files, so always read them through
`moon hash`. If it reports "Unable to find a hash manifest", the manifest may have been garbage
collected — re-run the task with `--force` to regenerate it.

> For cache file locations, hash interpretation, and the `--force` vs `--cache off` comparison, see
> `references/cache-issues.md`.

### Step 4: Diagnose the problem type

Use this table to jump to the right reference:

| Symptom                                                                            | Likely cause                                                                                                                                                                                                                                                                          | Quick check                                                                                      | Reference                         |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | --------------------------------- |
| Task doesn't exist                                                                 | Inheritance not applied — check `inheritedBy` conditions in `.moon/tasks/**/*` against the project's `toolchains`, `config.language`, `config.stack`, `config.layer`, `config.tags` via `moon project <name> --json` (a detected-only language doesn't match `inheritedBy.languages`) | `moon task <target> --json`                                                                      | `references/config-mistakes.md`   |
| "Nothing to do"                                                                    | `--affected` + no changes, `runInCI: false`, or `inheritedBy` mismatch (global task not inherited)                                                                                                                                                                                    | Check flags, `options.runInCI`, and `inheritedBy`                                                | `references/decision-tree.md`     |
| `--affected` misses changed files (or runs everything)                             | Shallow git clone in CI — with no base, moon warns "Detected a shallow checkout…" and disables affected (so everything runs); with an explicit base, diffs may be inaccurate                                                                                                          | Check clone depth; use full history (`fetch-depth: 0`)                                           | `references/decision-tree.md`     |
| Task fails: "…is unable to run as the requirement check … failed" <sup>v2.4+</sup> | A `requirement` check script exited non-zero (`task_runner::requirement_check_failed`); a failing `fingerprint` script also fails the task (`task_runner::hash_check_failed`)                                                                                                         | `moon task <target> --json` — inspect `checks`                                                   | `references/config-mistakes.md`   |
| Task skipped, not affected/CI-related <sup>v2.4+</sup>                             | All `condition` checks passed, so the task was intentionally skipped                                                                                                                                                                                                                  | `moon run <target> --log debug` — look for "Skipping task as all conditional checks have passed" | `references/config-mistakes.md`   |
| Task errors on execution                                                           | Wrong `command`/`script`, bad toolchain                                                                                                                                                                                                                                               | `moon run <target> --log debug`                                                                  | `references/config-mistakes.md`   |
| Stale cache (cached when it shouldn't be)                                          | Inputs too narrow, missing `env` vars, or dep `cacheStrategy: 'ignored'` (the v2.3 default for output-less deps)                                                                                                                                                                      | `moon hash <hash>`                                                                               | `references/cache-issues.md`      |
| Cache miss (re-runs every time)                                                    | Inputs too broad, volatile outputs, or dep `cacheStrategy: 'hash'` propagating upstream churn                                                                                                                                                                                         | `moon hash <h1> <h2>`                                                                            | `references/cache-issues.md`      |
| Cache miss from a `fingerprint` check <sup>v2.4+</sup>                             | A `fingerprint` check's script output is volatile (timestamps, PIDs), changing the hash every run                                                                                                                                                                                     | `moon hash <h1> <h2>` — look for the check hash                                                  | `references/cache-issues.md`      |
| Outputs not restored after cache hit                                               | `outputs` misconfigured; or <sup>v2.5+</sup> a daemon-side storage failure (only with the daemon **and** `casOutputsCache` or a remote cache) — logged only by the daemon, and a failed hydrate becomes a silent cache miss                                                           | Check `.moon/cache/outputs/`; `moon daemon logs`                                                 | `references/cache-issues.md`      |
| Env var has unexpected value                                                       | <sup>v2.5+</sup> Workspace-level `env` in `.moon/tasks/**/*` merged in, or `workspace.mergeStrategies.env` changed the merge behavior; or the variable is already set in the shell (a task's `env` only applies when it isn't)                                                        | `moon task <target> --json` — inspect `env`; `env \| grep <VAR>`                                 | `references/config-mistakes.md`   |
| Cache behaves differently across git worktrees <sup>v2.5+</sup>                    | `cache.sharedWorktreeCache` (alias `unstable_sharedWorktreeCache`) shares blobs/manifests via the base checkout's `.moon/cache`                                                                                                                                                       | Check the setting (both names) and `MOON_CACHE_SHARED_WORKTREE_CACHE`                            | `references/cache-issues.md`      |
| New dependency cycle error after upgrading to v2.5                                 | Async graph building (now default) validates cycles strictly, per dependency-scope partition                                                                                                                                                                                          | Set `experiments.asyncGraphBuilding: false` to confirm                                           | `references/decision-tree.md`     |
| Build re-runs on every upstream input change <sup>v2.3+</sup>                      | Dep using default `cacheStrategy: 'hash'` instead of `'outputs'`                                                                                                                                                                                                                      | `moon task <target> --json` — inspect dep entries                                                | `references/cache-issues.md`      |
| Task not matched by `#tag` target <sup>v2.3+</sup>                                 | Missing `tags` on the task, or `mergeTags` dropped them during inheritance                                                                                                                                                                                                            | `moon task <target> --json` — check `tags`                                                       | `references/config-mistakes.md`   |
| Task hangs / pipeline stuck                                                        | <sup>v2.6+</sup> A `wait` dep that is never stopped by a `cleanup` dep, a persistent task that was requested (it runs until interrupted, by design), or a persistent/`wait` task holding a `mutex` another task needs                                                                 | `moon action-graph <target> --dot`; `moon task <target>` for dep types                           | `references/config-mistakes.md`   |
| Cleanup dep didn't run <sup>v2.6+</sup>                                            | The task was hydrated from the cache or skipped (nothing to clean up), the pipeline was interrupted by a signal (Ctrl+C), or the cleanup has `runInCI` disabled                                                                                                                       | `moon run <target> --log debug` — look for "nothing to clean up"                                 | `references/config-mistakes.md`   |
| Task can't reach a `wait` dep (connection refused) <sup>v2.6+</sup>                | `wait` only waits for the dep to _start_, not to be _ready_                                                                                                                                                                                                                           | Poll a health check before sending requests                                                      | `references/config-mistakes.md`   |
| Persistent task doesn't run in CI <sup>v2.6+</sup>                                 | Persistent tasks default to `runInCI: false`; when enabled, they only run for the tasks that depend on them, or when explicitly targeted                                                                                                                                              | `moon task <target> --json` — inspect `options.runInCI`                                          | `references/config-mistakes.md`   |
| Output not displayed (or displayed when it shouldn't be)                           | `outputStyle` applies to transitive targets only; <sup>v2.4.3+</sup> primary targets always display output (except v2.5.2–v2.5.x in CI or on cache hits), unless <sup>v2.6+</sup> `explicitTaskOutputStyle` is enabled or `--output-style` is passed                                  | Check `options.outputStyle` and the command line                                                 | `references/config-mistakes.md`   |
| Tasks re-run once after upgrading to v2.6                                          | A task's `toolchains` list (part of its hash) is now ordered by configuration instead of hash order, which invalidates the cache once for tasks with multiple resolved toolchains                                                                                                     | Expected; the second run should hit the cache                                                    | `references/cache-issues.md`      |
| `moon hash` can't find a manifest <sup>v2.6+</sup>                                 | Hash manifests are blobs in the local CAS, and may have been garbage collected                                                                                                                                                                                                        | `moon run <target> --force`, then `moon hash` the new hash                                       | `references/cache-issues.md`      |
| Env var set that the config doesn't mention <sup>v2.6+</sup>                       | A toolchain activated its environment (like `JAVA_HOME`)                                                                                                                                                                                                                              | `MOON_DEBUG_PROCESS_ENV=true moon run <target> --log debug`                                      | `references/environment-debug.md` |
| Task is slow                                                                       | Dep chain bottleneck, no parallelism                                                                                                                                                                                                                                                  | `moon action-graph <target> --dot`                                                               | `references/decision-tree.md`     |
| Task does nothing (no-op)                                                          | Command is `noop`/`nop`/`no-op`                                                                                                                                                                                                                                                       | `moon task <target> --json`                                                                      | `references/config-mistakes.md`   |
| Task "fails silently"                                                              | `allowFailure: true` — the task is still marked failed, but the pipeline continues and the exit code ignores it                                                                                                                                                                       | Check `options.allowFailure`                                                                     | `references/config-mistakes.md`   |
| Task skipped locally                                                               | `runInCI: 'only'` set                                                                                                                                                                                                                                                                 | Check `options.runInCI`                                                                          | `references/config-mistakes.md`   |
| Task skipped in CI                                                                 | `runInCI: false` or `'skip'`                                                                                                                                                                                                                                                          | Check `options.runInCI`                                                                          | `references/config-mistakes.md`   |
| Mutex contention / hang                                                            | Tasks share the same `mutex`; a persistent or `wait` task holding it never releases it                                                                                                                                                                                                | Check `options.mutex`                                                                            | `references/config-mistakes.md`   |
| Task times out                                                                     | `timeout` option set too low                                                                                                                                                                                                                                                          | Check `options.timeout`                                                                          | `references/config-mistakes.md`   |

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

- `--force` ignores existing cache but **writes** new cache after execution — and also runs tasks
  that aren't affected (it skips affected checks).
- `--cache off` disables caching entirely — no reads, no writes.

> For all cache modes, see `references/cache-issues.md`.

---

## Common mistakes at a glance

These are the issues that come up most often. For details and fixes, see
`references/config-mistakes.md`.

- **Shell syntax in `command`** — pipes, `&&`, redirects require `script`; v2 rejects these as parse
  errors.
- **Missing `outputs` on build tasks** — cache can never hydrate artifacts.
- **Overly broad `inputs`** — `**/*` invalidates cache on every change; be specific.
- **Volatile outputs** — timestamps or absolute paths in build artifacts cause permanent cache
  misses.
- **Persistent task in `deps`** — a `required` dep on a persistent task is a hard error; tasks named
  `dev`/`start`/`serve` auto-get the `server` preset. In v2.6+, depend on it with `type: 'wait'`,
  enable `runInCI` on it (it defaults to off, and `wait` deps are still CI-checked), and pair it
  with a `cleanup` dep that stops it.
- **`--affected` vs `--force` confusion** — `--affected` restricts which tasks run; `--force`
  bypasses the cache **and** skips affected checks, so `--force` overrides `--affected` (tasks run
  even when not affected).
- **`allowFailure: true` hiding errors** — the failure is still recorded and displayed, but the
  pipeline continues and moon exits successfully; check stderr at
  `.moon/cache/states/<project>/<task>/stderr.log`.
- **`mutex` contention** — shared mutex serializes tasks; a persistent (or `wait`) task holding a
  mutex never releases it, so other tasks with that mutex hang.
- **`runInCI: 'only'`** — task silently skips when run locally (most surprising variant).
- **Missing outputs flip dep `cacheStrategy`** <sup>v2.3+</sup> — a dep without `outputs` now
  defaults to `cacheStrategy: 'ignored'`. Downstream tasks stop invalidating on its changes; set
  `cacheStrategy: 'hash'` explicitly to restore the pre-v2.3 default.
- **MQL tag fields on task queries** — the query is a positional argument
  (`moon query tasks "taskTag=quality"`; there's no `--query` flag on `moon query`). In v2.3–v2.4,
  `taskTag=` silently matched _nothing_ on task queries (this also broke task tag glob targets like
  `:#tag-*`); v2.5 fixed `taskTag`. `projectTag=`/`tag=` (and other project fields, except
  `project=`) still match **nothing** on task queries — query projects instead. On older versions,
  filter task tags with the `--tags` flag (`moon query tasks --tags quality`).
- **A `checks` script silently changes task behavior** <sup>v2.4+</sup> — a `requirement` failing
  aborts the task, a passing `condition` skips it, and a `fingerprint` mixes script output into the
  hash. Inspect `checks` in `moon task <target> --json` when a task fails, skips, or re-runs for no
  obvious reason.
- **Shallow git clone breaks `--affected`** — with a shallow clone (depth 1) and no explicit base,
  moon warns "Detected a shallow checkout…" and disables affected filtering for `moon run`/`ci`/
  `exec`, so **everything runs** (`moon query … --affected` gets an empty list instead). With an
  explicit base (`--base`, `MOON_BASE`, or a detected PR base), it diffs anyway and the result may
  be inaccurate. Use a full clone (`fetch-depth: 0`), or a blobless partial clone with full history
  (`git clone --filter=blob:none`).
- **Experiments are now on by default** <sup>v2.5+</sup> — `asyncGraphBuilding`,
  `asyncAffectedTracking`, and `nativeFileHashing` default to enabled. When bisecting graph,
  affected, or hashing oddities, disable the relevant experiment (config or
  `MOON_EXPERIMENT_*=false`) and compare — but also check the user's shell/CI for
  `MOON_EXPERIMENT_*` or `MOON_CACHE_*` overrides that silently change behavior.
- **Daemon archiving/hydration failures are invisible in the main process** <sup>v2.5+</sup> — when
  the daemon is enabled **and** outputs go to the local CAS (`casOutputsCache`) or a remote cache,
  they're archived and hydrated through the daemon, and storage failures only appear in
  `moon daemon logs` (default `.tar.gz` archives are always handled in-process). To rule the daemon
  out, re-run with `MOON_DAEMON=false`.
- **Workspace-level `env` is a new inheritance layer** <sup>v2.5+</sup> — `.moon/tasks/**/*` files
  can define `env` inherited by all matching projects. Project values win on conflict, unless
  `workspace.mergeStrategies.env` says otherwise (`append`, `prepend`, `preserve`, `replace`).
- **Expecting a `cleanup` dep to always run** <sup>v2.6+</sup> — cleanups are skipped when there's
  nothing to clean up (the task was hydrated from the cache or skipped, and didn't start a `wait`
  dep), and they never run when the pipeline is interrupted by a signal like Ctrl+C.
- **`cleanup` and `wait` deps don't affect the cache** <sup>v2.6+</sup> — they never contribute to
  the task's hash (their `cacheStrategy` can only be `ignored`), and they always run their command
  instead of being hydrated.
- **Persistent tasks run within the pipeline** <sup>v2.6+</sup> — they no longer wait until every
  other task has finished; they start as soon as their own deps complete. In CI, they're skipped
  unless `runInCI` is explicitly enabled, and even then only run for their dependents or when
  explicitly targeted. `moon check` skips them unless another task depends on them.

---

## When to load references

Each reference file covers a specific problem domain in depth. Load them only when the diagnostic
flow points you there — don't load everything upfront.

| Reference                         | When to load                                                                                                                                                                                                                 |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `references/decision-tree.md`     | When the symptom doesn't match the quick table above, or you need a systematic walk-through of all possibilities.                                                                                                            |
| `references/cache-issues.md`      | When the problem is clearly cache-related: unexpected hits, unexpected misses, outputs not restoring.                                                                                                                        |
| `references/config-mistakes.md`   | When the task config is wrong: command vs script, inheritance bugs, presets, persistent tasks, dependency types (`cleanup`/`wait`), affectedFiles, mutex, timeout, retries, runInCI variants, allowFailure, os, outputStyle. |
| `references/environment-debug.md` | When you need to go deeper with env vars, log levels, trace profiles, or inspection tools.                                                                                                                                   |
