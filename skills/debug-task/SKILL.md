---
name: debug-task
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

# moon task debugger

A workflow-oriented diagnostic skill for troubleshooting moon tasks. This is not
a reference manual — it guides you through a structured debugging flow so you
can isolate the problem quickly.

For conceptual background, see the [moon documentation](https://moonrepo.dev/docs).

**Before you start:** Ask the user for the `<project>:<task>` target to debug.
If they haven't provided a specific target, prompt them for it — the diagnostic
flow requires a concrete target to inspect.

---

## Quick-start: 5-step diagnostic flow

Work through these steps in order. Most issues resolve by step 3.

### Step 1: Inspect the resolved task configuration

The first thing to check is whether the task is configured the way the user
expects. moon merges configuration from multiple sources (global tasks, project
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
  (missing source files)? Check `state.defaultInputs` (true = using default
  `**/*`) and `state.emptyInputs` (true = explicitly set to `[]`)
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
- `command: 'eslint . && prettier --check .'` — shell syntax in `command` is a
  parse error in v2. Use `script` instead
- Empty `outputs` on a build task — cache will never restore artifacts
- `inputs: ['**/*']` — too broad, cache invalidates on every change
- A `persistent` task in a `deps` chain — moon produces a hard error
  (`PersistentDepRequirement`) at build time
- `command: 'noop'` or `nop` / `no-op` — the task is intentionally a no-op and
  does nothing. moon treats these specially
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
moon action-graph <project>:<task>
moon action-graph <project>:<task> --dot  # DOT format (useful for agents)
```

> For all graph commands and output formats, see `references/environment-debug.md`.

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

> For cache file locations, hash interpretation, and the `--force` vs `--cache off`
> comparison, see `references/cache-issues.md`.

### Step 4: Diagnose the problem type

Use this table to jump to the right reference:

| Symptom | Likely cause | Quick check | Reference |
|---------|-------------|-------------|-----------|
| Task doesn't exist | Inheritance not applied — check `inheritedBy` conditions in `.moon/tasks/**/*` against project's `toolchains`, `stack`, `layer`, `tags` via `moon project <name> --json` | `moon task <target> --json` | `references/config-mistakes.md` |
| "Nothing to do" | `--affected` + no changes, `runInCI: false`, or `inheritedBy` mismatch (global task not inherited) | Check flags, `options.runInCI`, and `inheritedBy` | `references/decision-tree.md` |
| Task errors on execution | Wrong `command`/`script`, bad toolchain | `moon run <target> --log debug` | `references/config-mistakes.md` |
| Stale cache (cached when it shouldn't be) | Inputs too narrow, missing `env` vars | `moon hash <hash>` | `references/cache-issues.md` |
| Cache miss (re-runs every time) | Inputs too broad, volatile outputs | `moon hash <h1> <h2>` | `references/cache-issues.md` |
| Outputs not restored after cache hit | `outputs` misconfigured | Check `.moon/cache/outputs/` | `references/cache-issues.md` |
| Task hangs / pipeline stuck | Persistent task in `deps` chain (hard error in v2) | `moon action-graph <target>` | `references/config-mistakes.md` |
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

> For all cache modes (`read`, `write`, `off`), see `references/cache-issues.md`.

---

## Common mistakes at a glance

These are the issues that come up most often. For details and fixes, see
`references/config-mistakes.md`.

- **Shell syntax in `command`** — pipes, `&&`, redirects require `script`; v2 rejects these as parse errors
- **Missing `outputs` on build tasks** — cache can never hydrate artifacts
- **Overly broad `inputs`** — `**/*` invalidates cache on every change; be specific
- **Volatile outputs** — timestamps or absolute paths in build artifacts cause permanent cache misses
- **Persistent task in `deps`** — hard error (`PersistentDepRequirement`); tasks named `dev`/`start`/`serve` auto-get `server` preset
- **`--affected` vs `--force` confusion** — `--affected` restricts; `--force` bypasses cache (they're opposites)
- **`allowFailure: true` hiding errors** — task reports success even when command fails; check stderr at `.moon/cache/states/<project>/<task>/stderr.log`
- **`mutex` contention** — shared mutex serializes tasks; combined with deps can deadlock
- **`runInCI: 'only'`** — task silently skips when run locally (most surprising variant)

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
