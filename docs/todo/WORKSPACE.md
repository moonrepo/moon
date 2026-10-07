# Workspace graph building: performance and cleanup plan

An audit of the `moon_workspace` crate, which builds the project and task graphs that nearly every
moon command needs. Written on 2026-10-07 against `3.0-async-graph` at `28ee8b4bee`, after the
synchronous graph builder and the `asyncGraphBuilding` experiment were removed. Line references are
as of that commit, and will drift.

Estimates are marked as such. Only the benchmark numbers were measured.

## Summary

Every command builds the graph through `WorkspaceBuilder::new_with_cache`
([session.rs:390](../../crates/app/src/session.rs#L390)), so the **cache hit path** is what nearly
every invocation pays, and it does far more than reading a cached file:

1. `preload` globs for projects, then parses every project's `moon.*` config, with one blocking task
   per project. On a hit, the parsed configs are thrown away.
2. The digest calls `load_all` on both plugin registries, which instantiates every configured
   toolchain and extension WASM plugin, only to read their versions and manifest file names.
3. Every candidate config and manifest path is hashed, and a manifest blob is written to the CAS.
4. `workspaceGraph.json` is read through `starbase_utils::json::read_file`, which strips comments
   from the whole file before parsing it.
5. `build` runs 3 git subprocesses in sequence, enforces constraints, and then both builders'
   `finalize` deep-clone every project and task. `ProjectGraph::set_graph` also re-runs a cycle
   check per edge, on a graph that was already validated.

### Benchmark baseline

`WorkspaceBuilder/build_graphs`
([workspace_graphs.rs](../../crates/workspace/benches/workspace_graphs.rs)) only covers the uncached
path:

| Projects | Mean     | 95% CI         | Notes               |
| :------- | :------- | :------------- | :------------------ |
| 100      | 91.8 ms  | 89.1–95.7 ms   | 11% outliers        |
| 1000     | 323.6 ms | 252.8–402.0 ms | 21% severe outliers |

The fixed cost is ~65–85 ms, as the bench creates the whole context (a cache engine, both plugin
registries, and a `Git::load`) inside the timed closure, and `build` shells out to git. The marginal
cost is only ~0.26 ms per project. The 1000 project run is very noisy, most likely because of
unbounded thread fan-out while loading configs
([F11](#f11-bound-config-loading-and-apply-it-in-order)).

### Cache file size

On this repo (6 projects, 21 tasks), `workspaceGraph.json` is 37.8 KB:

- `projects.graph` is 15.4 KB, of which `inherited` is 7.6 KB (49%), `config` 3.2 KB, and
  `fileGroups` 1.8 KB.
- `tasks.graph` is 20.3 KB, of which `options` is 11.1 KB, as every `TaskOptions` field is
  serialized, including defaults.
- `config_paths` is 1.2 KB, as it lists every _possible_ config file name per project.

Extrapolated linearly (estimate), a 5000 project repo has tens of MB of JSON that's read,
comment-stripped, parsed, and then cloned again in `finalize`.

## Findings

| #   | Finding                                                   | Category    | Path     | Impact (estimate)      |
| :-- | :-------------------------------------------------------- | :---------- | :------- | :--------------------- |
| F1  | Configs parsed before the cache check, discarded on a hit | perf        | hit      | high                   |
| F2  | Every WASM plugin instantiated to compute the digest      | perf, cross | hit      | high (measure first)   |
| F3  | `finalize` deep-clones every project and task             | perf        | both     | medium                 |
| F4  | `#[instrument]` formats the whole build data map          | perf        | miss     | low–medium             |
| F5  | 3 sequential git subprocesses in `build`                  | perf        | both     | low–medium             |
| F6  | Cache file payload and I/O                                | perf        | hit      | medium–high (with c/d) |
| F7  | Cycles checked twice, per edge                            | perf, cross | both     | low (high for chains)  |
| F8  | `interactive` dropped from slimmed task options (**bug**) | correctness | build    | n/a                    |
| F9  | State saved before the graph file is written              | correctness | miss/hit | n/a                    |
| F10 | `new_with_cache` panics when `vcs` is `None`              | correctness | n/a      | n/a                    |
| F11 | Unbounded, out-of-order config loading                    | perf        | both     | low (less variance)    |
| F12 | Serial `exists` probes for `inheritedBy.files`            | perf        | hit      | low                    |
| F13 | Allocation churn on the digest and extraction paths       | perf        | both     | low                    |
| F14 | `inherit_local_config` clones every `ProjectConfig`       | perf, cross | miss     | low–medium             |
| F15 | Tag-scoped task deps add project deps without graph edges | correctness | build    | needs a decision       |
| F16 | Vestigial state and small API cleanups                    | cleanup     | n/a      | n/a                    |
| F17 | Test coverage gaps                                        | tests       | n/a      | n/a                    |

### F1. Parse configs only on a cache miss

Configs are parsed by `preload`
([workspace_builder.rs:98](../../crates/workspace/src/workspace_builder.rs#L98),
[projects_builder.rs:686](../../crates/workspace/src/projects_builder.rs#L686)), but the digest only
needs each project's ID and source. On a hit, the parsed configs are dropped at `return Ok(cache)`.
They're only parsed first because a `moon.yml` `id` rename changes the IDs used for the
fingerprint's `projects` map, and for the guard that compares cached and current IDs.

**Change:** Split `load` into `locate` (globs, VCS ignores, and `config_paths`, returning pre-rename
IDs) and `load_build_data` (parsing). `new_with_cache` locates, generates the digest, and returns on
a hit, and only parses on a miss. Key the fingerprint by pre-rename IDs (a rename still changes the
hashed `moon.yml`, so still invalidates). Move the `default_project` validation and
`determine_repo_type` to the build path, and replace the ID set guard with a sources set guard
([F9](#f9-write-the-graph-before-the-state-and-treat-a-bad-cache-as-a-miss)).

- **Impact:** Removes N config parses from every hit. Estimate 50–100 µs CPU per project, so ~10–30
  ms wall at 1000 projects.
- **Risk:** Medium. The fingerprint inputs change, so caches miss once (as they already do per
  release). `load_graphs_for` must still parse, and `InvalidDefaultId` must still error on a miss.
- **Verify:** The `cache`, `custom_id`, and `default_id` modules in
  [project_graph_test.rs](../../crates/project-graph/tests/project_graph_test.rs). Asserting that
  nothing is parsed on a hit needs a hook (a tracing marker, or a counting config loader), so that
  test is optional.

### F2. Cache plugin metadata instead of instantiating plugins for the digest

The digest calls `load_all` on both registries
([workspace_cache.rs:188](../../crates/workspace/src/workspace_cache.rs#L188)), which loads,
instantiates, and (for toolchains) calls `register_toolchain` on every plugin
([registry_loader.rs:96](../../crates/plugin/src/registry_loader.rs#L96),
[toolchain_plugin.rs:49](../../crates/toolchain-plugin/src/toolchain_plugin.rs#L49)). The
fingerprint only needs 3 static facts per plugin: its version, its manifest file names, and whether
it exports `extend_project_graph`. These are properties of the WASM file, not of the workspace. The
bench configures no plugins, so this cost doesn't show up in it.

**Change:** Persist per-plugin metadata in `WorkspaceGraphCacheState`, keyed by plugin ID with a
staleness key (locator, plus the WASM file's length and mtime, or its sha256). Resolve each plugin's
file without instantiating it (needs a small `moon_plugin` API, and possibly a warpgate one), and
only load plugins whose entry is missing or stale. Keep `load_all` on the miss path, which needs
`extend_project_graph` anyway.

- **Impact:** Potentially the largest hit path win in real repos (estimate: tens of ms per plugin
  warm, seconds cold). **Measure first**, with debug log timestamps around "Loading all plugins" on
  a repo with ~5 toolchains, and deprioritize if instantiation is under ~10 ms per plugin.
- **Risk:** Medium–high. Cross-crate, and correctness depends on the staleness key. The new state
  fields are backward compatible through `#[serde(default)]`. Network plugins must not re-download.
- **Verify:** `cache::plugins::*`. Add "a changed plugin version invalidates" and "unchanged plugins
  aren't instantiated on a hit" (a `register_toolchain` marker file in `tc-tier1`, like the existing
  `extend_project_graph` one, makes this testable).

### F7. Check cycles once, in bulk

The miss path checks each edge with `would_cycle_in_scope`
([projects_builder.rs:455](../../crates/workspace/src/projects_builder.rs#L455)), and every path
then inserts each edge into 2 daggy `Dag`s in `set_graph`, whose `add_edge` runs its own path check
([project_graph.rs:341](../../crates/project-graph/src/project_graph.rs#L341)). That's O(E·(V+E)),
on a graph that's already been validated on a hit.

**Change:** In `set_graph`, add each partition's edges in bulk and check once with
`is_cyclic_directed` (as daggy's `add_edges` does). Only on failure, fall back to per-edge insertion
to report the same first offending edge. `would_cycle_in_scope` in `build_graph` then becomes
redundant, unless failing before building the remaining projects is preferred.

- **Impact:** Small for sparse graphs. Estimate tens of ms at 5000 projects with deep chains.
- **Risk:** Low–medium. Cross-crate, and the error messages and first offending edge must match.
- **Verify:** `set_graph_*` unit tests, and the `cycles` module.

### F9. Write the graph before the state, and treat a bad cache as a miss

`new_with_cache` saves the state (with the new hash) _before_ writing `workspaceGraph.json`
([workspace_builder.rs:165](../../crates/workspace/src/workspace_builder.rs#L165)). If the process
dies between the two, the next run hits on a stale graph, and the ID set guard only catches added or
removed projects, not changed ones. The workspace watcher also deletes both files without taking the
lock ([workspace_watcher.rs:149](../../crates/app/src/watchers/workspace_watcher.rs#L149)).

**Change:** Write the graph first (ideally to a temp file, then rename), then save the state. Drop
the `cache_path.exists()` pre-check, and treat an unreadable graph as a miss (logged at debug)
instead of a hard error. With F1, compare cached project sources instead of IDs.

- **Verify:** A new `cache` test that corrupts `workspaceGraph.json`, and asserts the next build
  rebuilds instead of erroring.

### F11. Bound config loading, and apply it in order

`load_build_data` spawns a blocking task per project
([projects_builder.rs:715](../../crates/workspace/src/projects_builder.rs#L715)) and applies results
in completion order. Tokio's blocking pool grows to 512 threads, so large repos spin up hundreds of
threads to parse YAML (contending with the rayon glob walker), and which `DuplicateProjectId` error
is reported depends on which parse finishes first.

**Change:** Use `moon_async_utils::run_pooled_blocking_tasks`, which is bounded to the CPU count and
applies outputs in input order, with sources sorted by ID.

- **Impact:** Bounded threads and deterministic errors. Probably most of the 1000 project bench
  variance (a hypothesis, so re-run the bench to confirm).
- **Verify:** `errors_duplicate_ids`, `custom_id::errors_duplicate_ids_from_rename`, and the bench.

### F13. Reduce allocation churn

Small and mechanical, best done while touching these files:

- Pass paths by value through `generate_graph_cache_digest` and `hash_input_paths` (and,
  cross-crate, let `hash_files` take a `Vec`), instead of cloning `config_paths` and
  `plugin_input_paths`.
- Build the `TasksQuerent` once per project instead of boxing one per task with deps (needs
  `TaskDepsBuilder.querent` to be `&dyn TasksQuerent`, cross-crate).
- Hoist the `project.id` clone out of the per-task loop in `build_graph`.
- In `WorkspaceTasksQuerent::query_tasks`, look up ID-scoped targets directly instead of scanning
  every task of each project, and use a set for tag membership.

### F14. Pass `ProjectConfig` by value to `inherit_local_config`

`build_project` passes `&config` from owned build data
([projects_builder.rs:157](../../crates/workspace/src/projects_builder.rs#L157)), and
`inherit_local_config` immediately clones it
([project_builder.rs:247](../../crates/project-builder/src/project_builder.rs#L247)).

- **Impact:** One `ProjectConfig` clone per project. Estimate 5–10% of per-project build cost.
- **Risk:** Low. A signature change in `moon_project_builder`.

### F15. Tag-scoped task deps: edges or not? (decision)

`build_graph` adds edges from `project.dependencies` as built by `ProjectBuilder`. `extract_tasks`
runs afterwards, and `TaskDepsBuilder` pushes implicit project deps for `#tag:task` targets
([task_deps_builder.rs:107](../../crates/task-builder/src/task_deps_builder.rs#L107)), which never
become edges. So consumers that read edges (sync ordering in the action graph builder, and
`dependents_of` in the affected tracker) disagree with `project.dependencies` for tag deps. This is
pre-existing (the sync builder did the same), not a regression.

**Options:** Add a second edge pass for the dependencies that `extract_tasks` added, or stop pushing
tag deps into `project.dependencies`. Either is a behavior change, and needs a test in
`dependencies::dependency_types`.

### F16. Cleanups

- `renamed_ids` is only used within `load_build_data`, to detect duplicate renames, and is then
  cleared, yet it's a struct field that's serialized (always empty). Make it a local. The "will
  ignore these IDs within lookups" log is misleading, as no lookups use it.
- `WorkspaceBuilder::new` and `preload` are `async` without awaiting anything, and
  `determine_repo_type` returns a `Result` but can't fail. Changing `new` is a public API change, so
  do it with F1.
- `load_project_build_data`, `extend_project_build_data_with_plugins`, and `build_project` are `pub`
  but not re-exported, so can be `pub(crate)`, as can the `WorkspaceProjectsBuilder` fields that
  only `workspace_builder.rs` touches.
- `enforce_constraints` uses `neighbors_directed` plus `find_edge` per neighbor (with an "Is this
  safe?" comment). `edges_directed` yields the target and scope directly.
- `has_loaded_graphs` and `build` both treat zero projects as "not loaded", so an empty workspace
  locates and loads configs twice. Use an explicit flag or an `Option`.
- When plugins discover new inputs on a miss, the digest is generated twice, and the first manifest
  blob is orphaned in the CAS. Compute the fingerprint without storing it, and store once at the
  end.
- Document why `WorkspaceBuilderContext` is held in an `Option` (deserialization).

### F17. Test coverage gaps

- An `interactive` `wait` dep through the workspace builder (F8).
- A corrupt or stale `workspaceGraph.json` is a miss (F9), and `vcs: None` with caching (F10).
- Determinism: build the same sandbox twice, and once from the cache, and assert identical node
  order and `to_dot` output. `loads_from_cache` only compares keys.
- Unit tests for `WorkspaceTasksQuerent` (alias resolution, tag scope, `query_task_has_outputs`).
- `map_plugin_input_paths` and `projects_locator` edge cases, which are only covered indirectly.
- Bench: add a cached (hit path) benchmark through `new_with_cache`, move context creation out of
  the timed region with `iter_batched`, and consider a chained workspace for F7.

## Implementation order

Each step can land on its own.

1. **Correctness, no format changes:** F8 (with its test), F10, and the write ordering half of F9.
2. **Zero-risk perf:** F4, F5 (`try_join!` only), F12, and the F16 cleanups that don't touch
   serialized fields.
3. **No clones in `finalize`:** F3.
4. **Bounded config loading:** F11, then re-run the 1000 project bench.
5. **Restructure the cached flow:** F1, the sources guard from F9, overlapping git calls from F5,
   `renamed_ids` as a local, and the double digest fix from F16. Changes the fingerprint inputs (one
   cache miss), and `WorkspaceBuilder::new` can become sync with `preload` removed (public API).
6. **Slim the cache file:** F6 (a) and (b). Changes the `workspaceGraph.json` shape. (c) and (d)
   only after a decision on JSON compatibility.
7. **Cross-crate perf:** F14, F7, and the cross-crate parts of F13.
8. **Plugin metadata caching:** F2, after measuring.

Decision needed: F15.

## Not recommended

- **A binary cache format (bincode, postcard):** `NodeState`'s `Deserialize` relies on
  `deserialize_any`, which non-self-describing formats reject. Revisit only if F6 isn't enough.
- **Persisting file hashes across processes:** Reintroduces the racily clean problems that the
  in-process memo was designed around.
- **Lazy `vcs_*` values in `GraphExpanderContext`:** Ripples into the token expander, and F5 gets
  most of the win.
- **Parallelizing `extract_tasks` or `WorkspaceTasksBuilder::build`:** Cheap relative to building
  projects, and they borrow `&mut Project` across the querent.
- **Removing the cache hit guard:** It's still useful against half-written state (F9).
- **Caching the project discovery glob walk:** It runs once per process.
- **Removing partial loading:** Test-only, small, and regression tested.
- **Avoiding the per-plugin `sources.clone()` when extending:** It's serialized into the WASM call
  anyway.
- **Fx maps in the fingerprint:** The hash depends on sorted iteration.
