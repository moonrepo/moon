# Decision tree: moon task diagnostics

A systematic walk-through for diagnosing any moon task issue. Start at the top and follow the
branches. Each leaf gives you the exact commands to run and what to fix.

---

## Does the task exist?

```bash
moon task <project>:<task> --json
```

If this returns an error or exit code 1, the task does not exist in that project.

### NO — task not found

**Check 1: Is it defined in the project config?**

Look in the project's config file (`moon.{json,jsonc,hcl,pkl,toml,yaml,yml}`) under `tasks:`.

```bash
# Read the project's config file directly
cat <project-root>/moon.yml  # or moon.json, moon.toml, etc.
```

**Check 2: Should it be inherited from global tasks?**

Global tasks live in `.moon/tasks/**/*`. Inheritance depends on `inheritedBy` conditions matching
the project's `toolchains`, `stack`, `layer`, `language`, or `tags` (project tags — distinct from
the per-task `tags` introduced in v2.3).

```bash
# See what the project's metadata looks like
moon project <project> --json
```

Compare the project against the `inheritedBy` conditions in the global task file.

Common inheritance failures:

- Project doesn't have the right `toolchains` set (e.g., global task requires `toolchain: 'node'`
  but project doesn't declare it)
- `inheritedBy` uses `and` clause that the project doesn't fully satisfy
- Project explicitly excludes the task via `workspace.inheritedTasks.exclude`
- Project renames the task via `workspace.inheritedTasks.rename`
- Project doesn't `include` the global task file (check for `include` directives)
- <sup>v2.3+</sup> The user expected `:#tag` to match the task, but the task's own `tags` field is
  missing the tag — or `options.mergeTags: 'replace'` dropped inherited tags. See
  `config-mistakes.md` § Task tags and `#tag` targets.

**Check 3: Is the task ID spelled correctly?**

Task IDs support unicode alphanumeric characters plus `/`, `.`, `_`, and `-`, with an optional
leading `@` (see `ID_PATTERN` in the `starbase_id` crate for the full regex). Check for typos,
especially with similar names (e.g., `build` vs `buildApp`).

**Fix:** Add the task to the project config, fix the `inheritedBy` conditions, or correct the task
ID.

---

## Does the task execute?

```bash
moon run <project>:<task> --log debug --force
```

### NO — "nothing to do" or skipped

**Check 1: Was `--affected` used?**

`--affected` restricts execution to tasks whose inputs changed since the base branch. If no inputs
changed, the task is skipped entirely.

```bash
# Run without --affected to confirm
moon run <project>:<task> --force
```

<sup>v2.4+</sup> If `--affected` runs _nothing_ (or the wrong set) in CI, suspect a **shallow git
clone**. moon reworked merge-base resolution in v2.4 and now needs full history to diff revisions
accurately — with a depth-1 clone it can't resolve the merge base, and logs a warning when it fails.
Use a full clone, or a blobless partial clone (`git clone --filter=blob:none`, or
`filter: 'blob:none'` with `actions/checkout`) which keeps history while deferring file downloads.

**Check 2: Is `runInCI` blocking execution?**

```bash
moon task <project>:<task> --json
# Check options.runInCI and state.setRunInCi
```

If `state.setRunInCi` is `true`, `runInCI` was set explicitly in config or applied by a preset. If
`false`, the value defaulted from the task type (build/test tasks run in CI, others don't). See
`config-mistakes.md` § `runInCI` variants for the full table of values and their local/CI behavior.

**Check 3: Is the `os` option filtering this platform out?**

If `options.os` is set and doesn't match the current platform, the task is silently skipped.

```bash
moon task <project>:<task> --json
# Check options.os — values: 'linux', 'macos', 'windows'
```

**Check 4: Is the task a no-op?**

Tasks with command `noop`, `nop`, or `no-op` intentionally do nothing. moon recognizes these as
special no-operation commands.

```bash
moon task <project>:<task> --json
# If command is "noop"/"nop"/"no-op", the task is intentionally empty
```

**Check 5: Are there input changes?**

The task may be legitimately cached. Check if the hash matches a previous run:

```bash
# Look at the last run info
cat .moon/cache/states/<project>/<task>/lastRun.json
```

**Check 6: Did a `condition` check skip the task?** <sup>v2.4+</sup>

If the task has one or more `condition` checks and **all** of them pass, moon intentionally skips
the task (the target ends in a `Skipped` / `SkippedConditional` state). This is the inverse of a
requirement — passing conditions mean "already done, don't run."

```bash
moon task <project>:<task> --json          # inspect the `checks` array
moon run <project>:<task> --log debug --force 2>&1 | grep -i "condition"
# "Skipping task as all conditional checks have passed"  → skipped by condition
```

See `config-mistakes.md` § Task checks.

**Fix:** Remove `--affected` if you want to force execution. Set `runInCI: 'always'` if the task
must always run in CI. Remove or change the `os` option if platform filtering is unwanted. Use
`--force` to bypass the cache. If a `condition` check is skipping the task, adjust or remove the
condition so it no longer passes.

### NO — execution error

**Check 1: Is the command valid?**

```bash
moon task <project>:<task> --json
```

Look at the `command` and `args` fields. Is the binary installed and on PATH? Is the toolchain set
up correctly?

**Check 2: Is `command` used where `script` is needed?**

If the command contains `&&`, `|`, `>`, or other shell syntax, it must use `script` instead of
`command`. The `command` field only accepts a single binary.

**Check 3: Is the working directory correct?**

By default, tasks run from the project root. If `options.runFromWorkspaceRoot` is `true`, it runs
from the workspace root instead. Verify file paths in the command are relative to the correct
directory.

**Check 4: Is the toolchain providing the right version?**

```bash
moon run <project>:<task> --log debug --force 2>&1 | grep -i "toolchain\|version\|resolv"
```

**Check 5: Are environment variables correct?**

Use `MOON_DEBUG_PROCESS_ENV=true` to reveal all env vars passed to the process. See
`environment-debug.md` for all debug env vars and log levels.

**Check 6: Is `timeout` or `allowFailure` involved?**

Check `options.timeout` and `options.allowFailure` in the JSON output. If `allowFailure: true`, the
task reports success even on failure — check stderr at
`.moon/cache/states/<project>/<task>/stderr.log`. See `config-mistakes.md` for details on both
options.

**Check 7: Did a `requirement` or `fingerprint` check fail?** <sup>v2.4+</sup>

`checks` run **before** the task. A `requirement` check that exits non-zero raises
`RequirementCheckFailed` ("Task X is unable to run as the requirement check `<script>` failed"), and
a `fingerprint` script that crashes raises `FingerprintCheckFailed` — in both cases the task never
executes.

```bash
moon task <project>:<task> --json          # inspect the `checks` array
# Run the failing check script by hand to see why it exits non-zero
```

See `config-mistakes.md` § Task checks.

**Fix:** Switch `command` to `script` for shell syntax. Fix the binary path or toolchain. Correct
the working directory. If a check is failing, fix (or remove) the offending check script. See
`config-mistakes.md` for the full `command` vs `script` guide.

---

## Does the task produce correct results?

### NO — stale or incorrect output (cache problem)

**The output is from a previous run (stale cache):**

The task's `inputs` don't cover all files that affect the output. A source file changed but wasn't
in `inputs`, so the hash didn't change, and moon served the old cached output.

```bash
# Inspect what was hashed
moon hash <hash>

# Force a fresh run to confirm
moon run <project>:<task> --force
```

> See `cache-issues.md` for detailed cache diagnosis.

**The dep that changed has `cacheStrategy: 'ignored'`** <sup>v2.3+</sup>:

In v2.3, deps without outputs default to `cacheStrategy: 'ignored'`, so changes to them don't
invalidate this task. If you expected an upstream lint/test change to bust the cache, this is why.
See `cache-issues.md` § Dependency cache strategies.

**The output is missing files:**

The task's `outputs` don't cover all files the build produces. moon only archives and restores
what's declared in `outputs`.

```bash
moon task <project>:<task> --json
# Check outputFiles and outputGlobs
```

**Fix:** Expand `inputs` to cover all source files. Expand `outputs` to cover all build artifacts.
See `cache-issues.md` for the full diagnosis flow.

### NO — wrong configuration applied

The task config doesn't match what you expect because of inheritance.

```bash
# See the fully resolved config
moon task <project>:<task> --json

# See which config files contributed
cat .moon/cache/states/<project>/snapshot.json
```

The snapshot's `inherited.layers` field shows which global config files were merged for each task.
Check merge strategies (`mergeArgs`, `mergeDeps`, `mergeEnv`, `mergeInputs`, `mergeOutputs`,
`mergeTags` <sup>v2.3+</sup>, `mergeToolchains`) — the default for args is `append`.

> See `config-mistakes.md` for common inheritance bugs.

---

## Is the task slow?

### Check 1: Dependency chain bottleneck

```bash
moon action-graph <project>:<task>
```

Visualize the graph and look for:

- Long serial chains where tasks could run in parallel.
- Tasks that don't need to depend on each other but do.
- A persistent task in the dependency chain (it never "finishes," blocking everything downstream).

### Check 2: Cache, inputs, mutex, or retries

```bash
moon task <project>:<task> --json
```

- **No cache?** Check `options.cache` — may be `false` or set by preset. See `cache-issues.md` for
  cache miss diagnosis.
- **Broad inputs?** Check `state.defaultInputs` — if `true`, the default `**/*` glob is hashing the
  entire project directory.
- **Mutex?** Check `options.mutex` — shared mutex serializes tasks. See `config-mistakes.md` §
  `mutex` contention.
- **Retries?** Check `options.retryCount` — flaky tasks with retries multiply execution time.

**Fix:** Remove unnecessary `deps`, narrow `inputs`, ensure caching is enabled, review mutex usage,
and investigate flaky tasks.

---

## Trace profiling for deep performance issues

For performance issues that aren't obvious from the action graph:

```bash
# Generate a trace profile
moon run <project>:<task> --dump --force

# This creates a .json trace file in the working directory
# Open it in Chrome DevTools: chrome://tracing
```

The trace shows exactly where time is spent — toolchain setup, dependency installation, hash
generation, process execution — with microsecond precision.
