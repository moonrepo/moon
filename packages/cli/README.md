# @moonrepo/cli

The official CLI for [moon](https://moonrepo.dev), a build system and repo management tool for the
JavaScript ecosystem, written in Rust!

- [Documentation](https://moonrepo.dev/docs)
- [Getting started](https://moonrepo.dev/docs/install)

## Requirements

- Node.js >= 14.15

## Installation

moon can be installed with npm, pnpm, or yarn.

```bash
# Install the dependency
yarn add --dev @moonrepo/cli

# Initialize moon in the repo
npx @moonrepo/cli init
```

## Usage

Once [projects](https://moonrepo.dev/docs/create-project) and
[tasks](https://moonrepo.dev/docs/create-task) have been configured, you can run tasks with:

```bash
# Run `lint` in project `app`
moon run app:lint

# Run `lint` in all projects
moon run :lint
```

## Why use moon?

Working in the JavaScript ecosystem can be very involved, especially when it comes to managing a
repository effectively. Which package manager to use? Which Node.js version to use? How to import
node modules? How to build packages? So on and so forth. moon aims to streamline this entire process
and provide a first-class developer experience.

- **Increased productivity** - With [Rust](https://www.rust-lang.org/) as our foundation, we can
  ensure robust speeds, high performance, and low memory usage. Instead of long builds blocking you,
  focus on your work.
- **Exceptional developer experience** - As veterans of the JavaScript ecosystem, we're well aware
  of the pain points and frustrations. Our goal is to mitigate and overcome these obstacles.
- **Incremental adoption** - At its core, moon has been designed to be adopted incrementally and is
  _not_ an "all at once adoption". Migrate project-by-project, or task-by-task, it's up to you!
- **Reduced scripts confusion** - `package.json` scripts can become unwieldy, very quickly. No more
  duplicating the same script into every package, or reverse-engineering which root scripts to use.
  With moon, all you need to know is the project name, and a task name.
- **Ensure correct versions** - Whether it's Node.js or npm, ensure the same version of each tool is
  the same across _every_ developer's environment. No more wasted hours of debugging.
- **Automation built-in** - When applicable, moon will automatically install `node_modules`, or sync
  package dependencies, or even sync TypeScript project references.
- And of course, the amazing list of features below!

## Features

> Not all features are currently supported, view the documentation for an accurate list!

#### Management

- **Smart hashing** - Collects inputs from multiple sources to ensure builds are deterministic and
  reproducible.
- **Remote caching** - Persists builds, hashes, and caches between teammates and CI/CD environments.
- **Integrated toolchain** - Automatically downloads and installs explicit versions of Node.js and
  other tools for consistency.
- **Multi-platform** - Runs on common development platforms: Linux, macOS, and Windows.

#### Organization

- **Project graph** - Generates a project graph for dependency and dependent relationships.
- **Code generation** - Easily scaffold new applications, libraries, tooling, and more!
- **Dependency workspaces** - Works alongside package manager workspaces so that projects have
  distinct dependency trees.
- **Ownership metadata** - Declare an owner, maintainers, support channels, and more, for LDAP or
  another integration.

#### Orchestration

- **Dependency graph** - Generates a dependency graph to increase performance and reduce workloads.
- **Action runner** - Executes actions in parallel and in order using a thread pool and our
  dependency graph.
- **Action distribution** - Distributes actions across multiple machines to increase throughput.
- **Incremental builds** - With our smart hashing, only rebuild projects that have been touched
  since the last build.

#### Notification

- **Flakiness detection** - Reduce flaky builds with automatic retries and passthrough settings.
- **Webhook events** - Receive a webhook for every event in the pipeline. Useful for metrics
  gathering.
- **Terminal notifications** - Receives notifications in your chosen terminal when builds are
  successful... or are not.
