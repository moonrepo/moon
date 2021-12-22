# CLI commands

- [Environment](#environment)
  - [`bin`](#bin)
  - [`setup`](#setup)
  - [`teardown`](#teardown)
- [Projects](#projects)
  - [`project`](#project)
  - [`project-graph`](#project-graph)

## Environment

### `bin`

The `bin <tool>` command will return an absolute path to a tool's binary within the toolchain. If a
tool has not been configured or installed, this will return a 1 or 2 exit code respectively, with no
value.

```shell
$ mono bin node
/Users/example/.monolith/tools/node/x.x.x/bin/node
```

A tool is considered "not configured" when not in use, for example, querying yarn/pnpm when the
package manager is configured for "npm". A tool is considered "not installed", when it has not been
downloaded and installed into the tools directory.

### `setup`

The `setup` command can be used to setup the developer and pipeline environments. It achieves this
by doing the following:

- Downloading and installing all configured tools into the toolchain.

```shell
$ mono setup
```

> This command should rarely be used, as the environment is automatically setup when running other
> commands, like detecting affected projects, running a task, or generating a build artifact.

### `teardown`

The `teardown` command, as its name infers, will teardown and clean the current environment,
opposite the [`setup`](#setup) command. It achieves this by doing the following:

- Uninstalling all configured tools in the toolchain.
- Removing any download or temporary files/folders.

```shell
$ mono teardown
```

## Projects

### `project`

The `project <id>` command will display all available information about a project that has been
configured and exists within the graph. If a project does not exist, the program will return with a
1 exit code.

```shell
$ mono project web
```

### `project-graph`

The `project-graph` command will generate a graph of all configured projects, with edges between
dependencies, and will output the graph in
[Graphviz DOT format](https://graphviz.org/doc/info/lang.html). This output can then be used by any
tool or program that supports DOT, for example, this
[live preview visualizer](https://dreampuf.github.io/GraphvizOnline).

```shell
$ mono project-graph > graph.dot
```

> A project ID can be passed to focus the graph to only that project and it's dependencies. For
> example, `mono project-graph web`.
