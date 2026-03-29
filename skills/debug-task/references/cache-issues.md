# Cache Issues: Diagnosis and Fixes

moon's cache is powered by content-based hashing. Every task run generates a
hash from multiple sources (command, args, inputs, outputs, env, dependencies).
If the hash matches a previous run, moon skips execution and restores the cached
output.

When the cache behaves unexpectedly, it's almost always because the inputs to
the hash don't match what you think they should.

---

## Table of contents

1. [Unexpected cache hit](#unexpected-cache-hit) — task uses stale results
2. [Unexpected cache miss](#unexpected-cache-miss) — task re-runs every time
3. [Outputs not restored](#outputs-not-restored) — cache hit but files missing
4. [Debugging tools](#debugging-tools) — commands for any cache issue

---

## Unexpected cache hit

**Symptom:** The task returns cached results when it should re-run. A source
file changed, but moon says "cached" and serves old output.

**Root cause:** The changed file isn't covered by the task's `inputs`.

### Diagnosis

```bash
# 1. Get the hash from the last run
cat .moon/cache/states/<project>/<task>/lastRun.json

# 2. Inspect what was included in the hash
moon hash <hash>
```

The hash manifest shows every source that contributed to the hash. If the file
you changed isn't listed, it's not in `inputs`.

### Common causes

**Inputs don't cover all relevant files:**

```yaml
# PROBLEM: only src/ is covered, but tests import from shared/
inputs:
  - 'src/**/*'

# FIX: add the missing directory
inputs:
  - 'src/**/*'
  - 'shared/**/*'
```

**Undeclared task dependency:**

If task A depends on the output of task B, but `deps` doesn't include B, then
B's outputs won't be factored into A's hash.

```yaml
# FIX: declare the dependency
tasks:
  build-app:
    command: 'vite build'
    deps:
      - 'shared-lib:build'
```

**Environment variable not included:**

If the task's behavior changes based on an env var (like `NODE_ENV`), but that
var isn't declared in the task's `env` config, it won't affect the hash.

```yaml
# FIX: declare env vars that affect the output
tasks:
  build:
    command: 'vite build'
    env:
      NODE_ENV: 'production'
```

Alternatively, you can track an env var in `inputs` using the `$` prefix:

```yaml
inputs:
  - 'src/**/*'
  - '$NODE_ENV'  # env var value is included in the hash
```

### Quick fix

```bash
# Force a fresh run to confirm the problem is cache-related
moon run <project>:<task> --force

# If --force produces the correct output, the cache is stale
# Expand inputs to cover the missing files
```

---

## Unexpected cache miss

**Symptom:** The task re-runs from scratch every time, even though nothing
meaningful changed. You never see "cached" in the output.

**Root cause:** Something in the hash changes on every run — either the inputs
are too broad, or the outputs include volatile files.

### Diagnosis

```bash
# 1. Run the task twice
moon run <project>:<task> --force
moon run <project>:<task>

# 2. Get both hashes
cat .moon/cache/states/<project>/<task>/lastRun.json
# Note: you need hashes from two consecutive runs

# 3. Diff the hashes to see what changed
moon hash <hash1> <hash2>
```

The diff highlights exactly which fields differ between runs. This tells you
what's causing the cache miss.

### Common causes

**Inputs too broad:**

```yaml
# PROBLEM: **/* matches too many irrelevant files in the project directory
# (node_modules and .git are globally ignored, but everything else matches)
inputs:
  - '**/*'

# FIX: be specific
inputs:
  - 'src/**/*'
  - 'package.json'
  - 'tsconfig.json'
```

**Outputs include volatile files:**

Files that change on every build — timestamps in generated files, sourcemaps
with absolute paths, build manifests with dates — cause the hash to differ even
when the source hasn't changed.

**Lockfile changes:**

If `package-lock.json` or `yarn.lock` is in `inputs` (it is by default for
node toolchain tasks), any dependency change invalidates the cache for every
task. This is usually correct behavior, but can be surprising.

**Git-ignored files leaking in:**

moon filters VCS-ignored files from `inputs` (including `node_modules` and
`.git` which are globally ignored), but edge cases exist. If you
see unexpected files in the hash manifest, check `.gitignore` coverage.

### Quick fix

```bash
# Narrow inputs to only the files that matter
# Exclude volatile outputs
# Use moon hash diff to pinpoint the changing field
```

---

## Outputs not restored

**Symptom:** moon says "cached" (cache hit), but the expected output files
don't appear in the project directory.

**Root cause:** The `outputs` configuration doesn't match the actual files the
task produces, or the archive is missing.

### Diagnosis

```bash
# 1. Verify the outputs config
moon task <project>:<task> --json
# Check outputFiles and outputGlobs

# 2. Check if the archive exists
ls .moon/cache/outputs/<hash>.tar.gz

# 3. If the archive exists, inspect its contents
tar tzf .moon/cache/outputs/<hash>.tar.gz
```

### Common causes

**Outputs misconfigured — path relativity:**

Output paths can be project-relative or workspace-relative. By default they
are project-relative, but you can use workspace-relative paths directly in
the `outputs` config. If the build tool writes to an unexpected location,
the paths won't match.

```yaml
# PROBLEM: build writes to <workspace>/dist, not <project>/dist
outputs:
  - 'dist'  # relative to project root

# FIX: adjust the path or the build tool's output directory
```

**Glob outputs + extra files:**

If `outputs` uses a glob like `dist/**/*`, and the build produces files outside
that glob, those files won't be archived. On hydration, only the archived files
are restored.

**Archive doesn't exist:**

If the task has never completed successfully with caching enabled, there's no
archive to restore. This happens when:

- The task was previously run with `--cache off`
- The task errored on the run that generated this hash
- The cache was cleaned (`moon clean`)

### Quick fix

```bash
# 1. Clear cache and force a successful run
moon run <project>:<task> --force

# 2. Verify the archive was created
ls .moon/cache/outputs/

# 3. Run again without --force to test hydration
moon run <project>:<task>

# 4. Check if output files appeared
ls <project>/dist/  # or whatever the output directory is
```

---

## Debugging tools

These commands are useful for any cache investigation:

```bash
# Inspect a hash manifest (all sources that generated the hash)
moon hash <hash>

# Compare two hashes (see exactly what changed)
moon hash <hash1> <hash2>

# JSON output for programmatic analysis
moon hash <hash> --json
moon hash <hash1> <hash2> --json

# See last run metadata (exit code, hash, timing)
cat .moon/cache/states/<project>/<task>/lastRun.json

# See full project snapshot (all resolved tasks and config)
cat .moon/cache/states/<project>/snapshot.json

# List cached output archives
ls .moon/cache/outputs/

# Inspect archive contents
tar tzf .moon/cache/outputs/<hash>.tar.gz

# Force a fresh run (bypasses cache, writes new cache)
moon run <project>:<task> --force

# Disable cache entirely (no reads or writes)
moon run <project>:<task> --cache off

# Other cache modes
moon run <project>:<task> --cache read   # read but don't write
moon run <project>:<task> --cache write  # write but don't read
```

### `--force` vs `--cache off`

These are different:

| Flag | Reads cache | Writes cache | Use when |
|------|------------|-------------|----------|
| `--force` | No | Yes | You want a fresh run but still want to populate the cache |
| `--cache off` | No | No | You want to completely bypass caching (e.g., debugging) |
| `--cache read` | Yes | No | You want to use existing cache but not pollute it |
| `--cache write` | No | Yes | Same as `--force` but more explicit |
| (default) | Yes | Yes | Normal operation |
