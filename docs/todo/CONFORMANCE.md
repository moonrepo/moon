# Conformance

Tracks the remaining work to increase moon's conformance with the
[vx conformance suite](https://vznjs.github.io/vx/conformance/) (moonrepo/moon#2818). A case ID
links to its row on the results page, like
[`ex25la`](https://vznjs.github.io/vx/conformance/#ex25la).

The suite's moon adapter (`private/vx-conformance/src/adapters/moon.ts` in `vznjs/vx`) writes
`.moon/workspace.yml` and `moon.yml` only, and configures no toolchain, as its sandbox has no
network. It maps a persistent task to `preset: server` with `runInCI: true`, and a dependant's edge
to it to a `type: wait` dependency.

## Status

The Environment, Cache, and Config sections were audited on 2026-11-21. The other sections (Task
graph, Inputs, Outputs, Failures, Affected, Run report, Isolation) have not been audited yet. Fixed
items are on the `audit-conformance-2818` branch, one commit each, until it's merged into `master`.

| Item                                                                                                    | Cases                                                                | Status                                 |
| ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | -------------------------------------- |
| Unset and empty env inputs hash the same                                                                | `hjbima`                                                             | Fixed                                  |
| Env inputs with lowercase names are file inputs                                                         | `f4a2p6`                                                             | Fixed                                  |
| Glob characters in a project's path, and `PATH` separators                                              | `rqk8s7`                                                             | Fixed (starbase_utils v0.14.13)        |
| A dependency's `bash` or `sh` bin replaces the task's shell                                             | `uqu850` (shell part)                                                | Fixed (starbase_shell v0.14.5)         |
| A failed run blocks hydrating a cached pass                                                             | `dbr6wr`                                                             | Fixed                                  |
| Timeouts greater than a day are ignored, and `0` kills the task                                         | `zqwwd3`                                                             | Fixed                                  |
| [Servers behind `wait` dependencies](#servers-behind-wait-dependencies)                                 | `uqu850`, `4xjgil`, `s0p48p`, `ex25la`, `fkw67u`, `m2vx9r`, `40x9mk` | Planned for 3.0                        |
| [Share the local cache across worktrees by default](#share-the-local-cache-across-worktrees-by-default) | `95qyl3`                                                             | Not yet decided                        |
| [Inputs that change while a task runs](#inputs-that-change-while-a-task-runs)                           | `o2iy1u`                                                             | Not yet decided                        |
| [Cases that only fail without a toolchain](#cases-that-only-fail-without-a-toolchain)                   | `o8neo7`, `l2ggkq`                                                   | No moon change, feedback for the suite |

## Servers behind `wait` dependencies

Cases: [`uqu850`](https://vznjs.github.io/vx/conformance/#uqu850),
[`4xjgil`](https://vznjs.github.io/vx/conformance/#4xjgil),
[`s0p48p`](https://vznjs.github.io/vx/conformance/#s0p48p) (Environment and Cache), and
[`ex25la`](https://vznjs.github.io/vx/conformance/#ex25la),
[`fkw67u`](https://vznjs.github.io/vx/conformance/#fkw67u),
[`m2vx9r`](https://vznjs.github.io/vx/conformance/#m2vx9r) (Task graph), and
[`40x9mk`](https://vznjs.github.io/vx/conformance/#40x9mk) (Failures). They fail with "did not
finish within 30 s" (or 60 s), or "left running: sleep 60".

### Current behavior

These are moon 2.6's documented `wait` semantics (`website/docs/config/project.mdx`, "Wait
dependencies"), reproduced with a server running `sleep 8`:

- **The run lasts as long as the server.** A persistent task that others wait on is dispatched
  without a permit and marked as completed immediately (`ActionPipeline`, the persistent branch of
  the dispatch loop). Once all other jobs finish, the pipeline waits for every child process to exit
  (`wait_for_running_to_shutdown`), so the run only ends when the server does. The docs say a wait
  dependency is "not stopped once this task has completed", and to pair it with a `cleanup`
  dependency to stop it. The adapter can't do that, as stopping the server makes it exit non-zero,
  which fails the pipeline.
- **The server boots for a cached dependant.** A wait dependency is started before the cache of its
  dependant is checked, so it boots even when the dependant replays (a second `boot` line in the
  repro).
- **The dependant isn't keyed on the server.** A wait dependency's `cacheStrategy` is forced to
  `ignored` (`TaskDepsBuilder`, for non-required dependency types), and the persistent job's state
  is set to `TargetState::Passthrough` before it hashes. After editing the server's sources, the
  dependant still replays with the same hash.

### What the cases expect

- The server is up while its dependant runs, and the run ends when the dependant does (`ex25la`,
  `uqu850`).
- The server is asked to stop with a catchable signal, then force killed (`fkw67u`).
- When the dependant fails, the run fails promptly and the server is stopped (`40x9mk`).
- The dependant runs again when the server's sources change (`4xjgil`).
- When every dependant replays from the cache, the server isn't started (`s0p48p`, `m2vx9r`). The
  server "declares it reads no files, so its dependant can be keyed without it": the dependant's key
  uses the server's inputs, without starting it.

### Target

`develop-3.0`, not `master`, in a new worktree off `origin/develop-3.0`:

- It changes documented behavior.
- 3.0 has rewritten the action graph builder, and changed the dispatcher, pipeline, and task runner,
  so a 2.x implementation would be thrown away.
- 3.0 uses starbase_process v0.4 (moonrepo/moon#2819), where `ProcessSet::nested_with_grace(..)` and
  `ProcessSet::shutdown(SignalType::Terminate)` send a catchable signal, then force kill after the
  grace period. That's how a single server is stopped. Today, every task spawns into the root set.

The cases keep failing on 2.x.

### Plan

Three phases, each shippable on its own, as one PR each, in order.

**Phase A: stop servers that nothing needs anymore.** Fixes the hangs (`ex25la`, `uqu850`, `fkw67u`,
`40x9mk`), and step 1 of the other cases.

1. Spawn each persistent task into its own nested `ProcessSet`, with the grace period of
   `pipeline.killProcessThreshold`, and keep a stop handle for it per target (in `ActionContext`).
2. In `JobDispatcher`, count the outstanding wait dependants of each persistent job, and decrement
   when each one finishes (passed, failed, skipped, or hydrated).
3. At zero, stop the server, unless it's a primary target (`moon run api:dev web:e2e`), or another
   persistent task depends on it.
4. A stop that moon requested is a success (for example a new `Stopped` status), whatever the exit
   code of the server, like 143 after a `SIGTERM`.
5. Docs: replace "is not stopped once this task has completed" and the advice to stop it with a
   `cleanup` dependency.

**Phase B: key dependants on their servers' inputs** (`4xjgil`).

1. Allow `cacheStrategy: hash` for wait dependencies (`TaskDepsBuilder`), and default to it for
   persistent wait dependencies.
2. In the dependant's `TaskRunner::hash`, compute the wait dependency's input fingerprint with
   `TaskHasher` (its inputs, env, and command, and the hashes of its own required dependencies),
   instead of reading its state, as it never produces a result to hash.

**Phase C: start servers only when needed** (`s0p48p`, `m2vx9r`). Requires phase B, as the key must
not need the server running.

1. For wait edges, dispatch the dependant first. It hashes and checks the cache. On a hit, it
   hydrates, and the server is never started. On a miss, it asks the dispatcher to start its wait
   dependencies, and waits until they've started (`JobDispatcher::has_started_wait_dependency`).
2. A wait dependency that no dependant requested finishes as `Skipped`. Cleanup dependencies already
   skip when there's nothing to clean up, so they compose.
3. The biggest change: the persistent and wait dispatch in `ActionPipeline`, `JobDispatcher`, and
   `TaskRunner` (hash and hydrate before dependencies start). Remove the docs caveat about disabling
   the cache of tasks that wait on servers.

### Decisions

Recommendations, not yet confirmed:

1. Target `develop-3.0`.
2. Stopping is the default for wait dependencies on 3.0, without a new option.
3. A dependant is keyed on its servers' inputs by default, with `cacheStrategy: ignored` to opt out.
4. One PR per phase, A, then B, then C.

### Tests

- `JobDispatcher` graph tests for `TaskDependencyType::Wait`: stop after the last dependant, the
  primary target exemption, and (phase C) the lazy start.
- `moon_cli` run tests mirroring each case, with a time limit, that check no processes are left
  running.

## Share the local cache across worktrees by default

Case: [`95qyl3`](https://vznjs.github.io/vx/conformance/#95qyl3). Not a bug: the local cache lives
in each checkout's `.moon/cache`, and sharing is opt-in. On 2.x it passes with
`experiments.casOutputsCache` and `cache.unstable_sharedWorktreeCache`. On `develop-3.0`, the CAS is
the only local cache, and `cache.sharedWorktreeCache` defaults to `false`. Flipping that default
needs coverage for concurrent writers from two worktrees, a bare repository layout (which falls back
to `~/.moon/cache/shared`), and `moon clean` not removing another worktree's entries.

## Inputs that change while a task runs

Case: [`o2iy1u`](https://vznjs.github.io/vx/conformance/#o2iy1u), which every tool but vx fails.
moon hashes a task before it runs, and never checks its inputs again before archiving, so a result
built from an input that changed mid-run is stored under the old key. A possible approach, behind an
experiment at first: snapshot the size, modified time, and inode of the task's input files (and
those of its `cacheStrategy: hash` dependencies) while hashing, then compare them after the command
passes, and skip archiving when anything changed. Measure the cost with the `bench` crate first.

## Cases that only fail without a toolchain

Cases: [`o8neo7`](https://vznjs.github.io/vx/conformance/#o8neo7) (dependency bin on `PATH`) and
[`l2ggkq`](https://vznjs.github.io/vx/conformance/#l2ggkq) (malformed `package.json`). Both pass
with the `javascript` toolchain enabled, as it adds `node_modules/.bin` to `PATH`, and is what reads
`package.json`. Feedback for the suite: vendor the toolchain's WASM plugin into the sandbox and
reference it with a `file://` locator in `.moon/toolchains.yml`. A minor polish for the plugins
repository: the parse error shows the WASI path (`/workspace/...`) instead of the real path.
