# Action graph builder: rewrite plan

A plan for replacing the action graph builder with a new implementation, written from scratch, that
produces the same nodes and edges under the same conditions. Written on 2026-10-15 against
`3.0-ag-new` at `20de2c6867`. Line references are as of that commit, and will drift. Links to the
previous builder are permalinks to that commit, as phase 5 deleted it.

[docs/ACTION_GRAPH.md](../ACTION_GRAPH.md) is the specification of the nodes, edges, and conditions
that must be preserved, and this plan doesn't repeat it. Estimates are marked as such. The phase 0
decisions were made on 2026-10-15 (see [Decisions](#decisions)). Phases 1 and 2 were done the same
day: the harness, run against the current builder (see [Baseline](#baseline)), and the new builder's
task stages (see [Phase 2 results](#phase-2-results)). Phase 3, provisioning and project syncs, was
done on 2026-10-16 (see [Phase 3 results](#phase-3-results)), which completes the new builder. Phase
4, the switch, and phase 5, deleting the previous builder, were done the same day (see
[Phase 4 results](#phase-4-results) and [Phase 5 results](#phase-5-results)). Phase 6, storing the
built graph directly, was done on 2026-10-16 too, as a separate change (see
[Phase 6 results](#phase-6-results)), which completes the plan.

## Summary

The current builder
([action_graph_builder.rs](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs))
is one struct of about 2,200 lines (excluding tests) that interleaves task selection, gating, plugin
queries, node creation, edge linking, and edge repair, while inserting nodes one at a time. Much of
its complexity exists to make insertion order not matter: affected state is tracked up front,
`--no-dependencies` edges are linked at the end, serial edges are removed when a later dependency
contradicts them, recursion carries path-local cycle sets, and some node types re-link on reuse
while others don't.

The new builder takes declarative requests, and builds in 4 stages with explicit intermediate data:

1. **Select** which task instances exist, and how they relate. Sync, no plugin calls.
2. **Resolve** which provisioning and sync actions exist. Plugin facts are gathered concurrently up
   front, then resolved synchronously.
3. **Assemble** nodes from typed keys, and edges from one rule function per edge kind.
   Deterministic, insertion order doesn't exist.
4. **Order and validate**: serial ordering last, cycle detection, then freeze into an `ActionGraph`.

Verification is a differential harness that runs every existing builder test, plus generated
workspaces, against both builders, and compares canonical graphs.

## Why rewrite

1. **One struct, many jobs.**
   [The builder's fields](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L147)
   hold affected tracking, the query, changed files, deferred `--no-dependencies` work, ignored
   dependencies, passthrough and primary targets, serial edge bookkeeping, and cleanup and wait
   targets, all mutated during insertion.
2. **Insertion order leaks into the result**, and is compensated for in 5 places:
   - affected state is tracked before insertion
     ([`track_affected`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L342),
     [`run_tasks`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L630)),
     as marking lazily starved transitive dependents;
   - `--no-dependencies` edges are deferred to `build()`
     ([`link_present_dependencies`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L1702));
   - serial edges are skipped when they'd cycle
     ([`try_link_edge`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L2184)),
     and removed newest-first when a later dependency contradicts them
     ([`unlink_serial_edges`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L2131));
   - [`internal_setup_toolchain`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L1527),
     [`internal_setup_environment`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L1362),
     and
     [`internal_sync_project`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L1622)
     carry path-local cycle sets;
   - node reuse is inconsistent:
     [`insert_node_if_missing!`](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph_builder.rs#L30)
     re-links edges, `insert_node_or_exit!` doesn't, so `SyncProject` edges are still decided by
     whichever caller reached the project first.
3. **Plugin calls are interleaved with graph mutation.** Every insertion awaits plugin functions
   (`locate_dependencies_root`, `define_requirements`, `has_func`, tier checks) while holding
   `&mut self`, through `Box::pin` async recursion. Results are memoized per input
   (`cache_func_with`), so nothing is called twice, but nothing runs concurrently either, and the
   control flow is hard to follow.
4. **Edge semantics are implicit.** Edges are passed around as `Vec<Option<NodeIndex>>` with "first
   available" and "fallback" rules, the install slot returns one of 3 node types, and nodes are
   inserted only to carry requirement edges.
5. **Dedupe is expensive.** `ActionNode`'s `Hash`
   ([action_node.rs:253](../../crates/action/src/action_node.rs#L253)) writes `label()`, which
   formats a new `String` on every lookup.
6. **The built graph is indirect.** It's a `Dag<NodeIndex, _>` whose weights index into a
   `FxHashMap<NodeIndex, ActionNode>`
   ([action_graph.rs:12](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph.rs#L12)),
   and `get_node_index` is a linear scan
   ([action_graph.rs:185](https://github.com/moonrepo/moon/blob/20de2c686770d6e833c7252d5eaccd1751d3cc0d/crates/action-graph/src/action_graph.rs#L185)).
   Fixed in phase 6.
7. **There was no benchmark** for building the action graph, so its cost was unknown. Phase 1 added
   one (see [Baseline](#baseline)).
8. **Deep dependency chains overflow the stack.** Insertion recurses through boxed futures once per
   task dependency and project dependency, and polling nests just as deep. A chain of 1,000 projects
   (one task each, depending on `^:build`) overflows both an 8MB main thread and a 2MB tokio worker,
   which is where starbase runs every CLI command, while 800 fits on both. Found by the phase 1
   benchmark, in the bench profile, and not verified against a release build of the CLI.

## The contract: what must not change

- **Nodes, identities, edges, and conditions**: everything in
  [docs/ACTION_GRAPH.md](../ACTION_GRAPH.md), including the install slot resolution table, required
  environments, the task gates and their order, the dependency type mapping (optional → required,
  cleanup reversed), serial ordering anchors (`prev` for persistent, `prev_standard` otherwise),
  present-dependency edges, affected gating (dependencies skip it), task selection (dependents walk
  skips cleanup edges, `--query` bounds it, partitioning after the affected filter), builder
  options, and the entry point behavior of each command.
- **Payload types**: `ActionNode` and its node structs, their labels (which appear in snapshots,
  `moon action-graph --dot|--json`, run reports, and webhooks), and `RunTaskNode.id` (used to stop
  the same action running in parallel).
- **What the pipeline reads**: edge weights (`Required`, `Wait`, `Cleanup`; never `Optional`),
  `cleanup_indices`, priorities, and every `ActionContext` field: primary targets, passthrough
  target states, ignored dependencies, cleanup and wait targets, changed files, and the affected
  state, including the marks made while gating tasks.
- **Errors**: the `ActionGraphError` variants and messages
  ([action_graph_error.rs](../../crates/action-graph/src/action_graph_error.rs)). Tests assert the
  `MissingToolchainRequirement` text, the job and plan errors, and `WouldCycle` naming a specific
  edge.
- **What commands read back**: the partition (`NodeIndex → Target`, and size) that `exec` prints and
  stores ([exec.rs:446](../../crates/app/src/commands/exec.rs#L446)), whether `setup_toolchain`
  inserted a node (`moon setup` counts them), and the node count.

## Decisions

These were artifacts of the current construction order, not part of the spec. They were decided on
2026-10-15 (phase 0), and they define what "equivalent" means for the differential harness: the new
builder may only produce a different graph where one of these decisions says so.

| #   | Behavior                                                                                             | Today                                                                                                             | Decision                                                                                                                                                                                                           |
| :-- | :--------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `SyncProject` dependency edges (ACTION_GRAPH.md, Notes)                                              | Linked only by the first caller to insert the node                                                                | Computed from the full set: once the synced projects are known, every synced project links each synced direct dependency                                                                                           |
| 2   | Cyclic project dependencies (cycles across scopes in the unioned project graph)                      | The project reached first gets the edge                                                                           | Production first: add production and peer edges first (that partition is acyclic), then development, build, and root edges in sorted order, skipping any that would close a cycle                                  |
| 3   | Serial ordering conflicts                                                                            | Serial edges are linked during insertion, and the newest is removed when a later dependency contradicts it        | Serial edges are added after every real edge exists, in declared order, skipping any that would cycle. Nothing is removed. Differences are limited to conflict cases, checked against the `serial_conflicts` tests |
| 4   | Mutual `for_setup_toolchain` requirements (2 toolchains that require each other)                     | Skipped along one recursion path, but a later request can fail with `WouldCycle`, depending on order (unverified) | Skip a requirement edge that would close a requirement cycle, in sorted order, which matches the intent of the path-local skip                                                                                     |
| 5   | Placeholder nodes for requirement-only toolchains (ACTION_GRAPH.md, Notes)                           | Inserted                                                                                                          | Keep                                                                                                                                                                                                               |
| 6   | Different `RunRequirements` per call                                                                 | Each call can pass its own; every command passes one per build                                                    | One `RunRequirements` per build, set on the builder                                                                                                                                                                |
| 7   | Persistent gate when a task is reached in several ways                                               | Decided per call, by the call's origin                                                                            | Union semantics: an instance exists if any path admits it, and it's primary only if a primary path admits it. Same results, verified with the `persistent_in_ci` tests                                             |
| 8   | Which edge `WouldCycle` names                                                                        | The edge being linked when the cycle formed                                                                       | `WouldCycle` for the first edge, in assembly order, that closes the cycle, with the full cycle chain appended to the message. Existing `contains` assertions keep passing                                          |
| 9   | `SetupProto` doesn't link `SyncWorkspace`, `moon sync` ignores `pipeline.*` (ACTION_GRAPH.md, Notes) | As described                                                                                                      | Keep for parity, and revisit separately, outside the rewrite                                                                                                                                                       |

## Design

### Requests

The builder stores requests, and only does work in `build()`, so the order requests are made in
doesn't matter:

```rust
let mut builder = ActionGraphBuilder::new(app_context, workspace_graph, options);

builder.set_requirements(reqs); // one per build (decision 6)
builder.set_query(query)?;
builder.set_changed_files(changed_files);
builder.track_affected(depth, ci_check);

let run = builder.run_tasks_with_plan(&plan); // returns a RequestId
builder.sync_workspace();

let output = builder.build().await?; // BuildOutput { context, graph, report }
let partition = output.report.partition(run);
```

| Request                        | Replaces                                                           | Used by                      |
| :----------------------------- | :----------------------------------------------------------------- | :--------------------------- |
| `RunTasks(locators\|plan)`     | `run_tasks`, `run_tasks_with_plan`, `run_task_by_target(_locator)` | `exec`, `action-graph`       |
| `RunTask(task)`                | `run_task`                                                         | `action-graph`, `sync`       |
| `SyncWorkspace`                | `sync_workspace`                                                   | `exec` in CI, `sync`, MCP    |
| `SyncProject(id)`              | `sync_project`                                                     | `sync`, `sync projects`, MCP |
| `SetupProto`                   | `setup_proto`                                                      | `setup`                      |
| `SetupToolchain(spec)`         | `setup_toolchain`                                                  | `setup`                      |
| `SetupEnvironmentRoot(spec)`   | `setup_environment_root`                                           | `setup`                      |
| `InstallDependencies(project)` | `install_dependencies_by_project`                                  | `docker setup`               |

`build()` becomes async and fallible. The test-only entry points (`run_task_with_config`,
`run_task_dependencies`, `install_dependencies_root`, project-scoped `setup_environment`) become
requests too, or test helpers.

### Stage 1: select

Turns the task requests into a `TaskPlan`. It reuses the current resolution logic (locators, tags,
the default project, `--query`, dependents expansion, affected tracking, job partitioning),
extracted from the builder into free functions.

```rust
struct TaskInstanceKey {
    target: Target,
    args: Vec<String>,
    env: Vec<(String, Option<String>)>, // sorted by key
}

struct TaskPlan {
    instances: IndexMap<TaskInstanceKey, TaskInstance>, // task, flags, priority, admission
    relations: Vec<TaskRelation>,                       // from, to, Required | Wait | Cleanup
    serial: Vec<SerialChain>,                           // parent, required deps in declared order
    primaries: Vec<TaskInstanceKey>,
    passthrough: FxHashSet<Target>,
    ignored_dependencies: FxHashMap<Target, FxHashSet<Target>>,
    cleanup_targets: FxHashSet<Target>,
    wait_targets: FxHashSet<Target>,
    partition: PartitionInfo,
}
```

- With one `RunRequirements` per build, the interactive and persistent flags are a function of the
  target, so they don't need to be in the key, while labels stay the same.
- The dependency closure is a worklist that applies the gates in the same order as today. Union
  semantics (decision 7) make the result independent of the order the worklist runs in.
- `--no-dependencies` is no longer deferred: once the closure is known, each instance's non-cleanup
  dependencies are filtered to the instances that exist, and the rest are recorded as ignored.
- Serial chains are recorded as data, not edges.
- Affected checks go through the same `AffectedTracker` calls as today, in a deterministic order, so
  `ActionContext.affected` gets the same marks.
- Project syncs are walked here too, at the point where a task is inserted, as their affected checks
  must happen in the same order as the task checks: marking a task affected also marks its project,
  so checking projects after every task could sync dependency projects that are skipped today. The
  walk needs no plugin calls, and uses an explicit stack, like tasks.
- Everything is recorded as an event log, in the order that nodes are inserted and edges are linked:
  workspace and project syncs, entering a task (before its dependencies), exiting a task (after
  them), and provisioning requests.

### Stage 2: resolve

Decides which `SyncWorkspace`, `SetupProto`, `SetupToolchain`, `SetupEnvironment`,
`InstallDependencies`, and `SyncProject` nodes exist.

**As built, this differs from the original design.** Plugin facts aren't gathered up front. Instead,
the assemble stage replays the event log, and resolves each task's toolchains (the install slot
table, environment setup with its requirements and required environments, toolchain setup with its
requirements, and the proto rule) right before inserting the task's node, through the toolchain
registry, as the current builder does. The registry memoizes every plugin call by its input, so
nothing is called twice. This was chosen because:

- the walks that overflowed the stack (task dependencies and project syncs) are iterative in stage
  1, while toolchain requirements recurse only a few levels;
- it ports the provisioning logic almost line by line, which keeps the risk of behavior changes low;
- it keeps the node order the same as today's, so most snapshots won't change at the switch;
- building is already fast (see [Phase 3 results](#phase-3-results)), so concurrent plugin calls
  aren't needed.

Requirements are read exactly as they are today: `requires` with `for_setup_toolchain` and
`for_setup_environment`, and required environments regardless of the flags (see the `SetupToolchain`
and `SetupEnvironment` sections of [ACTION_GRAPH.md](../ACTION_GRAPH.md)).

### Stage 3: assemble

```rust
enum NodeKey {
    SyncWorkspace,
    SetupProto,
    SetupToolchain(ToolchainSpec),
    SetupEnvironment { toolchain: Id, root: WorkspaceRelativePathBuf, project: Option<Id> },
    InstallDependencies { toolchain: Id, root: WorkspaceRelativePathBuf, members: Option<Vec<String>>, project: Option<Id> },
    SyncProject(Id),
    RunTask(TaskInstanceKey),
}
```

- `NodeKey` derives `Hash` and `Eq`, so lookups don't format strings. Keys are mapped to their
  `ActionNode` payloads when freezing.
- Edges are collected into a set of `(from, to, TaskDependencyType, EdgeReason)`, deduped by
  `(from, to, weight)`. `EdgeReason` (workspace, proto, toolchain requirement, environment
  requirement, required environment, install slot, project dependency, task owner, task slot,
  fallback, task dependency, cleanup, serial, present dependency) is for diagnostics, and doesn't
  reach the pipeline.
- There's one rule function per edge kind, named after its section in ACTION_GRAPH.md, iterating
  keys in sorted order.
- `SyncProject → SyncProject` edges are computed from the full set of synced projects, production
  and peer edges first, then the other scopes in sorted order, skipping any that would close a cycle
  (decisions 1 and 2).
- Nodes are added in the order the current builder inserts them, by replaying the event log: a
  task's project sync and toolchain actions before the task, and the task before its dependencies.
  So node indices are the same as today's.
- Relations are added in the order the current builder links them (a task's requirements once its
  dependencies have been visited, then its cleanups), so a cycle is reported for the same edge.

### Stage 4: order and validate

1. **Serial edges** are computed against the assembled graph: walk each dependency's subtree over
   `Required` edges between `RunTask` nodes, and add edges to the anchor, in chain order. An edge
   that would cycle is skipped. Since every real edge already exists, nothing is ever removed
   (decision 3). Serial links are applied in the order the current builder makes them, and the
   subtree walk skips serial, present dependency, cleanup, and wait edges.
2. **Preference edges** (serial, required environments, and requirement edges that would close a
   requirement cycle, per decision 4) are the only ones that may be skipped. Any other cycle is an
   error, reported as `WouldCycle` for the first edge that closes it, with the cycle chain appended
   (decision 8).
3. **Freeze** into an `ActionGraph`, `ActionContext`, and a `BuildReport` (each request's node
   indices, and the partition).

### Module layout

`crates/action-graph/src/builder/`: `mod.rs` (requests, the public API, the specs, the report, and
the context), `select.rs` (stage 1), and `assemble.rs` (stages 2 to 4: replaying the events,
resolving provisioning, the whole graph edges, and freezing).

The original estimate of 1,600 to 2,000 lines for the whole builder was low: phase 2 alone is 1,781
lines with doc comments, about a third of which is the task resolution logic, ported as is. The full
builder will likely be about the size of the current one (an estimate). The gain is structure and
determinism, not size: no repair logic (serial edge removal, deferred linking, cycle sets, or mixed
reuse rules), no recursion, and no graph mutation while selecting.

## Verification

1. **Canonical form.** Sorted nodes (by `NodeKey`, which includes args and env, as labels collide
   for dependency variants), sorted edges (`from`, `to`, weight), cleanup nodes, and every context
   field, sorted. Two builds are equivalent when their canonical forms match. Done in phase 1:
   `CanonicalActionGraph` in [tests/utils.rs](../../crates/action-graph/tests/utils.rs), keyed by
   `get_node_key` (the label, plus args and sorted env for tasks, and the project and members that
   labels omit), with `with_partition`, a line-by-line `diff`, and `assert_canonical_eq`. Parallel
   edges with the same weight are merged, as the pipeline merges them, and a key held by more than
   one node is reported in `duplicate_nodes`.
2. **Differential harness.** `ActionGraphContainer`
   ([tests/utils.rs](../../crates/action-graph/tests/utils.rs)) builds with either builder, and
   every test in
   [action_graph_builder_test.rs](../../crates/action-graph/tests/action_graph_builder_test.rs) (224
   tests, 114 snapshots) also runs in a differential mode that asserts both builders produce the
   same canonical form, except for the documented decisions. Phase 2 added the comparison as its own
   test (see [Phase 2 results](#phase-2-results)). Running the integration tests in a differential
   mode comes with the switch.
3. **Snapshots and `topo` assertions.** Node indices change with the new insertion order, which
   changes every dot snapshot and many `topo()` vectors, even for identical graphs. Snapshots are
   regenerated once, at the switch, and accepted only where the differential harness proves the
   graphs equivalent. `topo()` assertions move to canonical comparisons.
4. **Generated workspaces.** A seeded generator (extending `moon_bench_utils`, without a new
   dependency) that produces projects with dependencies (including cycles across scopes), tasks with
   every dependency type, serial parents, persistent and `runInCI` tasks, and `tc-*` toolchains.
   Compare both builders across many seeds, and check invariants: acyclic, every node except
   `SyncWorkspace` and `SetupProto` reaches `SyncWorkspace` when enabled, cleanup and wait edges
   only between tasks, and no edges from setup or sync nodes into tasks. Done in phase 1 for the
   current builder: `create_generated_workspace` in
   [generator.rs](../../crates/bench-utils/src/generator.rs), `find_invariant_violations` in
   [tests/utils.rs](../../crates/action-graph/tests/utils.rs), and
   [action_graph_invariants_test.rs](../../crates/action-graph/tests/action_graph_invariants_test.rs)
   (see [Baseline](#baseline)).
5. **Benchmark.** `crates/action-graph/benches/action_graph_builder.rs`, with `criterion` (the
   CodSpeed compat package) and `moon_bench_utils` (`create_simple_workspace`,
   `create_chained_workspace`), like `crates/affected/benches/affected_tracker.rs`, building every
   task at 100 and 1,000 projects (more locally), for both builders. Done in phase 1 for the current
   builder (see [Baseline](#baseline)).
6. **End to end.** The `moon_cli` tests for `run`, `ci`, `check`, `exec`, `setup`, `sync`, and
   `docker setup`.

## Phases

0. **Decide** the behaviors and open questions. Done on 2026-10-15, see [Decisions](#decisions) and
   [Resolved questions](#resolved-questions).
1. **Harness.** The canonical form, benchmark, and generator, run against the current builder only,
   as a baseline. Done on 2026-10-15, see [Baseline](#baseline).
2. **Tasks.** Stages 1, 3, and 4 for tasks only, behind a separate type, with the harness comparing
   the task subgraph (task nodes and the edges between them). Done on 2026-10-15, see
   [Phase 2 results](#phase-2-results).
3. **Provisioning.** Stage 2, and project sync, with the harness comparing full graphs. Done on
   2026-10-16, see [Phase 3 results](#phase-3-results).
4. **Switch.** Move the commands, MCP, and test utilities to the request API, regenerate snapshots
   (reviewed against the harness), and update ACTION_GRAPH.md. Done on 2026-10-16, see
   [Phase 4 results](#phase-4-results).
5. **Delete** the current builder. Done on 2026-10-16, see [Phase 5 results](#phase-5-results).
6. **Follow-up:** store the built graph directly as `DiGraph<ActionNode, TaskDependencyType>`, which
   touches the action pipeline (`job_dispatcher.rs`, `action_pipeline.rs`), the `action-graph`
   command, and the graph traits. Decided: a separate change, after the switch. The `ActionGraph`
   API stays the same during the rewrite. Done on 2026-10-16, see
   [Phase 6 results](#phase-6-results).

## Baseline

Phase 1, measured on 2026-10-15 against the current builder, on `3.0-ag-new` at `20de2c6867` plus
the harness changes.

### Benchmark

[benches/action_graph_builder.rs](../../crates/action-graph/benches/action_graph_builder.rs), run
with `cargo bench -p moon_action_graph --bench action_graph_builder` on an Apple M1 Pro (10 cores).
Each iteration creates a builder, calls `run_tasks` with every task as a target (`moon run` with
default requirements and options), and calls `build()`. The workspace graph is built, and every task
expanded, once outside the timing, and the app context is shared. The simple workspace has 4 tasks
per project (a chain of 3, plus `build`) and no project dependencies. The chained workspace is one
`build` task per project, with each project depending on the previous one. The 5,000 project case
only runs outside of CI and headless sessions, so it wasn't measured.

| Case                     | Projects | Tasks | Benched (worker) | Main thread (`block_on`) |
| :----------------------- | -------: | ----: | ---------------: | -----------------------: |
| `run_tasks/10`           |       11 |    44 |           191 µs |                   160 µs |
| `run_tasks/100`          |      101 |   404 |          1.63 ms |                  1.48 ms |
| `run_tasks/1000`         |    1,001 | 4,004 |          17.9 ms |                  16.4 ms |
| `chained/run_tasks/10`   |       11 |    11 |            73 µs |                    55 µs |
| `chained/run_tasks/100`  |      101 |   101 |           572 µs |                   503 µs |
| `chained/run_tasks/1000` |    1,001 | 1,001 |          7.72 ms |           stack overflow |

The benchmark spawns every build onto a runtime worker with a 256MB stack, as the 1,000 project
chain overflows the main thread (see [Why rewrite](#why-rewrite), item 8). Spawning measures 9% to
32% slower (the most for the smallest cases) than polling on the main thread, which is the second
column, measured before that change. Compare both builders with the same method. The cost grows
about linearly with the number of tasks: about 4µs per task in the simple workspace, and 5 to 8µs
per task in the chained workspace.

### Generated workspaces

[action_graph_invariants_test.rs](../../crates/action-graph/tests/action_graph_invariants_test.rs)
generates 64 workspaces by default (`ACTION_GRAPH_SEEDS=<count>` for more,
`ACTION_GRAPH_SEED=<seed>` to reproduce one), split across 4 test shards. Each has 3 to 10 projects
and 1 to 5 tasks per project, and is run from the workspace root, a project root, or a dependencies
workspace, as the tier 2 test toolchains locate the dependencies root from the working directory.
Each is built in 13 scenarios modeled on the commands: `run` (plain, `--affected`, `--no-deps` on
every other target, `--query` with `:task` locators, and with workspace sync, project dependency
sync, and installs disabled), `ci` (plain, deep affected depth and dependents, and partitioned into
2 jobs), `check --interactive`, `action-graph`, `sync`, `setup`, and `docker setup`.

- **Every invariant held for every seed and scenario.** No current-builder bug was found by them.
- Every build is also checked for: unique node identities, no `Optional` weights, every cleanup
  index being a task with an outgoing `Cleanup` edge. Each seed also builds the deep affected `ci`
  scenario twice, and requires identical canonical forms.
- A coverage check fails a shard if its seeds stop producing any of: every node type, dependency
  variants, a dependencies workspace (members), cleanup and wait edges, requirement edges between
  `SetupEnvironment` and between `SetupToolchain` nodes, `SyncProject` dependency edges, affected
  marks, ignored dependencies, and passthrough target states. Across the 208 graphs of one shard (16
  seeds), 274 task nodes shared their label with another node (dependency variants), and there were
  1,196 `SyncProject → SyncProject` edges.
- The suite takes about 25 seconds alone. Most of it is loading and dropping the toolchain plugins
  once per seed: dropping compiled plugins is slow on macOS (wasmtime deregisters the unwind info of
  every function), so each seed shares one plugin registry between its workspace graph and its
  builds.

## Phase 2 results

Done on 2026-10-15. The new builder is
[`builder::ActionGraphBuilder`](../../crates/action-graph/src/builder/mod.rs), which isn't used by
any command yet. Until phase 3, it produces the task subgraph only: the graph that the current
builder produces when every builder option is disabled.

### What was built

- **Requests** ([mod.rs](../../crates/action-graph/src/builder/mod.rs)): `run_tasks`,
  `run_tasks_with_plan`, `run_task`, `run_task_by_target`, and `run_task_by_target_locator` record
  requests and return a `RequestId`. `set_requirements`, `set_query`, `set_changed_files`, and
  `track_affected` configure the build, and `mock_affected` replaces the tracker in tests.
  `build().await` returns the graph, the context, and a `BuildReport` (each request's partition or
  nodes).
- **Stage 1** ([select.rs](../../crates/action-graph/src/builder/select.rs)): a port of the current
  task insertion, with the same gates in the same order, and the same affected calls. Dependencies
  are walked with an explicit stack instead of async recursion, so deep chains no longer overflow
  the stack. It records instances, relations, serial links, and context data, without creating nodes
  or edges.
- **Stages 3 and 4** ([assemble.rs](../../crates/action-graph/src/builder/assemble.rs)): nodes, then
  relations (a cycle is an error), then present dependencies (a cycle is a warning), then serial
  edges (a cycle is skipped), then freezing into an `ActionGraph`.
- **Decision 8**: `ActionGraphError::WouldCycle` gained an optional `cycle` help field, which the
  new builder fills with the whole cycle. The current builder passes `None`, so its errors don't
  change.
- `TargetsBlock` derives `Clone` and `Debug`, so that plan requests can be stored.

### Comparison

[action_graph_differential_test.rs](https://github.com/moonrepo/moon/blob/64b2db9817c0868b343fd0b2a229e8c6f8f66d8f/crates/action-graph/tests/action_graph_differential_test.rs)
(deleted in phase 5) builds the same requests with both builders (every option disabled for the
current one), and compares their canonical forms, including the context and partition, and error
messages when both fail:

| Comparison     | Workspaces                                                               | Requests                                                                                                                                                                                                            | Result    |
| :------------- | :----------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | :-------- |
| Generated      | 64 seeds (4 shards)                                                      | 11 shapes: `run`, `ci`, `run --affected`, `ci` with deep affected depth and dependents, `ci --job`, a partitioned plan, `check --interactive`, `--no-deps`, `--query`, `action-graph`, and `action-graph <targets>` | identical |
| Fixtures       | 14 fixtures (all but `tasks-ci-mismatch`, which fails to load by design) | the same 11 shapes                                                                                                                                                                                                  | identical |
| Request orders | the serial and `dep-types` fixtures                                      | every ordered pair of tasks, and every task in reverse order, with and without dependencies                                                                                                                         | identical |

Disabling serial ordering in the new builder makes every comparison fail (32 to 239 differences per
test), so the comparisons are sensitive to real differences.

### Findings

- **Decision 3** (serial edges added last, skipping conflicts) produced no differences in any
  compared case, including every request order of the `serial-conflict` fixture.
- **Decision 7** (union semantics for the persistent gate) holds: the gates run in the same order,
  and produce the same graphs.
- **The stack overflow is fixed** for the new builder: a chain of 1,000 projects builds on a test
  thread (2MB stack).
- **Not compared yet**: provisioning and sync nodes, and the decisions about them (1, 2, 4, 5, and
  9), which come with phase 3 (see [Phase 3 results](#phase-3-results)).

## Phase 3 results

Done on 2026-10-16. The new builder now produces whole graphs, with every option.

### What was built

- **Requests**: `sync_workspace`, `sync_project`, `setup_proto`, `setup_toolchain`,
  `setup_environment_root`, and `install_dependencies_by_project`, plus `get_spec`,
  `get_project_spec`, and `get_workspace_spec`. `moon setup` only sets up a toolchain's root
  environment when the toolchain inserted a node, which a builder that only records requests can't
  branch on, so `setup_toolchain_and_environment` encodes that as one request.
- **Stage 1** records the event log, and walks project syncs with the same gates and affected checks
  as today (see [Stage 1](#stage-1-select)).
- **Stages 2 to 4** ([assemble.rs](../../crates/action-graph/src/builder/assemble.rs)): replay the
  events, resolving provisioning as today, with `NodeKey` identities for provisioning and sync
  nodes. Then link the whole graph edges: project syncs (decisions 1 and 2), present dependencies,
  and serial edges.
- **Decision 4**: requirement edges between toolchain setups are skipped when they would cycle, like
  required environments.
- **Decisions 5 and 9**: kept as today, so nothing changed.

### Comparison

The differential test now builds every case with matching options, in 5 profiles: every action,
none, actions disabled in config, setup only, and project syncs only. Every task request shape runs
with and without actions, plus the provisioning commands: `ci` with a workspace sync, `run` with
actions disabled, `sync`, `sync projects`, `setup`, and `docker setup`. That's 28 cases for each of
the 64 generated workspaces and 14 fixtures, plus the request order comparisons, which now run with
every action.

Every node, every other edge, the context, partitions, and errors are identical. The only
differences are edges between project syncs, as decided. Instead of being compared, those edges are
checked against the decisions:

- an edge that only the new builder has must be a direct project dependency (decision 1);
- an edge that only the current builder has must not be a production or peer dependency, and must
  close a cycle in the new graph (decision 2).

Every difference across every case passed those checks. Removing the `InstallDependencies` edges
from the new builder makes every generated workspace comparison fail (55 to 97 differences per
shard), so the provisioning comparisons are sensitive to real differences.

### Findings

- **Decisions 1 and 2** change graphs whenever projects are synced through more than one caller, or
  depend on each other across scopes, both of which the generated workspaces do often. Fixture
  graphs didn't change.
- **Decision 4** wasn't exercised: no compared case has two toolchains that require each other for
  toolchain setup.
- **Error precedence**: tasks are all selected before any provisioning is resolved, so when a build
  has both a task error and a provisioning error, the task error is reported, even if the current
  builder would have hit the provisioning error first. No compared case hit this.
- **Deep chains** no longer overflow the stack with every action enabled either: project syncs are
  walked with an explicit stack too.

### Benchmark

Both builders, measured the same way as the [Baseline](#benchmark), on 2026-10-16:

| Case                     | Current |     New | Change |
| :----------------------- | ------: | ------: | -----: |
| `run_tasks/10`           |  200 µs |  148 µs |   -26% |
| `run_tasks/100`          | 1.75 ms | 1.19 ms |   -32% |
| `run_tasks/1000`         | 18.5 ms | 13.2 ms |   -29% |
| `chained/run_tasks/10`   |   76 µs |   66 µs |   -13% |
| `chained/run_tasks/100`  |  601 µs |  460 µs |   -24% |
| `chained/run_tasks/1000` | 8.25 ms | 4.35 ms |   -47% |

The first version of the new builder was twice as slow on the 1,000 project chain. Two fixes got it
here: the project sync walk keeps one set of the projects on the current path, instead of cloning
one per project, and production edges between project syncs are added in one batch, with a single
cycle check, instead of one search per edge.

## Phase 4 results

Done on 2026-10-16.

- **API**: `moon_action_graph::ActionGraphBuilder` is the new builder. The previous one was kept as
  `moon_action_graph::legacy::ActionGraphBuilder`, used only by the differential test and the
  benchmark, until phase 5. The shared types (`RunRequirements`, `RunPartition`, `TaskOrigin`, and
  `ActionGraphBuilderOptions`) moved to
  [action_graph_options.rs](../../crates/action-graph/src/action_graph_options.rs), so phase 5 only
  deletes the legacy builder's file.
- **Commands**: `run`, `ci`, `check`, and `exec` (through `exec.rs`), `action-graph`, `sync`,
  `sync projects`, `setup`, `docker setup`, and the MCP sync tools record requests and read results
  from the `BuildReport`. `moon setup` counts the toolchains it set up from the report, after the
  build.
- **Test-only requests**: `install_dependencies(spec, project)`, `install_dependencies_root`,
  `setup_environment(spec, root, project)`, and the debug-only `run_task_with_config` and
  `set_affected`, so that the existing tests exercise the same entry points as before.
- **Tests**: the 224 builder tests, the action pipeline tests, and the invariant test were migrated
  to the request API, mostly by a script. Results that used to be returned by a request (nodes,
  partitions, errors) are now read after the build, and the 12 `should_panic` tests build, so that
  their error surfaces.
- **Snapshots**: 192 builder tests passed with their snapshots unchanged. The 19 builder snapshots
  and 1 CLI snapshot that changed were each checked to contain the same nodes and edges: only the
  order of edges in the dot output changed, as serial edges and edges between project syncs are now
  linked last. Two tests that compared edge lists in order now sort them.
- **Docs**: [ACTION_GRAPH.md](../ACTION_GRAPH.md) describes the new builder, and the changelog has
  the visible changes (decisions 1, 2, and 8, the speedup, and the stack overflow fix).

## Phase 5 results

Done on 2026-10-16.

- **Deleted**: the previous builder (`action_graph_builder.rs`, 2,383 lines), its `legacy` module,
  and the differential test (1,017 lines), which needs both builders. Its job ended at the switch:
  the builder tests, their snapshots, and the invariant test now guard the new builder.
- **Ported tests**: two unit tests that only existed in the previous builder's file, for the path
  sets that guard against cycles, now run against the new builder:
  `setup_toolchain::keeps_requirements_path_local_across_siblings` (two toolchains that both require
  `tc-tier3` each link to its setup), and
  `sync_project::keeps_dependencies_path_local_across_siblings` (two projects that both depend on
  `baz` each link to its sync).
- **Benchmark**: only the new builder is measured, under the previous builder's `run_tasks` name, so
  that CodSpeed compares it with the history of the previous one. Builds are still spawned onto a
  worker, so that the numbers stay comparable with the tables above.
- **Docs**: links to the deleted files in this plan, [TOOLCHAIN_REQS.md](./TOOLCHAIN_REQS.md), and
  [AFFECTED.md](./AFFECTED.md) are permalinks to the commits they were written against.

## Phase 6 results

Done on 2026-10-16, on a branch from `develop-3.0` after the rewrite
([#2768](https://github.com/moonrepo/moon/pull/2768)) merged.

- **Storage**: `ActionGraph` stores a `DiGraph<ActionNode, TaskDependencyType>`, so a node is
  `graph[index]`, instead of a `Dag<NodeIndex, _>` whose weights were each node's own index, plus a
  `FxHashMap<NodeIndex, ActionNode>` of the nodes. `get_inner_nodes` is gone, and `ActionGraph::new`
  takes the graph alone. `get_nodes` returns nodes in index order, instead of the map's order.
- **Builder**: assembles into a `Dag<ActionNode, _>`, which keeps the cycle check on every edge, and
  freezes it with `into_graph()`, so the separate node list and the conversion into a map are gone.
- **`get_node_index`**: a map from node to index, created on first use, instead of a linear scan.
  Only the graph traits (`dependencies_of` and friends) use it, so builds don't pay for it.
- **Graph traits**: `GraphData` has a `NodeWeight` associated type, so a graph's node weights are
  either the nodes themselves (the action graph), or their indices (the project and task graphs,
  unchanged). `get_node_keys` and `to_focused_graph` use node indices instead of node weights, which
  were always the same.
- **JSON**: `to_json` serializes the graph's nodes by index whatever their weights are, so
  `moon action-graph --json` (and the project and task graph equivalents) output the same JSON as
  before. A unit test checks that a graph that stores its nodes inline, and one that stores indices,
  serialize the same, and the same as serializing the indexed graph as is.
- **Pipeline**: the job dispatcher reads nodes from the graph, instead of the node map.
- **Tests**: the builder, invariant, pipeline, project graph, task graph, and graph command tests
  pass with every snapshot unchanged.

### Benchmark

Measured the same way as the [Baseline](#benchmark), against `develop-3.0` before the change, on
2026-10-16. Every case is faster (p < 0.05). This wasn't profiled, but freezing no longer builds a
map of every node, which is probably most of it:

| Case                     |  Before |   After | Change |
| :----------------------- | ------: | ------: | -----: |
| `run_tasks/10`           |  144 µs |  138 µs |    -4% |
| `run_tasks/100`          | 1.18 ms | 1.14 ms |    -3% |
| `run_tasks/1000`         | 12.9 ms | 12.7 ms |    -3% |
| `chained/run_tasks/10`   |   64 µs |   62 µs |    -4% |
| `chained/run_tasks/100`  |  441 µs |  428 µs |    -3% |
| `chained/run_tasks/1000` | 4.19 ms | 4.04 ms |    -3% |

## Risks

- **Hidden order dependence that users rely on.** Graphs that differ only in conflict cases
  (decisions 1 to 3), and `WouldCycle` messages (decision 8), will change. The harness surfaces
  them, and they go in the changelog.
- **Affected state.** Gating marks tasks lazily, and those marks are output through the context.
  Stage 1 must make the same calls, or reports and `affectedFiles` change.
- **Error precedence.** Gathering facts concurrently can change which plugin error surfaces first.
  Report the first error by key, not by completion order.
- **Plugin call volume.** Gathering facts up front can query toolchains whose nodes would have been
  gated out later. Gate before gathering, and compare call counts in the benchmark.
- **API churn.** `build()` becomes async and returns a report: 7 call sites in `crates/app` and
  `crates/mcp`, plus the test utilities.
- **Test churn.** 224 tests and 114 snapshots, most of it mechanical.

## Resolved questions

Decided on 2026-10-15. No questions are open.

1. **API:** request-based. Builder methods record requests and return a `RequestId`, and
   `build().await` returns the graph, the context, and a report (the partition, and each request's
   node).
2. **Requirements:** one `RunRequirements` per build (decision 6).
3. **Project cycles:** production and peer edges first (decision 2).
4. **Serial conflicts:** add serial edges last, and skip conflicts (decision 3).
5. **Cycle errors:** `WouldCycle` for the closing edge, with the chain appended (decision 8).
6. **Graph storage:** a separate change, after the switch (phase 6, see
   [Phase 6 results](#phase-6-results)).
