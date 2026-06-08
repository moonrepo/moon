# Cache issues: Diagnosis and fixes

moon's cache is powered by content-based hashing. Every task run generates a hash from multiple
sources (command, args, inputs, outputs, env, dependencies, etc). If the hash matches a previous
run, moon skips execution and restores the cached output.

When the cache behaves unexpectedly, it's almost always because the inputs to the hash don't match
what you think they should.

---

## Table of contents

1. [Unexpected cache hit](#unexpected-cache-hit) — task uses stale results
2. [Unexpected cache miss](#unexpected-cache-miss) — task re-runs every time
3. [Outputs not restored](#outputs-not-restored) — cache hit but files missing
4. [Dependency cache strategies](#dependency-cache-strategies) — controlling how deps invalidate the
   hash (v2.3+)
5. [Experimental caching layers](#experimental-caching-layers) — native file hashing, local CAS
   (v2.3+)
6. [Debugging tools](#debugging-tools) — commands for any cache issue

---

## Unexpected cache hit

**Symptom:** The task returns cached results when it should re-run. A source file changed, but moon
says "cached" and serves old output.

**Root cause:** The changed file isn't covered by the task's `inputs`.

### Diagnosis

```bash
# 1. Get the hash from the last run
cat .moon/cache/states/<project>/<task>/lastRun.json

# 2. Inspect what was included in the hash
moon hash <hash>
```

The hash manifest shows every source that contributed to the hash. If the file you changed isn't
listed, it's not in `inputs`.

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

If task A depends on the output of task B, but `deps` doesn't include B, then B's outputs won't be
factored into A's hash.

```yaml
# FIX: declare the dependency
tasks:
  build-app:
    command: 'vite build'
    deps:
      - 'shared-lib:build'
```

**Dependency declared but `cacheStrategy` is `'ignored'`** <sup>v2.3+</sup>:

In v2.3 the default `cacheStrategy` for a dep without outputs is `ignored`, meaning the dep's hash
no longer contributes to this task. If you have a build task depending on a `lint` or `test` task
(neither of which declares outputs) and expect the lint/test changes to invalidate the build, the
v2.3 default will **not** invalidate it. To restore the old behavior:

```yaml
tasks:
  build:
    command: 'vite build'
    deps:
      - target: '~:lint'
        cacheStrategy: 'hash' # was the implicit default before v2.3
```

See [Dependency cache strategies](#dependency-cache-strategies) for the full picture.

**Environment variable not included:**

If the task's behavior changes based on an env var (like `NODE_ENV`), but that var isn't declared in
the task's `env` config, it won't affect the hash.

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
  - '$NODE_ENV'
```

### Quick fix

```bash
# Force a fresh run to confirm the problem is cache-related
moon run <project>:<task> --force
```

If `--force` produces the correct output, the cache is stale. Expand inputs to cover the missing
files.

---

## Unexpected cache miss

**Symptom:** The task re-runs from scratch every time, even though nothing meaningful changed. You
never see "cached" in the output.

**Root cause:** Something in the hash changes on every run — either the inputs are too broad, or the
outputs include volatile files.

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

The diff highlights exactly which fields differ between runs. This tells you what's causing the
cache miss.

### Common causes

**Inputs too broad:**

Common folders like `node_modules` and `.git` are globally ignored, but everything else matches.

```yaml
# PROBLEM: **/* matches too many irrelevant files in the project directory
inputs:
  - '**/*'

# FIX: be specific
inputs:
  - 'src/**/*'
  - 'package.json'
  - 'tsconfig.json'
```

**Outputs include volatile files:**

Files that change on every build — timestamps in generated files, sourcemaps with absolute paths,
build manifests with dates — cause the hash to differ even when the source hasn't changed.

**Lockfile changes:**

If `package-lock.json`, `yarn.lock`, etc, is in `inputs`, any dependency change invalidates the
cache for every task. This is usually correct behavior, but can be surprising.

**Git-ignored files leaking in:**

moon filters VCS-ignored files from `inputs` (including `node_modules` and `.git` which are globally
ignored), but edge cases exist. If you see unexpected files in the hash manifest, check `.gitignore`
coverage.

### Quick fix

```bash
# Narrow inputs to only the files that matter
# Exclude volatile outputs
# Use moon hash diff to pinpoint the changing field
```

---

## Outputs not restored

**Symptom:** moon says "cached" (cache hit), but the expected output files don't appear in the
project directory.

**Root cause:** The `outputs` configuration doesn't match the actual files the task produces, or the
archive is missing.

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

> If `experiments.casOutputsCache` is enabled (v2.3+), outputs are stored in a content-addressable
> store rather than per-hash tarballs — see
> [Experimental caching layers](#experimental-caching-layers) for what to look for instead.

### Common causes

**Outputs misconfigured — path relativity:**

Output paths can be project-relative or workspace-relative. By default they are project-relative,
but you can use workspace-relative paths directly in the `outputs` config. If the build tool writes
to an unexpected location, the paths won't match.

```yaml
# PROBLEM: build writes to <workspace>/dist, not <project>/dist
outputs:
  - 'dist' # relative to project root


# FIX: adjust the path or the build tool's output directory
```

**Glob outputs + extra files:**

If `outputs` uses a glob like `dist/**/*`, and the build produces files outside that glob, those
files won't be archived. On hydration, only the archived files are restored.

**Archive doesn't exist:**

If the task has never completed successfully with caching enabled, there's no archive to restore.
This happens when:

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

## Dependency cache strategies

Available in v2.3+.

Each entry in `deps` can declare a `cacheStrategy` that controls how that dep contributes to the
current task's cache hash:

| Strategy    | Effect on this task's hash                                                                                           | Use when                                                       |
| ----------- | -------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `'hash'`    | Mixes in the dep's full hash. Any change to the dep (inputs, command, args, env) invalidates this task.              | You want any upstream change to force a rebuild.               |
| `'ignored'` | Dep is a sequencing edge only; its changes never invalidate this task.                                               | The dep produces no artifact you care about (e.g. lint, test). |
| `'outputs'` | Mixes in the dep's output files instead of its hash. This task is only invalidated when upstream **outputs** change. | Build tasks that consume a dep's artifacts but not its source. |

### The v2.3 default change

When `cacheStrategy` is omitted, the effective default is chosen based on whether the dep declares
outputs:

- Dep **with** outputs → defaults to `'hash'` (same as pre-v2.3).
- Dep **without** outputs → defaults to `'ignored'` (pre-v2.3 was always `'hash'`).

This means tasks that depend on output-less tasks (lint, test, typecheck, etc.) will see **fewer**
cache invalidations after upgrading to v2.3 — which is usually correct, but can surprise you if you
were intentionally relying on lint/test churn to invalidate downstream tasks.

### Diagnosis

```bash
# Inspect the resolved deps and their cacheStrategy
moon task <project>:<task> --json
```

Each entry under `deps` shows its resolved `cacheStrategy`. If you didn't set it, the field reflects
the default chosen for you.

### Common surprises

**A build task no longer rebuilds when upstream source changes**

You used to rely on the implicit `hash` strategy on a `^:build` dep. v2.3 still defaults to `hash`
for deps that declare outputs, so this should not change — but if the upstream task lost its
`outputs` declaration, the dep silently flipped to `ignored`. Re-declare outputs on the upstream
task or explicitly set `cacheStrategy: 'hash'`.

**A build task rebuilds even when only an upstream's source comments changed**

The upstream is contributing its full hash. Switch the dep to `cacheStrategy: 'outputs'` so only
output-file changes invalidate this task:

```yaml
tasks:
  build:
    command: 'webpack'
    deps:
      - target: '^:build'
        cacheStrategy: 'outputs'
```

**Build invalidated by a `test` dep**

You declared `deps: ['~:test']` on a build task in pre-v2.3 expecting the lint/test changes to
invalidate. v2.3 makes this `ignored` by default. Set `cacheStrategy: 'hash'` explicitly if you need
the old behavior.

---

## Experimental caching layers

Available in v2.3+.

Two opt-in experiments can change how the local cache stores and verifies content. If a user reports
unexpected cache behavior, check whether either of these is enabled in `.moon/workspace.yml`:

```yaml
experiments:
  casOutputsCache: true # local content-addressable store for task outputs
  nativeFileHashing: true # bypass VCS for input hashing
```

### `casOutputsCache`

When enabled, task outputs are stored in a local content-addressable store (CAS) instead of as
`.tar.gz` archives. This changes the on-disk layout of `.moon/cache/outputs/` significantly — if you
inspect cache files directly, the per-hash `.tar.gz` will not be there.

**What to check when this is on:**

- `ls .moon/cache/outputs/` will look different (sharded by content hash, not flat).
- `tar tzf` won't work on individual blobs.
- Archiving and hydration currently run on the main thread (not the daemon), so they may be slower
  than the legacy tarball path.

**Quick toggle for diagnosis:**

```yaml
# Temporarily disable to confirm the experiment is the culprit
experiments:
  casOutputsCache: false
```

The optional `cache.cas.verifyIntegrity` setting forces re-verification of every read. If hydration
fails with a corruption error, this is the first thing to flip on.

### `nativeFileHashing`

When enabled, input hashing runs inside moon's task pool instead of shelling out to Git. This is
generally faster (10–50% in benchmarks) but produces hashes from a different code path than the VCS
default.

**Symptoms that suggest this experiment is involved:**

- Hashes don't match what they were before enabling the experiment — expected, but worth confirming.
- Hash diff (`moon hash <a> <b>`) attributes the change to file content even though Git reports the
  file as identical.

**Quick toggle for diagnosis:**

```yaml
experiments:
  nativeFileHashing: false
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

| Flag            | Reads cache | Writes cache | Use when                                                   |
| --------------- | ----------- | ------------ | ---------------------------------------------------------- |
| `--force`       | No          | Yes          | You want a fresh run but still want to populate the cache. |
| `--cache off`   | No          | No           | You want to completely bypass caching (e.g., debugging).   |
| `--cache read`  | Yes         | No           | You want to use existing cache but not pollute it.         |
| `--cache write` | No          | Yes          | Same as `--force` but more explicit.                       |
| (default)       | Yes         | Yes          | Normal operation.                                          |
