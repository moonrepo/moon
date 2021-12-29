# Monolith

Monolith is a Rust program for managing JavaScript based monorepo's.

> Inspired heavily from Bazel.

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
