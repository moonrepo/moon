# Cache moon workspace graph

A composite action that persists moon's workspace graph cache
(`.moon/cache/states/workspaceGraph.json` and `workspaceGraphStateV1.json`) between CI runs using
[`actions/cache`](https://github.com/actions/cache).

## Why?

As of moon 2.4.2, the workspace graph cache avoids all `extend_project_graph` plugin calls on a
cache hit — for example the Go toolchain's `go list -deps`, which can take minutes in large
workspaces. However, this cache is _local state_ and is **not** covered by remote caching (remote
caching only stores task hashes and output archives). On ephemeral CI runners the graph is
therefore rebuilt from scratch on every run.

This action restores and saves those state files so warm CI runs skip the expensive plugin calls.

Restoring a stale cache is always safe: moon computes its own digest (from project sources, config
files, toolchain manifests, plugin versions, and environment) and rebuilds the graph if it doesn't
match.

## Usage

Add the action after checkout and before the first `moon` command:

```yaml
jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
      - uses: moonrepo/moon/.github/actions/cache-moon-workspace@master
      - uses: moonrepo/setup-toolchain@v0
      - run: moon ci
```

## Inputs

| Input            | Default                                        | Description                                                                            |
| ---------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------- |
| `workspace-root` | `.`                                            | Relative path from the repository root to the moon workspace root.                     |
| `key-prefix`     | `moon-workspace-graph`                         | Prefix for the cache key. Change it to force a fresh cache.                            |
| `manifests`      | `**/go.mod`, `**/package.json`, `**/Cargo.toml` | Newline-separated globs of toolchain manifest files that feed the workspace digest.    |

Tune `manifests` to the toolchains enabled in your workspace, e.g. for a Go-only workspace:

```yaml
- uses: moonrepo/moon/.github/actions/cache-moon-workspace@master
  with:
    manifests: |
      **/go.mod
```

## Caveats

- The cached graph stores absolute paths, so it's only reused when the checkout path is identical
  between runs. GitHub-hosted runners use a stable path; self-hosted runners with varying work
  directories won't get hits.
- moon's digest includes the moon version, so the first run after a moon upgrade is always a miss.
- Jobs running in a `container:` produce a different digest than jobs running directly on the
  runner, so the two won't share a cache.
- `actions/cache` only saves on an exact key miss. The `restore-keys` fallback still helps, since
  moon re-validates the restored state itself.
