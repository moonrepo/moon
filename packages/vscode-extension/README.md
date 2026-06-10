# moon

While moon provides a powerful and robust command line, an in-editor interface with deep integration
can provide a much better developer experience, and that's exactly what this moon console does!

Whether you're just learning, or an experienced veteran, this console will take you to the moon!

## Features

### Projects view

The backbone of moon is the projects view. In this view, all moon configured projects will be
listed, categorized by their `type`, and designated with their `language`.

Each project can then be expanded to view all available tasks. Tasks can be ran by clicking the `▶`
icon, or using the command palette.

<img
src="https://raw.githubusercontent.com/moonrepo/dev/master/packages/vscode-extension/images/projects-view.png"
alt="Screenshot of projects view" width="300px" />

> This view is available in both the "Explorer" and "moon" sections.

### Tags view

Similar to the projects view, the tags view displays projects grouped by their `tags`.

<img
src="https://raw.githubusercontent.com/moonrepo/dev/master/packages/vscode-extension/images/tags-view.png"
alt="Screenshot of tags view" width="300px" />

> This view is only available in the "moon" section.

### Last run view

Information about the last ran task will be displayed in a beautiful table with detailed stats.

<img
src="https://raw.githubusercontent.com/moonrepo/dev/master/packages/vscode-extension/images/last-run-view.png"
alt="Screenshot of last run view" width="300px" />

> This view is only available in the "moon" section.

### Control panel

To provide the best experience, all major features, enhancements, and integrations can be found
within the moon specific control panel. Simply click the moon icon in the activity bar!

<img
src="https://raw.githubusercontent.com/moonrepo/dev/master/packages/vscode-extension/images/activity-icon.png"
alt="Screenshot of moon activity" width="50px"  />

## Requirements

This extension requires moon itself to be installed within the opened VS Code workspace. Learn more
about [installing and configuring moon](https://moonrepo.dev/docs/install)!

## Settings

The following settings are available for this extension.

- `moon.binPath` - Relative path from moon's workspace root to the moon binary. Defaults to
  `node_modules/@moonrepo/cli/moon`. _Windows will auto-suffix with `.exe`!_
- `moon.hideTasks` - List of tasks to hide in the projects view, using target syntax. Supports
  fully-qualified targets (`app:build`) and partial targets (`:build` or `*:build`). Defaults to
  `[]`.
- `moon.logLevel` - The log level to apply to all moon executed commands. Defaults to `info`.
- `moon.rootPrefixes` - List of relative paths from the editor root in which to find moon's
  workspace root. Defaults to `['.']`. This is useful if moon is initialized in a sub-folder.

## Commands

The following commands are available in the command palette (typically `cmd + shift + p`), and are
prefixed with "moon".

- **Open settings** - Opens the settings page and filters to all moon applicable settings.
- **Run task** - Prompts the user for a task(s) and runs it in the terminal.
- **View action graph** - Opens a panel that renders an interactive action graph visualization.
- **View project graph** - Opens a panel that renders an interactive project graph visualization.

## Roadmap

- [x] Projects view
  - [x] Categorize projects based on type
  - [x] List tasks
  - [x] Categorize tasks based on type
  - [x] Run a task
  - [x] Check a project
- [x] Tags view
- [x] Last run view
- [x] Commands and command palette
- [x] Watches and reacts to file system changes.
- [x] Schema validation for YAML configs
- [ ] moon language server
- [ ] Auto-completion
- [ ] In-editor code generation
- [x] Graph visualizer
- [x] Multi-workspace support
