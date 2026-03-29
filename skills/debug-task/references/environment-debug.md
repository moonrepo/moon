# Environment and Debug Tools

This reference covers the debug environment variables, log levels, and
inspection tools available for deep debugging of moon tasks.

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

moon provides several environment variables that reveal internal state during
task execution. Set them before running `moon run`:

| Variable | What it reveals |
|----------|----------------|
| `MOON_DEBUG_PROCESS_ENV` | All environment variables passed to the child process. By default moon hides these to avoid leaking secrets. |
| `MOON_DEBUG_PROCESS_INPUT` | Full stdin passed to the child process. By default moon truncates this. |
| `MOON_DEBUG_MCP` | Debug output from MCP server interactions. |
| `MOON_DEBUG_REMOTE` | Debug output from remote caching — connection errors, sync status. |
| `MOON_DEBUG_WASM` | Debug output from WASM plugins — loading, execution, memory profiles. |

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

### Environment variables in task config

Tasks can declare env vars that affect both execution and hashing:

```yaml
tasks:
  build:
    command: 'vite build'
    env:
      NODE_ENV: 'production'
```

Env vars declared in `env` are included in the hash. If you change `NODE_ENV`
from `production` to `development`, the hash changes and the cache misses.

Env vars **not** declared in `env` (but present in the shell) are still passed
to the process, but they don't affect the hash. This means a different
`NODE_ENV` in your shell won't trigger a cache miss unless it's in the config.

---

## Log levels

Control verbosity with the `--log` global option or `MOON_LOG` environment
variable.

| Level | What you see |
|-------|-------------|
| `off` | Nothing |
| `error` | Only errors |
| `warn` | Warnings and above |
| `info` | (default) Status messages, task output |
| `debug` | Internal decisions — hash generation, cache checks, toolchain resolution |
| `trace` | Everything — network requests, child process details, file system operations |
| `verbose` | Like `trace` plus span information (timing, nesting) |

### Recommendations

- **Start with `debug`** for most issues. It shows why moon made each decision
  without drowning you in noise.
- **Escalate to `trace`** only if `debug` doesn't reveal the problem. Trace
  output is voluminous — pipe it to a file.
- **Use `verbose`** for performance profiling. The span information shows exactly
  how long each operation took.

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

Shows the fully resolved task configuration after inheritance, merging, and
token resolution. This is the single most useful debugging command — always
start here.

**Tip:** Running `moon task <project>:<task>` without `--json` also displays
all available `PATH`s for the resolved toolchain.

### `moon project` — inspect project metadata

```bash
# Human-readable output
moon project <project>

# Machine-readable JSON
moon project <project> --json
```

Shows project metadata: language, toolchain, stack, layer, tags, dependencies,
file groups, and all configured tasks.

### `moon task-graph` / `moon project-graph` — visualize graphs

```bash
# Visualize the task dependency graph
moon task-graph <project>:<task>

# Visualize the project dependency graph
moon project-graph <project>
```

These show task-level and project-level dependency relationships respectively,
complementing the lower-level action graph below.

### `moon action-graph` — visualize the dependency graph

```bash
# Open interactive visualization in browser
moon action-graph <project>:<task>

# Focus on a specific target and include its dependents
moon action-graph <project>:<task> --dependents

# Export for external tools
moon action-graph <project>:<task> --dot > graph.dot
moon action-graph <project>:<task> --json > graph.json
```

The action graph shows every action moon will take to run a target: toolchain
setup, dependency installation, project sync, and task execution. It's the
best tool for diagnosing:

- Why a task depends on something unexpected
- Why a task is blocked (look for persistent nodes)
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

> For interpreting hash diffs in cache investigations, see `cache-issues.md`.

### `moon query` — query project and task information

```bash
# Find all projects matching criteria
moon query projects --language typescript
moon query projects --stack frontend

# Find all tasks across projects
moon query tasks
moon query tasks --project <project>
```

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

This is the most granular debugging tool. Use it when you know something is
slow but can't tell what from the logs alone.

---

## Cache file locations

Quick reference for where moon stores internal state:

```
.moon/cache/
  hashes/<hash>.json              # Hash manifest — what was hashed
  outputs/<hash>.tar.gz           # Archived task outputs
  states/<project>/
    snapshot.json                 # Project snapshot (resolved tasks, config)
    <task>/
      lastRun.json                # Last run metadata (exit code, hash)
      stdout.log                  # Captured stdout from last run
      stderr.log                  # Captured stderr from last run
```

All paths are relative to the workspace root. The `.moon/cache/` directory
should be git-ignored.

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
# 1. Visualize the graph
moon action-graph <project>:<task>

# 2. Look for persistent task nodes with dependents
# 3. Restructure deps so persistent tasks are leaf nodes
```
