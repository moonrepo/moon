# Workspace graph: node-level caching in the CAS

A design for caching the workspace graph per project node, in the local content-addressable store,
instead of as one file holding the entire graph. Written on 2026-10-15 against `3.0-ws-cache` at
`aac8b85f81` (the head of `develop-3.0` at the time), after the graph perf series in
moonrepo/moon#2751 and moonrepo/moon#2767. Line references are as of that commit, and will drift.

Estimates are marked as such. Only the timings in "Today's cache" were measured, and they're
directional (debug binary, one machine). Nothing here has been implemented.

This supersedes the cache-related findings in [WORKSPACE.md](./WORKSPACE.md) (F1, F6, F9, F16's
double digest), and composes with the rest (F2, F15). See
[Relationship to WORKSPACE.md](#relationship-to-workspacemd) at the end.

## Summary

The cache is one JSON blob validated by one hash. Any input change rebuilds and rewrites the whole
graph, and every hit reads and deserializes the whole graph, whatever the command needs. At 1000
trivial projects the hit path is already slower than a rebuild; in real repos the file is tens of MB
and the rebuild is seconds, so the cache helps, but a single `moon.yml` edit still costs a full
rebuild, and every `moon run` still pays a full deserialize.

Per-node caching is possible, with one structural condition: **the unit is the project** (its
`Project`, its unexpanded tasks, and its raw outgoing relationships), never an individual task node,
and the build splits into a per-node phase that is a pure function of a project's own inputs, plus a
cheap assembly phase that is recomputed on every load. Four computations cross project boundaries
today, and each gets a specific treatment (memoize-and-diff for plugins, assembly-time alias
resolution, always-recomputed task dependency resolution, a global layer for inheritance).

Storage requirements, set by Miles:

- Cached nodes live in the **local CAS only**, never in `states/` (which is going away) and never in
  a remote backend.
- It must work from the **daemon**, whose watcher today deletes the cache files and rebuilds
  everything.

The design adds a third keyed `CasStore`, `.moon/cache/graph/`, next to `blobs/` and `manifests/`. A
node object is keyed by the hash of its inputs; a single index object, keyed by a deterministic hash
of the workspace root, holds the topology and the per-node keys. The daemon stops deleting anything
and runs the same incremental builder, which decides what is dirty from fingerprints, so the daemon
needs no event-to-node mapping of its own.

Three phases: (1) partial rebuild and rewrite, CAS-backed, with the daemon changes; (2) lazy payload
loading, so a targeted `moon run` reads only what it reaches; (3) precision and speed (per-file
`.moon/tasks` hashing, a persisted stat cache, payload trimming).

## Today's cache

### What is stored

One file, `.moon/cache/states/workspaceGraph.json`, is the serialized `WorkspaceBuilder`
([workspace_builder.rs:220](../../crates/workspace/src/workspace_builder.rs#L220)):
`projects.{aliases_to_ids, graph}` and `tasks.graph`, where each graph is a petgraph
`DiGraph<NodeState<Project | Task>, _>` with the node weights inline. A sibling state file,
`workspaceGraphStateV1.json`
([workspace_cache.rs:19](../../crates/workspace/src/workspace_cache.rs#L19)), holds the last hash
and the plugin-discovered input paths.

On this repo (6 projects, 21 tasks) the graph file is 19 KB. At 1000 trivial projects with 4 tasks
each it is 2.2 MB, of which `Project.config` with its defaults is 0.5 of the 0.8 MB of project
bytes, and unexpanded task `inputs` are 0.5 of the 1.6 MB of task bytes. Absolute `root` paths are
repeated per project. Extrapolated (estimate), a 2000 project repo with 15 tasks per project and
real inputs is 20–30 MB of JSON.

### What invalidates it

One `WorkspaceGraphFingerprint`
([workspace_cache.rs:31](../../crates/workspace/src/workspace_cache.rs#L31)), built by
`create_graph_cache_hasher`
([workspace_cache.rs:180](../../crates/workspace/src/workspace_cache.rs#L180)):

| Field        | Contents                                                                                                                                                                                                                                                                                         |
| :----------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `projects`   | final project id → source, which requires every project config parsed first (renames via `id:`)                                                                                                                                                                                                  |
| `inputs`     | content hashes of `.moon/*.yml`, `.moon/tasks/**/*`, every project config candidate name × every source, toolchain `manifest_file_names` at the root and in every source, plugin-discovered `input_files` from the previous build, `inheritedBy.files` × every source; missing files are omitted |
| `extensions` | versions of extension plugins that export `extend_project_graph`                                                                                                                                                                                                                                 |
| `toolchains` | versions of toolchain plugins that export `extend_project_graph`                                                                                                                                                                                                                                 |
| `env`        | `MOON_OUTPUT_STYLE`, `MOON_RETRY_COUNT`                                                                                                                                                                                                                                                          |
| `in_docker`  | because the file holds absolute paths                                                                                                                                                                                                                                                            |
| `version`    | `MOON_VERSION`                                                                                                                                                                                                                                                                                   |

Known gaps (from moonrepo/moon#2726's follow-ups): lock files and `config_file_globs` feed toolchain
detection (`detect_project_usage`,
[toolchain_plugin.rs:352](../../crates/toolchain-plugin/src/toolchain_plugin.rs#L352)) but aren't
hashed, so adding a lockfile can leave `inheritedBy.toolchain` stale; `define_requirements` output
depends on plugin versions that aren't hashed unless the plugin also exports `extend_project_graph`.

### The hit and miss flows

`new_with_cache` ([workspace_builder.rs:86](../../crates/workspace/src/workspace_builder.rs#L86)):

1. Take `workspaceGraph.lock`, load the state file.
2. `preload`: locate sources (globs), then **parse every project config**
   ([projects_builder.rs:354](../../crates/workspace/src/projects_builder.rs#L354),
   [projects_builder.rs:734](../../crates/workspace/src/projects_builder.rs#L734)).
3. Hash. On a match, read and deserialize the whole graph file, compare the project id sets, and
   return the cached builder. The parsed configs are dropped.
4. On a miss, call every plugin's `extend_project_graph` with all sources
   ([projects_builder.rs:829](../../crates/workspace/src/projects_builder.rs#L829)), build every
   project in parallel
   ([projects_builder.rs:462](../../crates/workspace/src/projects_builder.rs#L462)), resolve task
   deps across projects
   ([projects_builder.rs:382](../../crates/workspace/src/projects_builder.rs#L382)), build the task
   DAG, write the graph file, then the state.

Measured during the item 6 work on `3.0-ws-graphs` (debug binary, 1000 trivial projects, directional
only): hit path 142 ms, of which JSON deserialize 73 ms and hashing 44 ms, against a full rebuild of
71 ms. The bench shape has no plugins and trivial configs, which is why the miss is cheap there;
real repos pay plugin calls (the JS plugin reads every `package.json` inside WASM), toolchain
detection probes, and real config parsing on every miss.

### The daemon

`WorkspaceWatcher`
([workspace_watcher.rs:51](../../crates/app/src/watchers/workspace_watcher.rs#L51)) reacts to
`.moon/{workspace,toolchains,extensions}.*`, `.moon/tasks/**`, any `moon.*`, and project directory
creation or removal. For the last two it calls `rebuild_graphs`
([workspace_watcher.rs:140](../../crates/app/src/watchers/workspace_watcher.rs#L140)), which
**deletes both cache files** and then runs `MoonSession::rebuild_graphs`
([session_atomic.rs:42](../../crates/app/src/session_atomic.rs#L42)): `get_workspace_graph()`, which
is `new_with_cache` (a guaranteed miss), followed by a swap of `DaemonState.workspace_graph`. So the
daemon is a cache warmer plus an in-memory graph holder. It does not react to manifests
(`package.json`), lockfiles, or `inheritedBy.files` changes at all.

## Why the graph is not a per-node function

Four computations read across project boundaries. Each one is what makes a naive "hash the project's
files, cache its node" scheme wrong, and each has a treatment that restores purity:

| Computation                                                                                                                                                                                                                                                                      | Where                                                                                                                                                                | Treatment                                                                                                                                                                                                                                                                                                                                                                               |
| :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `extend_project_graph` takes **all** sources and returns per-project outputs. The JS plugin (`toolchains/javascript/src/tier2.rs` in moonrepo/plugins) derives a project's alias from its package name, and its implicit deps from its package deps ∩ the other packages' names. | [projects_builder.rs:110](../../crates/workspace/src/projects_builder.rs#L110)                                                                                       | **Memoize and diff.** Run the plugin phase only when its own fingerprint changes. Hash each project's `Vec<ExtendProjectOutput>`; a project is dirty only when its own output hash changed. The plugin cost is unchanged on a plugin-layer miss, but what moon rebuilds afterwards is proportional to what the outputs changed.                                                         |
| Alias resolution of dependency ids when project edges are created.                                                                                                                                                                                                               | [projects_builder.rs:505](../../crates/workspace/src/projects_builder.rs#L505)                                                                                       | **Move to assembly.** Nodes store raw ids. Edges are resolved against the current alias map when the graph is assembled. `ProjectExpander::expand_deps` ([project_expander.rs:34](../../crates/project-expander/src/project_expander.rs#L34)) already resolves aliases at expansion time, so this applies the existing pattern one step earlier. Alias changes then never dirty a node. |
| Task dependency resolution reads other projects' tags, task targets, task options (`persistent`, `runInCI`, `onFailure`, `interactive`), and whether a dependency has outputs.                                                                                                   | [task_deps_builder.rs:42](../../crates/task-builder/src/task_deps_builder.rs#L42) via [projects_builder.rs:382](../../crates/workspace/src/projects_builder.rs#L382) | **Always recompute.** It is hash-map lookups per dependency. At 4000 tasks it is low single-digit ms (estimate from the per-task work; measure in phase 1). Its inputs are small task summaries, which the index carries in phase 2, so no cross-node fingerprint is ever needed.                                                                                                       |
| Inheritance matching (`.moon/tasks/*` plus `inheritedBy` conditions), the root project and repo type, constraints config.                                                                                                                                                        | `inherit_global_configs`, [projects_builder.rs:585](../../crates/workspace/src/projects_builder.rs#L585)                                                             | **Global layer** in v1: any change rebuilds every node, as today. Phase 3 narrows `.moon/tasks` to per file.                                                                                                                                                                                                                                                                            |

With those four handled, phase A has a clean statement: **a project node is a pure function of
(global layer, the project's own files, the project's own plugin extension outputs).** Nothing in
phase A reads another project. That purity is also exactly what makes a content-keyed store correct:
the node's key is a hash of those inputs and nothing else.

## Design

### The unit: a project node

A node object holds one project's `Project` (with its tasks still embedded, as `ProjectBuilder`
produces it), the project's plugin extension outputs (its build input, kept so a dirty node can
rebuild without re-running plugins), and the key it was stored under. Three things are deliberately
**not** final in the payload, and are produced by assembly on every load:

- Dependency ids are raw (an alias stays an alias).
- Task deps are as configured, not resolved to targets (`^:build` stays `^:build`).
- `Project.root` is absent; it's derived from the source and the workspace root at load.

Task nodes are not cached individually. A project's tasks are built together by `TasksBuilder` from
the same inheritance chain, and a `moon.yml` edit invalidates all of them at once anyway, so the
project is the natural unit, and it keeps the object count at one per project.

### Storage: a keyed store in the local CAS

#### Why a keyed store

`LocalStorage` ([local_storage.rs:34](../../crates/cache-local/src/local_storage.rs#L34)) owns two
`CasStore`s with different semantics:

- `blobs/`: content-addressed. Key = SHA-256 of the bytes, integrity verified on read.
- `manifests/`: key-value. Key = the task's input hash, value = the `TaskManifest`. Integrity
  verification is disabled
  ([local_storage.rs:68](../../crates/cache-local/src/local_storage.rs#L68)) because the key is not
  the content hash. The comment on `TaskManifest::digest_source`
  ([task_manifest.rs:40](../../crates/manifest/src/task_manifest.rs#L40)) spells this out: "works
  more like a key-value store".

Graph objects are the second kind. The key is the fingerprint of a node's inputs, computed before
the node exists; the value is the built node, which nothing can hash before building. So graph
objects go in a keyed store, not in `blobs/`.

#### Why a third store, not `manifests/`

The manifest GC (`evict_manifests`,
[local_storage.rs:260](../../crates/cache-local/src/local_storage.rs#L260)) reads every object in
the store, parses it as a `TaskManifest`, and silently `continue`s on anything that doesn't parse.
Graph objects stored there would never be evicted, and the blob reachability sweep that follows
would never know about them. A dedicated store keeps the GC rule for graph objects separate (a plain
TTL, see below) and keeps `evict_manifests` untouched.

Add to `LocalStorage`:

```rust
graph: Arc<CasStore>,   // CasStore::new(cache_dir.join("graph"), config with verify_integrity = false)
```

It's sharded like the others (`graph/<2 hex>/<62 hex>`), writes are temp + rename
([cas.rs:63](../../crates/cas/src/cas.rs#L63), [cas.rs:427](../../crates/cas/src/cas.rs#L427)), so
readers never see partial objects and no lock is needed for object writes.

#### Why not `blobs/` plus a pointer

An alternative keeps payloads content-addressed in `blobs/` and stores a tiny pointer object
(fingerprint → payload digest) in the keyed store, the way task manifests reference output blobs. It
costs two files per node and a new GC root kind (the blob sweep would have to learn that pointer
objects keep payload blobs alive), and buys dedupe that never happens (every node is distinct by id
and source) plus integrity verification that an embedded key field and a serde failure already cover
(both are treated as a per-node miss). Not worth it.

#### Two object kinds

| Object | Key (a `Digest`)                                                                                                                                                  | Value                      | Written when                 |
| :----- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------- | :--------------------------- |
| Node   | `ContentHasher("workspace-graph-node")` over the node fingerprint (global hash, layer-2 partial hash, extension hash; see [Fingerprints](#fingerprints-and-keys)) | `ProjectNodePayload` JSON  | as soon as the node is built |
| Index  | `ContentHasher("workspace-graph-index")` over `{ workspace_root, schema }`: deterministic, the one mutable pointer                                                | `WorkspaceGraphIndex` JSON | last, when anything changed  |

The index key is the only non-content key in the design. It's what lets a load find "this checkout's
current graph" without knowing any content hash up front. Everything else is reached from the index.

#### Local only

Graph objects follow the existing "hash manifests (local only)" pattern:
`Storage::store_hash_manifest_with_hasher`
([storage.rs:348](../../crates/cache-storage/src/storage.rs#L348)) iterates `get_local_backends()`
([storage.rs:226](../../crates/cache-storage/src/storage.rs#L226)) and writes inline, and
`load_hash_manifest` ([storage.rs:402](../../crates/cache-storage/src/storage.rs#L402)) reads from
local backends only. Note that the general `retrieve_blobs`
([storage.rs:289](../../crates/cache-storage/src/storage.rs#L289)) consults remotes, so the graph
methods must not go through it.

API shape, mirroring the optional trait methods that remotes already leave as no-ops (`gc`,
[storage_backend.rs:47](../../crates/cache-storage/src/storage_backend.rs#L47);
`find_blobs_by_prefix`,
[storage_backend.rs:142](../../crates/cache-storage/src/storage_backend.rs#L142)):

```rust
// crates/cache-storage: StorageBackend (new optional methods, defaults for remotes)
pub enum KeyedStore {
    TaskManifests, // the existing `manifests/`
    Graph,         // the new `graph/`
}

async fn retrieve_keyed_object(&self, store: KeyedStore, key: &Digest) -> miette::Result<Option<BlobOutput>> {
    Ok(None)
}
async fn store_keyed_object(&self, store: KeyedStore, key: Digest, bytes: Vec<u8>) -> miette::Result<()> {
    Ok(())
}
/// Refresh the object's mtime when it's older than `if_older_than`, so GC treats it as used.
async fn touch_keyed_object(&self, store: KeyedStore, key: &Digest, if_older_than: Duration) -> miette::Result<()> {
    Ok(())
}

// crates/cache-storage: Storage (local backends only, like load_hash_manifest)
pub async fn load_graph_object(&self, key: &Digest) -> miette::Result<Option<BlobContent>>;
pub async fn store_graph_object(&self, key: Digest, bytes: Vec<u8>) -> miette::Result<()>;
pub async fn touch_graph_object(&self, key: &Digest, if_older_than: Duration) -> miette::Result<()>;
```

`LocalStorage` implements the three against `self.graph` on `spawn_blocking`, the same way
`retrieve_task_manifest` / `store_task_manifest`
([local_storage.rs:130](../../crates/cache-local/src/local_storage.rs#L130),
[local_storage.rs:152](../../crates/cache-local/src/local_storage.rs#L152)) do. A local read returns
`BlobContent::File(path)`, so loading a payload is one file read with no copy through the storage
layer. Writes are inline (not through `background_tasks`), because the very next step, or another
process, reads them back.

The `KeyedStore` enum lets `manifests/` be reached through the same methods later, and keeps the
trait from growing a method pair per store. If touching the trait is unwelcome, the three graph
methods can be inherent methods on `LocalStorage` reached through a downcast, but the trait already
carries local-only optional methods, so the enum is the cleaner shape.

#### Cache mode

`get_cache_mode` ([cache_mode.rs:77](../../crates/cache-item/src/cache_mode.rs#L77)) is honored
through the engine's `is_readable` / `is_writable`:

| `MOON_CACHE` | Index and node reads | Node and index writes |
| :----------- | :------------------- | :-------------------- |
| `off`        | no                   | no                    |
| `read`       | yes                  | no                    |
| `write`      | no                   | yes                   |
| `readwrite`  | yes                  | yes                   |

Today the builder only skips the cache when VCS is disabled (it still needs VCS for nothing on the
hit path; the check is historical). Keep that behavior for the no-VCS case, and add the mode check.

#### The lock

`workspaceGraph.lock` lives in `locks/`, not `states/`, so it stays. It shrinks to the index write.
Node writes need no lock: two processes that build the same node from the same inputs produce the
same key and the same bytes, and `CasStore::write` is atomic. See [Two writers](#two-writers) for
the index race.

#### Shared worktree cache, and path-free payloads

With `cache.sharedWorktreeCache` on ([session.rs:212](../../crates/app/src/session.rs#L212)),
`blobs/` and `manifests/` live in a directory shared by all worktrees of the repo. The graph store
goes there too, which makes node objects shareable: two checkouts of the same branch have identical
inputs and so identical keys. Two requirements follow.

**No absolute paths in payloads.** `Project.root`
([project.rs:73](../../crates/project/src/project.rs#L73)) is the only one, set from
`source.to_logical_path(workspace_root)` in `ProjectBuilder::new`
([project_builder.rs:77](../../crates/project-builder/src/project_builder.rs#L77)). Mark it
`#[serde(skip)]` and set it when a payload is loaded. Unexpanded `Task`s hold no absolute paths
(`inputs` are `Input` values, `TaskOptions` has no `PathBuf`), and `GraphExpanderContext` is runtime
only. User configs can contain absolute paths of their own, which is their business. This also
removes a measured size driver.

**The index key includes `workspace_root`**, so each worktree has its own index while sharing nodes.
`in_docker` stays in the global hash for now (it exists because of absolute paths; drop it in phase
3 once a test proves payloads are path-free).

#### Garbage collection

Extend `LocalStorage::gc`
([local_storage.rs:95](../../crates/cache-local/src/local_storage.rs#L95)): after the manifest
eviction and the blob reachability sweep, run `self.graph.gc(lifetime)`
([cas.rs:276](../../crates/cas/src/cas.rs#L276), [gc.rs:14](../../crates/cas/src/gc.rs#L14)), the
plain mtime TTL sweep. Graph objects are small (one per project) and stay out of the `max_size`
budget, which exists for task outputs.

To make eviction least-recently-used rather than a fixed TTL, as manifests already are (they're
touched on a hit, [local_storage.rs:130](../../crates/cache-local/src/local_storage.rs#L130)), the
builder touches what it used, through `touch_graph_object(key, 24h)`:

- every node object a load used, when its mtime is older than 24 hours. The existence check in
  validation already needs the object's metadata, so the age decision is free;
- the index on every write, and on every load when older than 24 hours, so an unchanged but actively
  used workspace never loses its index to the TTL.

`moon clean --all` and the daemon's `CleanCache` RPC both reach this through `Storage::clean`
([storage.rs:253](../../crates/cache-storage/src/storage.rs#L253)); nothing to add. An evicted node
is a per-node miss: rebuilt and re-stored on the next load.

### Data model

```rust
// Stored at the deterministic index key. One per workspace root.
pub struct WorkspaceGraphIndex {
    /// Bumped whenever the index or payload shape changes. Independent of MOON_VERSION,
    /// which is in the global hash and already invalidates every node per release.
    pub schema: u32,
    pub global_hash: ContentHash,          // layer 0
    pub plugins_hash: ContentHash,         // layer 1
    pub repo_type: RepoType,
    pub root_id: Option<Id>,
    pub aliases_to_ids: BTreeMap<String, Id>,
    pub plugin_input_paths: BTreeSet<WorkspaceRelativePathBuf>,
    /// Located source -> final id, so an unchanged source resolves its id without parsing.
    pub sources: BTreeMap<WorkspaceRelativePathBuf, Id>,
    pub projects: BTreeMap<Id, ProjectNodeEntry>,
}

pub struct ProjectNodeEntry {
    pub source: WorkspaceRelativePathBuf,
    /// From the locator, before an `id:` rename. The root project's inferred id is the
    /// workspace folder name, so it must be fingerprinted.
    pub inferred_id: Id,
    /// Layer 2 without the extension hash. Decides whether the final id can be reused.
    pub partial_hash: ContentHash,
    /// Hash of this project's Vec<ExtendProjectOutput>, in plugin order.
    pub extension_hash: ContentHash,
    /// hash(global_hash, partial_hash, extension_hash). The node object's key.
    pub node_key: Digest,

    // Phase 2 only. What assembly, constraints, and queries need without the payload:
    pub language: LanguageType,
    pub layer: LayerType,
    pub stack: StackType,
    pub tags: Vec<Id>,
    pub toolchains: Vec<Id>,
    pub aliases: Vec<ProjectAlias>,
    pub dependencies: Vec<ProjectDependencyConfig>,  // raw ids, incl. plugin-provided
    pub inherited_from: Vec<String>,
    pub tasks: BTreeMap<Id, TaskSummary>,
}

// Phase 2 only.
pub struct TaskSummary {
    pub target: Target,
    pub deps: Vec<TaskDependencyConfig>,  // as configured, unresolved
    pub tags: Vec<Id>,
    pub has_outputs: bool,
    pub type_of: TaskType,
    // Exactly the TaskOptions fields TaskDepsBuilder reads:
    pub on_failure: TaskOptionOnFailure,
    pub interactive: bool,
    pub run_in_ci: TaskOptionRunInCI,
    pub persistent: bool,
}

// Stored at the node key. One per project.
pub struct ProjectNodePayload {
    /// Must equal the key of the object it was read from, otherwise it's a stale write
    /// from another process, and the node is rebuilt in process.
    pub key: Digest,
    /// Tasks embedded, task deps unresolved, dependency ids raw, `root` skipped.
    pub project: Project,
    /// Build input, kept so a dirty node rebuilds without re-running plugins.
    pub extensions: Vec<ExtendProjectOutput>,
}
```

Phase 1 keeps the index small: entries carry the three hashes, the key, the source, and the ids.
Phase 2 adds the summaries. Size expectations (estimate): payload bytes ≈ today's file minus the
root paths; the phase-2 index ≈ 10–20% of payload bytes, since summaries carry targets, a few option
values, and dependency lists, while payloads carry `inputs`, `args`, `env`, `options`, and `config`.

`Project.tasks` is emptied when tasks move to the task graph today
([projects_builder.rs:382](../../crates/workspace/src/projects_builder.rs#L382)); the payload is
written **before** that, with the tasks still inside, so one object per project holds both.

### Fingerprints and keys

Three layers. Every node key contains the global hash, so a global change changes every key, and the
old generation of objects simply ages out.

**Layer 0, global.** Any change ⇒ every node key changes ⇒ every node rebuilt (today's behavior).

```rust
fingerprint!(struct GlobalFingerprint {
    version: String,                               // MOON_VERSION
    in_docker: bool,
    env: BTreeMap<String, String>,                 // MOON_OUTPUT_STYLE, MOON_RETRY_COUNT
    workspace_configs: BTreeMap<path, hash>,       // .moon/workspace.*, toolchains.*, extensions.*
    tasks_configs: BTreeMap<path, hash>,           // .moon/tasks/**/* (v1; phase 3 splits this)
    enabled_toolchains: Vec<Id>,
    plugin_versions: BTreeMap<Id, String>,         // ALL loaded toolchain + extension plugins
    repo_type: RepoType,
    root_id: Option<Id>,
})
```

`plugin_versions` covers every loaded plugin, not only those exporting `extend_project_graph`:
`define_requirements` and `detect_project_usage` outputs feed the build too (a gap today).
`repo_type` and `root_id` come from the located sources
([projects_builder.rs:585](../../crates/workspace/src/projects_builder.rs#L585)) and are global
because `ProjectBuilder` consults `root_project_id` and `monorepo` for every project.

**Layer 1, plugins.** Any change ⇒ run the plugin phase once, then diff per project.

```rust
fingerprint!(struct PluginsFingerprint {
    plugin_versions: BTreeMap<Id, String>,         // those exporting extend_project_graph
    plugin_configs: BTreeMap<Id, hash>,            // registry.create_config(id), serialized
    projects: BTreeMap<Id, source>,                // the plugin input itself
    inputs: BTreeMap<path, hash>,                  // stored plugin_input_paths + manifest_file_names
                                                   // at the root and in every source, missing omitted
})
```

**Layer 2, node (partial).** Any change ⇒ that node's key changes. Computed per project, in
parallel, without parsing the config.

```rust
fingerprint!(struct NodeFingerprint {
    inferred_id: Id,
    source: WorkspaceRelativePathBuf,
    configs: BTreeMap<file_name, hash>,            // every project config candidate name, missing omitted,
                                                   // so creating moon.yml where none existed invalidates
    detection: BTreeMap<Id, Vec<(file_name, bool)>>, // per loaded toolchain: existence of manifest_file_names,
                                                   // lock_file_names, and non-glob config_file_globs
    detection_globs: BTreeMap<Id, Vec<path>>,      // matched paths for glob-like config_file_globs (rare)
    inherited_by_files: BTreeMap<file_name, hash>, // inheritedBy.files candidates in the project root
    plugin_inputs: BTreeMap<path, hash>,           // stored plugin_input_paths under this source
})
```

`detection` closes the lockfile gap: `detect_project_usage` checks exactly these names, so
fingerprinting their existence makes the node key track what the builder would decide.

**Node key.**

```rust
hasher("workspace-graph-node").hash_content((global_hash, partial_hash, extension_hash))
```

where `extension_hash` is the hash of the project's `Vec<ExtendProjectOutput>` from layer 1, in
plugin order. It's folded in last, after the plugin phase, because it's the one layer-2 input that
isn't knowable from the project's own files.

**Never fingerprinted.** Alias resolution of edges, both cycle checks, task dependency resolution,
the implicit project deps that task deps create, constraints enforcement, `default_project`
validation, VCS info. All recomputed on every load, from index entries and payloads.

### Invalidation matrix

| Change                                                       | Result                                                                                                                                                                                                                       |
| :----------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Edit `packages/a/moon.yml`                                   | `a`'s partial hash and key change. `a` is rebuilt, one object stored, the index rewritten. Every other node is found by key.                                                                                                 |
| Create `moon.yml` in a project that had none                 | Same: the candidate name's hash appears in `configs`.                                                                                                                                                                        |
| Edit `packages/a/package.json`, name unchanged               | Layer 1 differs → plugins run once. Only `a`'s output hash changes → `a` rebuilt. Its new `extensions` are stored with it.                                                                                                   |
| Rename `a`'s package (its alias)                             | Plugins run. `a` rebuilt. Any `b` whose implicit deps referenced the old name has a changed output → `b` rebuilt. Projects depending on `a` by moon id are untouched: aliases resolve at assembly.                           |
| Add project `z`                                              | The locator finds a new source → `z` built. Layer 1 differs (the id map is the plugin input) → plugins run → projects whose manifests reference `z` change output → rebuilt. Everyone else reused.                           |
| Remove project `z`                                           | Entry dropped from the index; its object ages out through GC. Dependents' edges to `z` drop at assembly (a `Loading` placeholder, filtered in `finalize`). A hard `z:build` task dep errors from assembly, as it does today. |
| Rename `z` via `id:`                                         | `z`'s config hash changes → its node rebuilt under the new id; the old entry is removed by the source map. Dependents referencing the old id behave as for a removed project.                                                |
| Set `persistent: true` on `z:build`                          | `z` rebuilt. Assembly re-resolves `a:build → z:build` and raises `PersistentDepRequirement` without touching `a`'s object.                                                                                                   |
| Add tag `lib` to `z`                                         | `z` rebuilt. `#lib:build` deps everywhere re-resolve at assembly.                                                                                                                                                            |
| Edit `.moon/tasks/node.yml`                                  | v1: global hash → every key changes → all rebuilt. Phase 3: only nodes with `node.yml` in `inherited_from`, unless the file's `inheritedBy` block changed, which stays global.                                               |
| Edit `.moon/workspace.yml` or `toolchains.yml`               | All rebuilt (as today).                                                                                                                                                                                                      |
| Upgrade moon, or any loaded plugin                           | All rebuilt (as today, plus the `define_requirements` gap closed).                                                                                                                                                           |
| Add a lockfile to `packages/a`                               | `a` rebuilt (new; today this is missed).                                                                                                                                                                                     |
| Node object evicted, corrupt, or its embedded key mismatches | That node rebuilt in process and re-stored (if writable).                                                                                                                                                                    |
| Second worktree, shared cache on, same branch                | Node objects found by key; only the worktree's own index is written.                                                                                                                                                         |
| `MOON_CACHE=off`                                             | Full build, nothing read or written.                                                                                                                                                                                         |

### Load algorithm

Every `new_with_cache`:

1. **Index.** If readable, `load_graph_object(index_key)`. Missing, unparsable, or `schema` mismatch
   ⇒ an empty index. Record whether it was older than 24 hours (for the touch).
2. **Locate.** Sources from globs and configured sources
   ([projects_builder.rs:657](../../crates/workspace/src/projects_builder.rs#L657), minus the config
   loading), sorted. No config is parsed here. This is item 6's `locate_sources` split from
   `3.0-ws-graphs`'s stash, and the sources list is what every later step keys on.
3. **Global hash.** Compute `GlobalFingerprint`. Differs from `index.global_hash` ⇒ every entry is
   treated as absent. Nothing is deleted.
4. **Partial fingerprints, in parallel** (`run_pooled_blocking_tasks`, as config loading is today).
   For each `(inferred_id, source)`:
   - Find the entry through `index.sources[source]` → `index.projects[id]`.
   - Compute `NodeFingerprint` → `partial_hash`.
   - Entry exists and `entry.partial_hash == partial_hash` ⇒ **candidate-clean**: the final id is
     `entry`'s, no parse.
   - Otherwise ⇒ **dirty**: `load_project_build_data` now (parse the config, resolve the `id:`
     rename).
   - A source with no entry is **new** (parsed, like dirty). An entry with no source is **removed**.
   - Duplicate-id and `default_project` checks run on the final id set, as today.
5. **Plugins.** Compute `PluginsFingerprint` from the final id→source map. If it differs from
   `index.plugins_hash`, or any dirty/new node has no stored `extensions` to read (no object), run
   `extend_project_build_data_with_plugins` once with all sources, rebuild `aliases_to_ids`, collect
   `plugin_input_paths`, and hash each project's outputs into `extension_hash`. Otherwise take
   `aliases_to_ids`, `plugin_input_paths`, and every `extension_hash` from the index. For dirty
   nodes without a plugin run, their `extensions` come from their old object (read it; the project
   changed but its plugin outputs didn't).
6. **Node keys.** `node_key = hash(global_hash, partial_hash, extension_hash)` per project. For a
   candidate-clean node: key equals `entry.node_key` **and** `load_graph_object(key)` finds it ⇒
   **clean**; otherwise dirty (its plugin outputs changed, or the object was evicted). Keep the
   returned `BlobContent::File` path for step 8.
7. **Rebuild dirty and new nodes, in parallel** with `build_project`
   ([projects_builder.rs:149](../../crates/workspace/src/projects_builder.rs#L149)), unchanged, fed
   the stored or fresh `extensions`. As each finishes: serialize the payload with its key, and if
   writable `store_graph_object` immediately. Writing per node, not at the end, is what makes an
   aborted build leave useful work behind.
8. **Load clean payloads** (phase 1: all; phase 2: lazily). Deserialize, set `Project.root`, and
   reject a payload whose embedded `key` differs from the object key (another process's stale
   write): rebuild that node in process as in step 7.
9. **Assemble.** See [Assembly](#assembly).
10. **Index.** If anything changed (any dirty, new, or removed node; any layer hash; aliases or
    plugin input paths) and writable: take `workspaceGraph.lock`, `store_graph_object(index_key)`,
    release. Touch used node objects and the index per [Garbage collection](#garbage-collection).
    `finalize` as today.

#### Warm-run cost

Step 4 is ~10–20 `stat` calls per project (config candidates, detection names, `inheritedBy`
candidates) plus content reads of the files that exist. That is the same work today's single hash
does, attributed per node; it's not new cost. Step 6 is one `exists` per project. Step 8 is one
small file read per project in phase 1. Phase 3's stat cache turns step 4 into stats only, and the
daemon removes the pass from the CLI's critical path entirely once it serves graphs (not in scope).

Estimate at 2000 projects, warm, phase 1: step 4 ≈ 20–40 k syscalls spread over the blocking pool,
step 8 ≈ 2000 reads of ~10–15 KB. Tens of ms on macOS and Linux. Windows is the risk, see
[Risks](#risks-and-open-questions).

#### The index is never trusted

Validation recomputes every key from the filesystem and then checks the store. The index only
supplies: final ids for sources whose partial hash matched (guarded by that hash), plugin output
hashes and the alias map (guarded by `plugins_hash`), and the previous `plugin_input_paths` (which
only widen what layer 1 hashes). A stale or lost index costs extra misses, never a wrong graph. This
is the property that makes the daemon integration and the two-writer case simple.

#### Abort safety

Node writes are atomic and idempotent, and the index is written last. A build aborted halfway (the
daemon aborts an in-flight build when a new event arrives) leaves correct node objects and an older
index, which the next load reconciles by finding those objects by key in step 6. Today's ordering
comment at [workspace_builder.rs:218](../../crates/workspace/src/workspace_builder.rs#L218) (graph
before state, so a failure in between is a miss) carries over as "objects before index".

#### Two writers

Process A observes state S1. B observes S2 after an edit. B writes index(S2), then A writes
index(S1). The next load recomputes keys from disk (S2), finds that B already stored those node
objects, and rewrites the index. Correct, and the cost is one extra validation. The lock around the
index write only prevents interleaved partial writes, which `CasStore::write` already prevents, so
it's belt and braces, and it is held for microseconds rather than for the whole build as today.

### Assembly

Runs on every load, in this order, over index entries and loaded payloads:

1. Insert project nodes into the `ProjectDiGraph` in **sorted id order** (deterministic; today's
   order is parallel completion order).
2. For each project's `dependencies`, resolve the id through `aliases_to_ids` and add the edge,
   skipping `root` scope as today
   ([projects_builder.rs:505](../../crates/workspace/src/projects_builder.rs#L505) moves here and
   stops mutating the stored project).
3. `find_cycle_in_scopes` over the whole graph, as today.
4. `extract_tasks` ([projects_builder.rs:382](../../crates/workspace/src/projects_builder.rs#L382)):
   build the `tags_to_ids`, `tags_to_targets`, `ids_to_target_options`, and `target_to_has_outputs`
   maps from the loaded projects (phase 1) or from `TaskSummary` entries (phase 2), then
   `TaskDepsBuilder` per task with deps. This is where `^:build`, `#tag:build`, `alias:build`, and
   the `persistent`/`runInCI`/`allowFailure`/`interactive` constraints are checked, and where
   implicit project deps from task deps are pushed onto `project.dependencies`. The errors it raises
   today keep surfacing here, on every load that needs them.
5. `WorkspaceTasksBuilder::build` (task DAG) and `find_cycle`, as today.
6. `enforce_constraints`
   ([projects_builder.rs:610](../../crates/workspace/src/projects_builder.rs#L610)), which already
   runs post-cache today, from projects (phase 1) or entries (phase 2).
7. `default_project` validation, VCS info join, `finalize`.

Note F15 in [WORKSPACE.md](./WORKSPACE.md): implicit project deps added in step 4 never become edges
today. This design doesn't change that; the decision stays open, and whichever way it goes is an
assembly change only.

### The daemon

The daemon needs no event-to-node mapping of its own. It runs the same incremental builder, and the
[load algorithm](#load-algorithm) decides what is dirty. Changes to
[workspace_watcher.rs](../../crates/app/src/watchers/workspace_watcher.rs):

1. **Stop deleting.** `rebuild_graphs`
   ([workspace_watcher.rs:140](../../crates/app/src/watchers/workspace_watcher.rs#L140)) drops the
   two `fs::remove_file` calls
   ([workspace_watcher.rs:150](../../crates/app/src/watchers/workspace_watcher.rs#L150)) and the
   `STATE_*_FILE_NAME` imports, which no longer exist. It still aborts an in-flight build (safe, see
   [Abort safety](#abort-safety)), the callers still `reset_components`
   ([session_atomic.rs:58](../../crates/app/src/session_atomic.rs#L58)) so the session's `OnceCell`s
   rebuild, and `MoonSession::rebuild_graphs` still awaits `get_workspace_graph()`. That run
   validates, rebuilds only dirty nodes, stores them, rewrites the index, and swaps
   `DaemonState.workspace_graph`. The CLI's next invocation finds everything by key.
2. **Watch the inputs that now matter per node.** `on_file_event`
   ([workspace_watcher.rs:51](../../crates/app/src/watchers/workspace_watcher.rs#L51)) only matches
   moon configs and project directories. Add, routed through the existing `reset_projects`
   ([workspace_watcher.rs:195](../../crates/app/src/watchers/workspace_watcher.rs#L195)):

   | Event path                                                     | Source of the name set                                         | Why                                  |
   | :------------------------------------------------------------- | :------------------------------------------------------------- | :----------------------------------- |
   | a file named in any loaded toolchain's `manifest_file_names`   | toolchain registry on the session (`load_all`, metadata)       | layer 1 input, and layer 2 detection |
   | a file named in any loaded toolchain's `lock_file_names`       | same                                                           | layer 2 detection                    |
   | a file named in any `.moon/tasks` config's `inheritedBy.files` | `session.tasks_config` entries                                 | layer 2                              |
   | a path in the index's `plugin_input_paths`                     | the builder returned by the last rebuild (or the loaded index) | layer 1 input                        |
   | a project directory created or removed (already handled)       | `is_a_project_root`                                            | new or removed node                  |

   Without this the CLI still validates correctly on its own; the daemon just would not pre-warm for
   the most common per-node edit, a `package.json` change. The name sets are small and change only
   on config reloads, so compute them once per `reset_*` and keep them on the watcher.

3. **Config reloads stay as they are.** `reset_workspace`, `reset_toolchains`, `reset_extensions`,
   and `reset_tasks` already reload the config into the session and choose between `rebuild_context`
   and `rebuild_graphs`. A graph rebuild after a `.moon/tasks` change is a global-hash miss, so
   every node is rebuilt, which matches v1 of the matrix. Phase 3 narrows that inside the builder;
   the watcher needs no change for it.
4. **Daemon and CLI concurrently.** Covered by [Two writers](#two-writers). Both are correct for
   what they saw; the lock keeps index writes whole.
5. **Long-lived graphs and GC.** `DaemonState` holds an `Arc<WorkspaceGraph>` between events. In
   phase 2 a lazily materialized node reads its object on first access; if GC evicted it in the
   meantime, materialization rebuilds the node in process (same path as a corrupt object), and
   touching used objects makes that rare. In phase 1 the daemon's graph is fully loaded at build
   time, so eviction can't affect it.
6. **`CleanCache` RPC.** Goes through `Storage::clean`, which now sweeps `graph/`. Nothing to add.
7. **Abort on event bursts.** The watcher already debounces (500 ms) and aborts the previous build.
   With per-node writes, a burst of edits to 50 projects that interrupts the build 3 times still
   converges: each attempt stores the nodes it finished, and the last one stores the rest and the
   index.

## Phases

### Phase 1: partial rebuild and rewrite, CAS-backed

Deliverable: editing one project rebuilds one project and writes one object; adding a project no
longer rebuilds the world; the daemon warms the same store without deleting anything. Reads still
load every clean payload (parallel small reads).

1. `graph/` store in `LocalStorage`; `KeyedStore` and the three optional `StorageBackend` methods;
   `Storage::{load,store,touch}_graph_object` (local only); the TTL sweep in `gc`.
2. `Project.root` skipped in serde and derived on load; a test that payload bytes contain no
   workspace root path.
3. Index and payload types, the three fingerprints, node and index keys, in `workspace_cache.rs`.
   `WorkspaceGraphCacheState`, `STATE_GRAPH_FILE_NAME`, and `STATE_CACHE_FILE_NAME` removed.
4. `projects_builder.rs`: `load` split into `locate_sources` and `load_build_data_for(dirty)`;
   `build_graph` stops resolving alias ids before storing; payload production after `build_project`
   and before `extract_tasks`; `extract_tasks` unchanged in phase 1.
5. `new_with_cache` becomes the [load algorithm](#load-algorithm), with cache mode checks.
6. Daemon changes 1 and 2.
7. Tests, bench shapes, CHANGELOG bullet.

### Phase 2: lazy payload loading

Deliverable: `moon run a:build` reads the index plus the objects reachable from `a:build`;
`moon project a` reads one object.

1. Summaries in `ProjectNodeEntry` (produced from the built project in step 7 of the load).
2. `TaskDepsBuilder` takes a `TaskSummary` plus entry data instead of `&mut Task` and
   `&mut Project`; it returns resolved deps and implicit project deps rather than mutating. The
   `WorkspaceTasksQuerent` reads the same summary maps.
3. `ProjectGraph` and `TaskGraph` gain a `materialize(id | target)` path: on first `get_unexpanded`,
   load the object, derive `root`, patch `project.dependencies` with the implicit deps and each
   task's `deps` with the resolved deps from assembly, then insert the node. The expanded-project
   caches (`projects`, `tasks` scc maps) are unchanged.
4. `enforce_constraints` reads entries. `get_all` / `get_many` materialize in parallel, reusing
   `map_in_parallel`.
5. A payload that fails to load at materialization time (evicted, corrupt) is rebuilt in process
   through the same `build_project` call, which needs the builder context to stay reachable from the
   graph (an `Arc<WorkspaceBuilderContext>` on `GraphExpanderContext`, or a rebuild callback).

### Phase 3: precision and speed

- **Per-file `.moon/tasks` hashing.** Hash each tasks config twice from its parsed form: the
  `inheritedBy` block (⇒ global, since it decides which projects match) and the rest (⇒ per file).
  Nodes carry `inherited_from` already, so a node's partial fingerprint includes the "rest" hashes
  of exactly the files it inherited from. Adding or removing a tasks file stays global.
- **Persisted stat cache.** `path → (len, mtime_ns, ctime_ns, hash)` in the index, with git's racy
  guard: when a file's mtime equals or exceeds the index's own write time, re-hash it. Turns step 4
  into stats only. [WORKSPACE.md](./WORKSPACE.md) lists persisted hashes under "Not recommended"
  because of racily clean files; the guard is the standard answer, but it needs a test that edits a
  file within the same second as the index write.
- **Payload trimming.** Skip default `Project.config` fields (0.5 of the project bytes). Consider a
  binary payload format (postcard): WORKSPACE.md's objection (`NodeState`'s `deserialize_any`) no
  longer applies, as payloads hold a `Project` directly, not a `NodeState`.
- **Drop `in_docker`** from the global hash once the path-free test exists.
- **Plugin metadata without instantiation** (WORKSPACE.md F2): the global and plugin layers need
  every plugin's version, which today means `load_all`. Composes with this design unchanged.
- **Optional pdk change.** A per-project `extend_project` hook for outputs that need only the
  project's own files (alias, inferred tasks), leaving `extend_project_graph` for cross-project
  facts (implicit deps). Only then does the plugin cost become proportional too.

## Tests

Extend the `cache` module in
[project_graph_test.rs](../../crates/project-graph/tests/project_graph_test.rs) and
`serializes_graph_for_the_cache` in
[tasks_builder_test.rs](../../crates/workspace/tests/tasks_builder_test.rs). The tc-tier1 WASM
plugin's `extend_project_graph` marker file and per-project `tc.cfg`/`tc.lock` reads are the
existing hooks for plugin-call and input-file assertions.

Storage:

- Objects land under `.moon/cache/graph/`, nothing graph-related under `states/`; the index object
  exists at the deterministic key; a rerun reuses every object (no plugin calls, no new objects,
  object mtimes unchanged apart from touches).
- One `moon.yml` edit creates exactly one new node object and rewrites only the index.
- Evicted object, corrupt object, payload key mismatch ⇒ that node rebuilt and re-stored, others
  untouched.
- `MOON_CACHE=read` writes nothing; `off` reads and writes nothing; `write` never reads.
- Shared worktree cache: a second sandbox pointed at the same shared dir reuses node objects and
  writes its own index.
- Payload bytes contain no absolute path (search the bytes for the sandbox root).
- `clean --all` with a short lifetime evicts graph objects; a load after it rebuilds and re-stores;
  an index older than the lifetime is also gone and the next load is a full miss, not an error.
- Two concurrent loads of one sandbox converge on one index and one object set.

Invalidation (each row of the matrix that isn't covered above):

- Add, remove, and rename (`id:`) a project; the root project appearing and disappearing.
- Manifest edit with and without an alias change; assert which dependents are rebuilt.
- `persistent` and `runInCI` changes on a dependency surface the `TaskDepsBuilder` error from
  assembly without rebuilding the dependent.
- Lockfile added ⇒ node rebuilt.
- `.moon/tasks` edit ⇒ all rebuilt (v1); per-file in phase 3.
- Plugin version change ⇒ all rebuilt (use the tc-tier plugins' version field).

Daemon:

- Unit tests for the `on_file_event` → `reset_*` mapping, including the new manifest, lockfile,
  `inheritedBy`, and plugin-input names.
- An integration test that a `moon.yml` edit under the daemon stores exactly one new node object and
  rewrites the index, with no deletions.

Determinism and shape:

- Build the same sandbox twice, and once from the cache, and assert identical node order and
  `to_dot` output (WORKSPACE.md F17 asked for this; it matters more now that assembly orders by id).
- `only_caches_whats_required_to_finalize` becomes an index and payload shape assertion.

Bench: add `load_graphs_cached` (warm, nothing dirty) and `load_graphs_cached_one_dirty` (one
`moon.yml` touched between iterations) shapes to
[workspace_graphs.rs](../../crates/workspace/benches/workspace_graphs.rs) at 1000 projects, with
context creation outside the timed region.

## Acceptance numbers

Targets at 1000 trivial projects, release bench (estimates, to be replaced by measurements in phase
1):

| Scenario                 | Today                        | Phase 1 target                                    |
| :----------------------- | :--------------------------- | :------------------------------------------------ |
| warm, nothing dirty      | ~142 ms (debug, directional) | ≤ full rebuild; dominated by step 4 stats + reads |
| one `moon.yml` dirty     | full rebuild + full rewrite  | warm cost + one `build_project` + one write       |
| one `package.json` dirty | full rebuild + full rewrite  | warm cost + plugin phase + one build + one write  |
| add one project          | full rebuild + full rewrite  | warm cost + plugin phase + one build + one write  |
| `.moon/tasks` edit       | full rebuild + full rewrite  | full rebuild + N small writes (v1)                |

Phase 2 changes the warm row for targeted commands to "index read + reachable objects".

## Risks and open questions

- **Node index determinism.** Assembly inserts in sorted id order, so `NodeIndex` values change
  versus today's completion-order graphs. Anything that tie-breaks by index (`walk_indices`) may
  reorder output and churn snapshots once. Nothing should persist a `NodeIndex`; audit
  `ProjectNode.index` / `TaskNode.index` consumers.
- **Windows small-file reads.** 2000 object reads on NTFS can cost hundreds of ms (estimate).
  Mitigations: pooled parallel reads (phase 1), lazy loading (phase 2). The CAS layout rules out a
  packed single file, which the pre-CAS version of this design had as a fallback; this is the one
  escape hatch the CAS requirement removes.
- **Object churn.** Every global-layer change leaves a dead generation of node objects until GC. At
  2000 projects × ~15 KB (estimate) that is ~30 MB per generation; the TTL sweep bounds it. If it
  matters, the builder can delete the previous generation's keys it just replaced, except under the
  shared worktree cache, where another checkout may still use them, so it would have to be gated on
  `cache_shared_dir.is_none()`.
- **Index size** (phase 2). Task summaries scale with task count. If the index exceeds a few MB in
  real workspaces, move summaries for tasks without deps into payloads and keep only targets and
  options.
- **Plugin phase still whole-graph.** A manifest edit re-reads every manifest inside the plugin. The
  diff limits what moon rebuilds, but only the phase-3 pdk refinement makes the plugin cost
  proportional too.
- **Glob-based toolchain detection** in layer 2 walks the project dir for toolchains that declare
  glob-like `config_file_globs`. Verify which bundled plugins do before deciding to probe (walk per
  project per load) or to skip (document the gap).
- **`extends` in project configs** pointing at files outside the project, or at URLs, are not
  fingerprinted today either. Unchanged gap; note it where layer 2 is documented.
- **The `KeyedStore` trait change.** Touches `StorageBackend`, which remotes implement. The defaults
  keep them compiling, but it's a public-ish surface; the inherent-method alternative is noted
  above.
- **Index key and `moon clean` without `--all`.** `clean_stale_cache(lifetime, all=false)` only
  sweeps `locks/`; graph objects are only swept with `--all`, like blobs. That matches "objects are
  cache", but means a plain `moon clean` no longer resets the graph cache the way deleting `states/`
  did. If a reset is wanted, add it to `--all` only, or add a `--graph` flag.
- **Partial loads** (`load_graphs_for`, test-only) stay on the uncached path, as today.

## Files touched (phase 1)

| File                                                                                                                                                                                                                                    | Change                                                                                                                  |
| :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------- |
| [local_storage.rs](../../crates/cache-local/src/local_storage.rs)                                                                                                                                                                       | `graph` store, keyed-object methods, TTL sweep in `gc`                                                                  |
| [storage_backend.rs](../../crates/cache-storage/src/storage_backend.rs)                                                                                                                                                                 | `KeyedStore`, three optional methods with defaults                                                                      |
| [storage.rs](../../crates/cache-storage/src/storage.rs)                                                                                                                                                                                 | `load_graph_object`, `store_graph_object`, `touch_graph_object` (local only)                                            |
| [workspace_cache.rs](../../crates/workspace/src/workspace_cache.rs)                                                                                                                                                                     | index and payload types, three fingerprints, node and index keys, read/write through `Storage`; state type removed      |
| [workspace_builder.rs](../../crates/workspace/src/workspace_builder.rs)                                                                                                                                                                 | `new_with_cache` becomes the load algorithm; cache mode; `STATE_*_FILE_NAME` removed                                    |
| [projects_builder.rs](../../crates/workspace/src/projects_builder.rs)                                                                                                                                                                   | locate / validate / load-dirty split; no alias mutation in `build_graph`; payload production; `root` derivation on load |
| [project.rs](../../crates/project/src/project.rs)                                                                                                                                                                                       | `root` skipped in serde                                                                                                 |
| [workspace_watcher.rs](../../crates/app/src/watchers/workspace_watcher.rs)                                                                                                                                                              | no deletes; manifest, lockfile, inherited-file, and plugin-input events                                                 |
| [session_atomic.rs](../../crates/app/src/session_atomic.rs)                                                                                                                                                                             | unchanged shape                                                                                                         |
| [project_graph_test.rs](../../crates/project-graph/tests/project_graph_test.rs), [tasks_builder_test.rs](../../crates/workspace/tests/tasks_builder_test.rs), [workspace_graphs.rs](../../crates/workspace/benches/workspace_graphs.rs) | tests and bench shapes                                                                                                  |
| `CHANGELOG.md`                                                                                                                                                                                                                          | "Workspace graph" internal bullet; note that the graph cache moved out of `states/` into `.moon/cache/graph`            |

## Relationship to WORKSPACE.md

| Finding                                     | Status under this design                                                                                                             |
| :------------------------------------------ | :----------------------------------------------------------------------------------------------------------------------------------- |
| F1 parse configs only on a miss             | Subsumed: step 4 parses only dirty and new projects, and final ids for unchanged sources come from the index.                        |
| F2 plugin metadata without instantiation    | Independent and still valuable: the global and plugin layers need every plugin's version.                                            |
| F6 cache file payload and I/O               | Subsumed by per-node objects, path-free payloads, and phase 3 trimming.                                                              |
| F9 write order and bad cache as a miss      | Subsumed: objects before index, and every object is independently verifiable.                                                        |
| F15 tag-scoped task deps without edges      | Unchanged; the decision stays open and is an assembly-only change.                                                                   |
| F16 double digest on plugin input discovery | Subsumed: the index is written once, at the end, and `plugin_input_paths` only widen the next run's layer 1.                         |
| F17 determinism test                        | Required now, since assembly orders by id.                                                                                           |
| "Not recommended: persisting file hashes"   | Revisited in phase 3 with the racy guard; still optional.                                                                            |
| "Not recommended: binary cache format"      | The objection (`NodeState` + `deserialize_any`) doesn't apply to payloads, which hold a `Project` directly. Still optional, phase 3. |

## Rejected alternatives

- **Caching individual task nodes.** Tasks are built per project from one inheritance chain and
  invalidate together; separate objects would multiply the object count by the task count for no
  finer invalidation.
- **Consumed-fact tracking for aliases, tags, and dependency task options** (record what each node
  read from other nodes, validate those facts on load). Correct, but it needs the cross-node facts
  of dirty nodes before they're rebuilt, which forces an ordering problem through cycles across
  scope partitions. Moving alias resolution to assembly and recomputing task deps makes the facts
  unnecessary.
- **Storing graph objects in `manifests/`.** Immortal under `evict_manifests`, see above.
- **Content-addressed payloads in `blobs/` with pointer objects.** Two files per node and a new GC
  root kind for no dedupe, see above.
- **Keeping the index in a plain file under `.moon/cache/`.** Works, but it's the `states/` idea by
  another name, and the keyed store already gives atomic writes and GC for free.
- **A packed payload file with an offset table.** Would help Windows, but it isn't the CAS, and it
  needs compaction and its own locking.
- **Having the daemon serve graphs to CLI clients.** Would remove step 4 from the CLI's path
  entirely, but it's a protocol and lifecycle change far beyond caching; this design leaves the CLI
  self-sufficient and the daemon as a warmer, which is what it is today.
