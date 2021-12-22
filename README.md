# Monolith

Monolith is a Rust program for managing JavaScript based monorepo's.

> Inspired heavily from Bazel.

## Features

- **Hermetic environments** - Ensure the same environment and expectations across every machine.
- **Concurrent tasks** - Run tasks in parallel using a worker farm.
- **Other cool stuff...**

## Concepts

### Task

An action that can be ran within the context of a [project](#project), and are configured through a
[`tasks.ts`](#tasksts) file. Tasks are separated into the following types:

- **Build** - Generates an output from an input. Example: babel, rollup, webpack.
- **Test** - Validates criteria on a set of inputs. Example: jest, eslint, typescript.
- **Run** - Runs a one-off or long-lived process. Example: (watch mode), prettier, ts-node.

Underneath the hood, a task is simply a node module binary or a JavaScript/TypeScript script, that
is ran as a child-process. Tasks communicate between the Monolith Rust client and server through a
JSON-like message system, with this message structure supporting the following fields:

- `script` (`string`) - Either a node module binary name, npm package name, or JavaScript file path
  to execute.
- `args` (`string[]`) - List of command like arguments that will be passed to `script`.
- `inputs` (`(FileGroupToken | ProjectFile | WorkspaceFile)[]`) - List of file groups (via tokens),
  relative files/folders/globs from project root, and relative files/folders/globs from workspace
  root.
  - Inputs are used as the delta to determine if the task should rebuild, based on the state of the
    previous build.
  - Inputs can be referenced in arguments using the `@in` token.
- `outputs` (`ProjectFile[]`) - List of relative files/folders/globs from project root, that will be
  created from this task.
  - Outputs can be piped into other tasks to create a dependency chain.
  - Outputs can be referenced in arguments using the `@out` token.
- `deps` (`Target[]`) - List of external tasks, using a target label, that will be executed before
  this task.
  - The output of these targets can be referenced in arguments using the `@dep` token.
- `options` (`TaskOptions`) - Object of options to customize the task process.
  - `retryCount` (`number`) - Number of times to retry the task before ultimately failing.

#### Tokens

- File groups
  - `@glob` - Returns the file group as a glob (typically as-is).
  - `@root` - Returns the file group, reduced down to the lowest possible directory.
  - `@dirs` - Returns the file group, reduced down to all possible directories.
  - `@files` - Returns the file group as a list of all possible files.
- Inputs & outputs
  - `@in` - Points to an index within a task's `inputs` list. This will be expanded to the
    underyling file path(s).
  - `@out` - Points to an index within a task's `outputs` list. This will be expanded to the
    underyling file path(s).
  - `@dep` - Points to an index within a task's `deps` list. This will be expanded to the underyling
    file path(s) of the task's output.
- Other
  - `@cache` - Returns an absolute file path to a location within the cache folder.
  - `@pid` - Returns the running project's ID as a fully-qualified ID from the workspace root.

### Target

A target is a label composed of a [project ID](#pid) and task name, separated by a colon (`:`).
Targets are used by terminal commands and task configurations for declaring cross-project or
cross-task dependencies.

For example, if project `/apps/client` contained a task named `lint`, then the target would be
`/apps/client:lint`.
