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
items are in moonrepo/moon#2820, one commit each. The last five rows are smaller issues found while
fixing and reviewing them, which no case tests directly.

| Item                                                                                                    | Cases                                                                | Status                                 |
| ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | -------------------------------------- |
| Unset and empty env inputs hash the same                                                                | `hjbima`                                                             | Fixed                                  |
| [Env inputs with lowercase names are file inputs](#env-inputs-with-lowercase-names-are-file-inputs)     | `f4a2p6`                                                             | Won't fix, names must be uppercase     |
| Glob characters in a project's path, and `PATH` separators                                              | `rqk8s7`                                                             | Fixed (starbase_utils v0.14.13)        |
| A dependency's `bash` or `sh` bin replaces the task's shell                                             | `uqu850` (shell part)                                                | Fixed (starbase_shell v0.14.6)         |
| A failed run blocks hydrating a cached pass                                                             | `dbr6wr`                                                             | Fixed                                  |
| Timeouts greater than a day are ignored, and `0` kills the task                                         | `zqwwd3`                                                             | Fixed                                  |
| [Servers behind `wait` dependencies](#servers-behind-wait-dependencies)                                 | `uqu850`, `4xjgil`, `s0p48p`, `ex25la`, `fkw67u`, `m2vx9r`, `40x9mk` | Planned for 3.0                        |
| [Share the local cache across worktrees by default](#share-the-local-cache-across-worktrees-by-default) | `95qyl3`                                                             | Not yet decided                        |
| [Inputs that change while a task runs](#inputs-that-change-while-a-task-runs)                           | `o2iy1u`                                                             | Not yet decided                        |
| [Cases that only fail without a toolchain](#cases-that-only-fail-without-a-toolchain)                   | `o8neo7`, `l2ggkq`                                                   | No moon change, feedback for the suite |
| [Empty env inputs don't mark tasks as affected](#empty-env-inputs-dont-mark-tasks-as-affected)          | Related to `hjbima`                                                  | Not yet decided                        |
| [Env variables are substituted inside longer names](#env-variables-are-substituted-inside-longer-names) | Related to `f4a2p6`                                                  | Not yet decided                        |
| [Token variables aren't escaped in globs](#token-variables-arent-escaped-in-globs)                      | Related to `rqk8s7`                                                  | Not yet decided                        |
| [Env variables are substituted in project paths](#env-variables-are-substituted-in-project-paths)       | Related to `rqk8s7`                                                  | Not yet decided                        |
| [Other bugs found while reviewing the fixes](#other-bugs-found-while-reviewing-the-fixes)               | Related to `hjbima`, `rqk8s7`, `zqwwd3`                              | Not yet decided                        |

## Env inputs with lowercase names are file inputs

Case: [`f4a2p6`](https://vznjs.github.io/vx/conformance/#f4a2p6). An env input's name must be
uppercase (`[A-Z0-9_]`, `crates/config/src/patterns.rs`), the convention for environment variables,
so `$__proto__` is a file input, and the case's variable never keys the task. Lowercase names were
supported for a while during the audit, but that also made a bare lowercase output, like `$foo`,
fail to parse instead of being a file, so it was reverted. The docs now say that names must be
uppercase, and that a lowercase name is a file path. A separate `env` field, or an `{ env: '...' }`
input, like other tools use, would avoid parsing names out of paths, and could be considered for a
major release.

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

## Empty env inputs don't mark tasks as affected

Related to [`hjbima`](https://vznjs.github.io/vx/conformance/#hjbima), which is fixed for the hash:
an unset env input and one set to an empty string now hash differently. The affected tracker still
treats an empty variable as unset, and only marks a task as affected by an env input when the
variable is set and not empty (`crates/affected/src/affected_tracker.rs:496` and
`crates/affected/src/task_tracker.rs:59`). So setting a variable to `""` changes the task's hash,
but doesn't make it affected. This was left as is during the audit, as treating empty as unset there
was intentional. Decide whether the two should agree.

## Env variables are substituted inside longer names

Related to [`f4a2p6`](https://vznjs.github.io/vx/conformance/#f4a2p6). Substitution uses `ENV_VAR`
(`crates/env-var/src/lib.rs`), which matches a name without a boundary. So in `$FOObar` it
substitutes `$FOO` and keeps `bar`, and in a `$Foo` file input it substitutes `$F`, giving `oo`. A
name of only underscores is no longer substituted, so `$__proto__` is kept as-is. The same regex
infers env inputs from a command (`inferInputs`, `EnvScanner` in
`crates/env-var/src/env_scanner.rs`), so `$FOObar` in a command infers `$FOO`. Matching the whole
name, or not substituting a partial match, changes how args, env, inputs, and outputs are
substituted everywhere, so it needs its own change and tests.

## Token variables aren't escaped in globs

Related to [`rqk8s7`](https://vznjs.github.io/vx/conformance/#rqk8s7). A task glob is kept as
configured, alongside a form that moon walks and matches with, where the project's path is escaped.
Token variables are replaced in both forms afterward, with their values as-is
(`create_glob_for_task` in `crates/task-expander/src/token_expander.rs`). In a `packages/[lib]`
project, an input of `/$projectSource/src/**/*` becomes `packages/[lib]/src/**/*` in both forms, so
`[lib]` is a character class, and the task hashes none of its files. A fix would replace token
variables with escaped values in the form to walk and match with. Of the path variables, this mostly
affects `$projectSource`, as the others are absolute. But `$target` always contains `:`, so an
output like `dist/$target/**/*` fails with "Failed to create glob" in every project, as it did on
v2.6.4.

## Env variables are substituted in project paths

Related to [`rqk8s7`](https://vznjs.github.io/vx/conformance/#rqk8s7), and unchanged from 2.6.4. Env
variables are replaced after the project's path is joined with a task's path, so a `$` in the
project's path is substituted too. In a `packages/$X` project, an input of `src/**/*` becomes
`packages//src/**/*`, and the run fails with "Failed to create glob from pattern". File paths are
substituted the same way. A fix would replace variables in the path as configured, before joining it
with the project's path.

## Other bugs found while reviewing the fixes

These were found while reviewing the fixes in moonrepo/moon#2820, and also happen on v2.6.4. Some of
them cause failures in sections of the suite that haven't been audited yet.

- **`MOON_AFFECTED_FILES` fails on a path with `:`.** Related to
  [`rqk8s7`](https://vznjs.github.io/vx/conformance/#rqk8s7). Affected files are joined with
  `env::join_paths` (`crates/task-runner/src/command_builder.rs:405`), which fails on the `PATH`
  separator, like `PATH` itself did. A task that passes affected files as an environment variable
  fails with "path segment contains separator `:`" for a file like `packages/a/src/a:b.txt`, or for
  any file of a `packages/x:y` project when it runs from the workspace root.
- **A requirement check that times out passes.** Related to
  [`zqwwd3`](https://vznjs.github.io/vx/conformance/#zqwwd3). A check that times out has no output
  and no error, and `execute_checks` (`crates/task-runner/src/task_runner.rs`) only fails a
  requirement whose output isn't successful, so the task runs.
- **A fingerprint check that times out is left out of the hash.** Related to
  [`zqwwd3`](https://vznjs.github.io/vx/conformance/#zqwwd3). For the same reason, `hash_checks`
  skips its result, so the task is replayed from the cache when what the check fingerprints has
  changed.
- **A timeout doesn't kill the task's child processes.** Related to
  [`zqwwd3`](https://vznjs.github.io/vx/conformance/#zqwwd3). On a timeout, the command's future is
  dropped, and starbase_process v0.1.3 doesn't kill the process tree, so a `sleep` the command
  started kept running, with a parent of `1`. The Failures cases `y2xi09`, `oxdk18`, `8sy8f6`,
  `h6rv38`, `bxtaeb`, and `aih3s5` fail on this. Containing the process tree of tasks and checks
  with starbase_process v0.4 (moonrepo/moon#2819, for 3.0) may fix it, but that hasn't been checked.
- **Values that an `envFile` substitutes from the environment aren't hashed.** Related to
  [`hjbima`](https://vznjs.github.io/vx/conformance/#hjbima). With `OTHER=${MODE-dflt}` in an env
  file, and no `$MODE` input, changing `MODE` replays the cached result, as the hash is the same.
  The same substitution in a task's `env` is hashed.
- **`${VAR-x}` and `${VAR+x}` treat an empty variable as unset.** Related to
  [`hjbima`](https://vznjs.github.io/vx/conformance/#hjbima). Unlike in a POSIX shell, they behave
  like `${VAR:-x}` and `${VAR:+x}` (`crates/env-var/src/env_substitutor.rs:112-127`).
- **Env wildcards in file groups aren't expanded.** With a file group of `['$BAR_*']`, and an input
  of `@envs(group)` or `@group(group)`, the task's env inputs contain the literal `BAR_*`, and the
  hasher looks up a variable with that name. A file group keeps the pattern as is
  (`crates/file-group/src/file_group.rs:85`), and the tokens copy it into the task's env inputs
  (`crates/task-expander/src/token_expander.rs`), while a task's own `$BAR_*` input is expanded to
  the variables that match.
- **A project in a directory that starts with `!` hashes none of its inputs.** Related to
  [`rqk8s7`](https://vznjs.github.io/vx/conformance/#rqk8s7). With `hasher.walkStrategy: glob`, a
  project at `packages/!foo` hashes no inputs, and a task with outputs fails, as none are found. `!`
  isn't a glob character in wax, so it isn't escaped, but when walking, starbase_utils treats the
  pattern after its leading directory, `!foo/...`, as a negation.
