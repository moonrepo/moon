# 0.1.0

- [x] website
  - [x] getting started

## Targets

- [x] macos x64
- [x] linux x64 (gnu)
- [x] windows x64

## Projects

- [x] Can define metadata
- [x] File groups
  - [x] Configure in `project.yml`
  - [x] Deep overrides with global `project.yml`
- [x] Tasks
  - [x] Configure in `project.yml`
  - [x] Deep merges with global `project.yml`
  - [x] Supports merge strategies
- [x] Checks if affected based in a file list
- [x] Lazy loads `package.json`
- [x] Lazy loads `tsconfig.json`

## Project graph

- [x] Lazy load projects when needed
- [x] Get direct dependencies
- [x] Get direct dependents

## Tasks

- [x] Command (and type)
- [x] Args
- [x] Inputs
  - [x] Checks if affected based in a file list
  - [x] Globs
  - [x] Relative paths
  - [x] Workspace relative paths
- [x] Outputs
  - [x] Write outputs to `.moon/cache/out`
  - [x] Symlink/copy outputs back to project dir
- [x] Dependencies (on other tasks)
- [x] Environment vars
- [x] Tokens
  - [x] Expands tokens defined in configs
- [x] Can run from project root or workspace root (using `run_from_workspace_root`)
- [x] Self referencing targets (`~`)
- [x] Deps referencing targets (`^`)

## Action runner

- [x] Sorts dep graph topologically
  - [x] Groups into batches and parallelizes
  - [x] Runs in a thread pool (via tokio)
- [x] Runs task based on `type`
- [x] Retries when failed (using `retry_count`)
- [x] Streams output when a primary target
- [x] Buffers output when a non-primary target
- [x] Bubbles up errors
- [x] Installs npm dependencies
- [x] Syncs `package.json` and `tsconfig.json` for all projects
  - [x] Writes JSON preserving field order
- [x] Handle non-0 exit codes
- [x] Handle offline

## CLI

- [x] `init` command to scafflold a new project
- [x] `project` command for displaying info
- [x] `project-graph` command for outputting DOT format
- [x] `setup` command for installing tools
- [x] `teardown` command for uninstalling tools
- [x] `bin` command to return tool paths
- [x] `run` command to run targets
  - [x] Args after `--` are passed to the underlying command
  - [x] Only run on affected changes
  - [x] Run multiple targets
- [x] `ci` command for smart running affected targets (below)

## CI

- [x] Compares PR against default branch
- [x] Runs tasks if `outputs` defined or `run_in_ci` is true
- [x] Runs dependencies AND dependents

## Cache

- [x] add a `--no-cache` option to disable all caching
- [x] hashing
  - [x] use `stdin` for commands that take long arguments
  - [x] dont load `package.json`/`tsconfig.json` so much
  - [x] delete old hashes when the hash changes
  - [x] include local file changes in hash

# 0.2.0

## Cache

- [ ] hashing
  - [ ] ignore hashes for files that are gitignored
- [ ] add docs on caching options

## Tests

- [ ] add code coverage reports in CI
- [ ] increase code coverage and add more integration tests

## Targets

- [ ] macos arm/m1
- [ ] linux x64 (musl)

# Backlog

## Tasks

- [ ] Add `@cache` token

## Action runner

- [ ] Add a debug layer so that the node processes can be inspected
- [ ] Write output logs for every action

## CLI

- [ ] `run-many`
- [ ] `graph`
  - [ ] Spin up an interactive website with full project/task data

## Node.js

- [ ] Add chrome profiling support to spawned processes
- [ ] Publish npm packages
