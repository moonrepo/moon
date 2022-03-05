# CLI commands

- [Environment](#environment)
  - [`bin`](#bin)
  - [`setup`](#setup)
  - [`teardown`](#teardown)
- [Projects](#projects)
  - [`project`](#project)
  - [`project-graph`](#project-graph)
- [Jobs](#jobs)
  - [`ci`](#ci)
    - [Revision comparison](#revision-comparison)
    - [Parallelism](#parallelism)

## Environment

### `bin`

The `bin <tool>` command will return an absolute path to a tool's binary within the toolchain. If a
tool has not been configured or installed, this will return a 1 or 2 exit code respectively, with no
value.

```shell
$ moon bin node
/Users/example/.moon/tools/node/x.x.x/bin/node
```

A tool is considered "not configured" when not in use, for example, querying yarn/pnpm when the
package manager is configured for "npm". A tool is considered "not installed", when it has not been
downloaded and installed into the tools directory.

### `setup`

The `setup` command can be used to setup the developer and pipeline environments. It achieves this
by doing the following:

- Downloading and installing all configured tools into the toolchain.

```shell
$ moon setup
```

> This command should rarely be used, as the environment is automatically setup when running other
> commands, like detecting affected projects, running a task, or generating a build artifact.

### `teardown`

The `teardown` command, as its name infers, will teardown and clean the current environment,
opposite the [`setup`](#setup) command. It achieves this by doing the following:

- Uninstalling all configured tools in the toolchain.
- Removing any download or temporary files/folders.

```shell
$ moon teardown
```

## Projects

### `project`

The `project <id>` command will display all available information about a project that has been
configured and exists within the graph. If a project does not exist, the program will return with a
1 exit code.

```shell
$ moon project web
```

### `project-graph`

The `project-graph` command will generate a graph of all configured projects, with edges between
dependencies, and will output the graph in
[Graphviz DOT format](https://graphviz.org/doc/info/lang.html). This output can then be used by any
tool or program that supports DOT, for example, this
[live preview visualizer](https://dreampuf.github.io/GraphvizOnline).

```shell
$ moon project-graph > graph.dot
```

> A project ID can be passed to focus the graph to only that project and it's dependencies. For
> example, `moon project-graph web`.

## Jobs

### `ci`

The `ci` command is a special command that should be ran in a continuous integration (CI)
environment, as it does all the heavy lifting necessary for effectively running jobs. It achieves
this by doing the following:

- Determines touched files by comparing the current HEAD against the base.
- Determines all targets (project + tasks) that need to run based on touched files.
- Additionally runs affected targets dependencies _and_ dependents.
- Generates a task and dependency graph.
- Installs the toolchain, Node.js, and npm dependencies.
- Runs all tasks within the graph using a thread pool.
- Displays stats about all passing, failed, and invalid tasks.

```shell
$ moon ci
```

#### Revision comparison

By default the command will compare the current branches HEAD against the base revision, which is
typically the configured `vcs.defaultBranch` (master, main, trunk, etc). Both of these can be
customized with the `--base` and `--head` options respectively.

```shell
$ moon ci --base another-branch --head <SHA>
```

#### Parallelism

If your CI environment supports splitting up tasks across multiple jobs, then you can utilize Moon's
built in parallelism, by passing `--jobTotal` and `--job` options. The `--jobTotal` option is an
integer of the total number of jobs available, and `--job` is the current index (0 based) amongst
the total.

When these options are passed, Moon will only run affected targets based on the current job slice.

```shell
$ moon ci --job 1 --jobTotal 5
```

> Your CI environment may provide environment variables for these 2 values.
