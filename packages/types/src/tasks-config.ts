// Automatically generated by schematic. DO NOT MODIFY!

/* eslint-disable */

import type { ExtendsFrom } from './common';

export type TaskArgs = null | string | string[];

/** Expanded information about a task dependency. */
export interface TaskDependencyConfig {
	/** Additional arguments to pass to this dependency when it's ran. */
	args: TaskArgs;
	/** A mapping of environment variables specific to this dependency. */
	env: Record<string, string>;
	/** Marks the dependency is optional when being inherited from the top-level. */
	optional: boolean | null;
	/** The target of the depended on task. */
	target: string;
}

export type TaskDependency = string | TaskDependencyConfig;

export type TaskOptionEnvFile = boolean | string | string[];

/** The strategy in which to merge a specific task option. */
export type TaskMergeStrategy = 'append' | 'prepend' | 'preserve' | 'replace';

/** The operating system in which to only run this task on. */
export type TaskOperatingSystem = 'linux' | 'macos' | 'windows';

/** The style in which task output will be printed to the console. */
export type TaskOutputStyle = 'buffer' | 'buffer-only-failure' | 'hash' | 'none' | 'stream';

/** The priority levels a task can be bucketed into. */
export type TaskPriority = 'critical' | 'high' | 'normal' | 'low';

/** A list of available shells on Unix. */
export type TaskUnixShell =
	| 'bash'
	| 'elvish'
	| 'fish'
	| 'ion'
	| 'murex'
	| 'nu'
	| 'pwsh'
	| 'xonsh'
	| 'zsh';

/** A list of available shells on Windows. */
export type TaskWindowsShell = 'bash' | 'elvish' | 'fish' | 'murex' | 'nu' | 'pwsh' | 'xonsh';

/** Options to control task inheritance and execution. */
export interface TaskOptionsConfig {
	/** The pattern in which affected files will be passed to the task. */
	affectedFiles: boolean | 'args' | 'env' | null;
	/**
	 * When affected and no files are matching, pass the task inputs
	 * as arguments to the command, instead of `.`.
	 */
	affectedPassInputs: boolean | null;
	/** Allows the task to fail without failing the entire pipeline. */
	allowFailure: boolean | null;
	/** Caches the `outputs` of the task */
	cache: boolean | null;
	/**
	 * Lifetime to cache the task itself, in the format of "1h", "30m", etc.
	 * If not defined, caches live forever, or until inputs change.
	 */
	cacheLifetime: string | null;
	/**
	 * Loads and sets environment variables from the `.env` file when
	 * running the task.
	 */
	envFile: TaskOptionEnvFile | null;
	/**
	 * Automatically infer inputs from file groups or environment variables
	 * that were utilized within `command`, `script`, `args`, and `env`.
	 */
	inferInputs: boolean | null;
	/**
	 * Marks the task as interactive, so that it will run in isolation,
	 * and have direct access to stdin.
	 */
	interactive: boolean | null;
	/**
	 * Marks the task as internal, which disables it from begin ran
	 * from the command line, but can be depended on.
	 */
	internal: boolean | null;
	/**
	 * The default strategy to use when merging `args`, `deps`, `env`,
	 * `inputs`, or `outputs` with an inherited task. Can be overridden
	 * with the other field-specific merge options.
	 *
	 * @default 'append'
	 */
	merge: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `args` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeArgs: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `deps` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeDeps: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `env` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeEnv: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `inputs` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeInputs: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `outputs` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeOutputs: TaskMergeStrategy | null;
	/**
	 * Creates an exclusive lock on a virtual resource, preventing other
	 * tasks using the same resource from running concurrently.
	 */
	mutex: string | null;
	/** The operating system in which to only run this task on. */
	os: TaskOperatingSystem | TaskOperatingSystem[] | null;
	/**
	 * The style in which task output will be printed to the console.
	 *
	 * @default 'buffer'
	 * @envvar MOON_OUTPUT_STYLE
	 */
	outputStyle: TaskOutputStyle | null;
	/**
	 * Marks the task as persistent (continuously running). This is ideal
	 * for watchers, servers, or never-ending processes.
	 */
	persistent: boolean | null;
	/**
	 * Marks the task with a certain priority, which determines the order
	 * in which it is ran within the pipeline.
	 *
	 * @default 'normal'
	 */
	priority: TaskPriority | null;
	/**
	 * The number of times a failing task will be retried to succeed.
	 *
	 * @envvar MOON_RETRY_COUNT
	 */
	retryCount: number | null;
	/**
	 * Runs direct task dependencies (via `deps`) in sequential order.
	 * This _does not_ apply to indirect or transient dependencies.
	 */
	runDepsInParallel: boolean | null;
	/** Runs the task from the workspace root, instead of the project root. */
	runFromWorkspaceRoot: boolean | null;
	/** Whether to run the task in CI or not, when executing `moon ci` or `moon run`. */
	runInCI: boolean | 'always' | 'affected' | null;
	/**
	 * Runs the task within a shell. When not defined, runs the task
	 * directly while relying on `PATH` resolution.
	 */
	shell: boolean | null;
	/** The maximum time in seconds that a task can run before being cancelled. */
	timeout: number | null;
	/**
	 * The shell to run the task in when on a Unix-based machine.
	 *
	 * @default 'bash'
	 */
	unixShell: TaskUnixShell | null;
	/**
	 * The shell to run the task in when on a Windows machine.
	 *
	 * @default 'pwsh'
	 */
	windowsShell: TaskWindowsShell | null;
}

/** Platforms that each programming language can belong to. */
export type PlatformType = 'bun' | 'deno' | 'node' | 'python' | 'rust' | 'system' | 'unknown';

/** Preset options to inherit. */
export type TaskPreset = 'server' | 'watcher';

/** The type of task. */
export type TaskType = 'build' | 'run' | 'test';

/** Configures a task to be ran within the action pipeline. */
export interface TaskConfig {
	/**
	 * Arguments to pass to the command when it's ran. Can be
	 * defined as a string, or a list of individual arguments.
	 */
	args: TaskArgs;
	/**
	 * The command or command line to execute when the task is ran.
	 * Supports the command name, with or without arguments. Can be
	 * defined as a string, or a list of individual arguments.
	 */
	command: TaskArgs;
	/**
	 * Other tasks that this task depends on, and must run to completion
	 * before this task is ran. Can depend on sibling tasks, or tasks in
	 * other projects, using targets.
	 */
	deps: TaskDependency[] | null;
	/** A human-readable description about the task. */
	description: string | null;
	/**
	 * A mapping of environment variables that will be set when the
	 * task is ran.
	 */
	env: Record<string, string> | null;
	/** Extends settings from a sibling task by ID. */
	extends: string | null;
	/**
	 * Inputs and sources that will mark the task as affected when comparing
	 * against touched files. When not provided, all files within the project
	 * are considered an input. When an empty list, no files are considered.
	 * Otherwise, an explicit list of inputs are considered.
	 */
	inputs: string[] | null;
	/**
	 * Marks the task as local only. Local tasks do not run in CI, do not have
	 * `options.cache` enabled, and are marked as `options.persistent`.
	 *
	 * @deprecated Use `preset` instead.
	 */
	local: boolean | null;
	/** Options to control task inheritance and execution. */
	options: TaskOptionsConfig;
	/**
	 * Outputs that will be created when the task has successfully ran.
	 * When `cache` is enabled, the outputs will be persisted for subsequent runs.
	 */
	outputs: string[] | null;
	/**
	 * The platform in which the task will be ran in. The platform determines
	 * available binaries, lookup paths, and more. When not provided, will
	 * be automatically detected.
	 *
	 * @default 'unknown'
	 * @type {'bun' | 'deno' | 'node' | 'python' | 'rust' | 'system' | 'unknown'}
	 */
	platform: PlatformType;
	/** The preset to apply for the task. Will inherit default options. */
	preset: TaskPreset | null;
	/**
	 * A script to run within a shell. A script is anything from a single command,
	 * to multiple commands (&&, etc), or shell specific syntax. Does not support
	 * arguments, merging, or inheritance.
	 */
	script: string | null;
	/**
	 * The toolchain(s) in which the task will be ran in. The toolchain determines
	 * available binaries, lookup paths, and more. When not provided, will
	 * be automatically detected.
	 */
	toolchain: string | string[];
	/**
	 * The type of task, primarily used for categorical reasons. When not provided,
	 * will be automatically determined.
	 *
	 * @default 'test'
	 */
	type: TaskType | null;
}

/**
 * Configures tasks and task related settings that'll be inherited by all
 * matching projects.
 * Docs: https://moonrepo.dev/docs/config/tasks
 */
export interface InheritedTasksConfig {
	/** @default 'https://moonrepo.dev/schemas/tasks.json' */
	$schema?: string;
	/**
	 * Extends one or many task configuration files. Supports a relative
	 * file path or a secure URL.
	 */
	extends: ExtendsFrom | null;
	/**
	 * A mapping of group IDs to a list of file paths, globs, and
	 * environment variables, that can be referenced from tasks.
	 */
	fileGroups: Record<string, string[]>;
	/**
	 * Task dependencies that'll automatically be injected into every
	 * task that inherits this configuration.
	 */
	implicitDeps: TaskDependency[];
	/**
	 * Task inputs that'll automatically be injected into every
	 * task that inherits this configuration.
	 */
	implicitInputs: string[];
	/** Default task options for all inherited tasks. */
	taskOptions: TaskOptionsConfig | null;
	/** A mapping of tasks by ID to parameters required for running the task. */
	tasks: Record<string, TaskConfig>;
}

export type PartialTaskArgs = null | string | string[];

/** Expanded information about a task dependency. */
export interface PartialTaskDependencyConfig {
	/** Additional arguments to pass to this dependency when it's ran. */
	args?: PartialTaskArgs | null;
	/** A mapping of environment variables specific to this dependency. */
	env?: Record<string, string> | null;
	/** Marks the dependency is optional when being inherited from the top-level. */
	optional?: boolean | null;
	/** The target of the depended on task. */
	target?: string | null;
}

export type PartialTaskDependency = string | PartialTaskDependencyConfig;

/** Options to control task inheritance and execution. */
export interface PartialTaskOptionsConfig {
	/** The pattern in which affected files will be passed to the task. */
	affectedFiles?: boolean | 'args' | 'env' | null;
	/**
	 * When affected and no files are matching, pass the task inputs
	 * as arguments to the command, instead of `.`.
	 */
	affectedPassInputs?: boolean | null;
	/** Allows the task to fail without failing the entire pipeline. */
	allowFailure?: boolean | null;
	/** Caches the `outputs` of the task */
	cache?: boolean | null;
	/**
	 * Lifetime to cache the task itself, in the format of "1h", "30m", etc.
	 * If not defined, caches live forever, or until inputs change.
	 */
	cacheLifetime?: string | null;
	/**
	 * Loads and sets environment variables from the `.env` file when
	 * running the task.
	 */
	envFile?: TaskOptionEnvFile | null;
	/**
	 * Automatically infer inputs from file groups or environment variables
	 * that were utilized within `command`, `script`, `args`, and `env`.
	 */
	inferInputs?: boolean | null;
	/**
	 * Marks the task as interactive, so that it will run in isolation,
	 * and have direct access to stdin.
	 */
	interactive?: boolean | null;
	/**
	 * Marks the task as internal, which disables it from begin ran
	 * from the command line, but can be depended on.
	 */
	internal?: boolean | null;
	/**
	 * The default strategy to use when merging `args`, `deps`, `env`,
	 * `inputs`, or `outputs` with an inherited task. Can be overridden
	 * with the other field-specific merge options.
	 *
	 * @default 'append'
	 */
	merge?: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `args` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeArgs?: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `deps` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeDeps?: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `env` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeEnv?: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `inputs` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeInputs?: TaskMergeStrategy | null;
	/**
	 * The strategy to use when merging `outputs` with an inherited task.
	 *
	 * @default 'append'
	 */
	mergeOutputs?: TaskMergeStrategy | null;
	/**
	 * Creates an exclusive lock on a virtual resource, preventing other
	 * tasks using the same resource from running concurrently.
	 */
	mutex?: string | null;
	/** The operating system in which to only run this task on. */
	os?: TaskOperatingSystem | TaskOperatingSystem[] | null;
	/**
	 * The style in which task output will be printed to the console.
	 *
	 * @default 'buffer'
	 * @envvar MOON_OUTPUT_STYLE
	 */
	outputStyle?: TaskOutputStyle | null;
	/**
	 * Marks the task as persistent (continuously running). This is ideal
	 * for watchers, servers, or never-ending processes.
	 */
	persistent?: boolean | null;
	/**
	 * Marks the task with a certain priority, which determines the order
	 * in which it is ran within the pipeline.
	 *
	 * @default 'normal'
	 */
	priority?: TaskPriority | null;
	/**
	 * The number of times a failing task will be retried to succeed.
	 *
	 * @envvar MOON_RETRY_COUNT
	 */
	retryCount?: number | null;
	/**
	 * Runs direct task dependencies (via `deps`) in sequential order.
	 * This _does not_ apply to indirect or transient dependencies.
	 */
	runDepsInParallel?: boolean | null;
	/** Runs the task from the workspace root, instead of the project root. */
	runFromWorkspaceRoot?: boolean | null;
	/** Whether to run the task in CI or not, when executing `moon ci` or `moon run`. */
	runInCI?: boolean | 'always' | 'affected' | null;
	/**
	 * Runs the task within a shell. When not defined, runs the task
	 * directly while relying on `PATH` resolution.
	 */
	shell?: boolean | null;
	/** The maximum time in seconds that a task can run before being cancelled. */
	timeout?: number | null;
	/**
	 * The shell to run the task in when on a Unix-based machine.
	 *
	 * @default 'bash'
	 */
	unixShell?: TaskUnixShell | null;
	/**
	 * The shell to run the task in when on a Windows machine.
	 *
	 * @default 'pwsh'
	 */
	windowsShell?: TaskWindowsShell | null;
}

/** Configures a task to be ran within the action pipeline. */
export interface PartialTaskConfig {
	/**
	 * Arguments to pass to the command when it's ran. Can be
	 * defined as a string, or a list of individual arguments.
	 */
	args?: PartialTaskArgs | null;
	/**
	 * The command or command line to execute when the task is ran.
	 * Supports the command name, with or without arguments. Can be
	 * defined as a string, or a list of individual arguments.
	 */
	command?: PartialTaskArgs | null;
	/**
	 * Other tasks that this task depends on, and must run to completion
	 * before this task is ran. Can depend on sibling tasks, or tasks in
	 * other projects, using targets.
	 */
	deps?: PartialTaskDependency[] | null;
	/** A human-readable description about the task. */
	description?: string | null;
	/**
	 * A mapping of environment variables that will be set when the
	 * task is ran.
	 */
	env?: Record<string, string> | null;
	/** Extends settings from a sibling task by ID. */
	extends?: string | null;
	/**
	 * Inputs and sources that will mark the task as affected when comparing
	 * against touched files. When not provided, all files within the project
	 * are considered an input. When an empty list, no files are considered.
	 * Otherwise, an explicit list of inputs are considered.
	 */
	inputs?: string[] | null;
	/**
	 * Marks the task as local only. Local tasks do not run in CI, do not have
	 * `options.cache` enabled, and are marked as `options.persistent`.
	 *
	 * @deprecated Use `preset` instead.
	 */
	local?: boolean | null;
	/** Options to control task inheritance and execution. */
	options?: PartialTaskOptionsConfig | null;
	/**
	 * Outputs that will be created when the task has successfully ran.
	 * When `cache` is enabled, the outputs will be persisted for subsequent runs.
	 */
	outputs?: string[] | null;
	/**
	 * The platform in which the task will be ran in. The platform determines
	 * available binaries, lookup paths, and more. When not provided, will
	 * be automatically detected.
	 *
	 * @default 'unknown'
	 */
	platform?: PlatformType | null;
	/** The preset to apply for the task. Will inherit default options. */
	preset?: TaskPreset | null;
	/**
	 * A script to run within a shell. A script is anything from a single command,
	 * to multiple commands (&&, etc), or shell specific syntax. Does not support
	 * arguments, merging, or inheritance.
	 */
	script?: string | null;
	/**
	 * The toolchain(s) in which the task will be ran in. The toolchain determines
	 * available binaries, lookup paths, and more. When not provided, will
	 * be automatically detected.
	 */
	toolchain?: string | string[] | null;
	/**
	 * The type of task, primarily used for categorical reasons. When not provided,
	 * will be automatically determined.
	 *
	 * @default 'test'
	 */
	type?: TaskType | null;
}

/**
 * Configures tasks and task related settings that'll be inherited by all
 * matching projects.
 * Docs: https://moonrepo.dev/docs/config/tasks
 */
export interface PartialInheritedTasksConfig {
	/** @default 'https://moonrepo.dev/schemas/tasks.json' */
	$schema?: string | null;
	/**
	 * Extends one or many task configuration files. Supports a relative
	 * file path or a secure URL.
	 */
	extends?: ExtendsFrom | null;
	/**
	 * A mapping of group IDs to a list of file paths, globs, and
	 * environment variables, that can be referenced from tasks.
	 */
	fileGroups?: Record<string, string[]> | null;
	/**
	 * Task dependencies that'll automatically be injected into every
	 * task that inherits this configuration.
	 */
	implicitDeps?: PartialTaskDependency[] | null;
	/**
	 * Task inputs that'll automatically be injected into every
	 * task that inherits this configuration.
	 */
	implicitInputs?: string[] | null;
	/** Default task options for all inherited tasks. */
	taskOptions?: PartialTaskOptionsConfig | null;
	/** A mapping of tasks by ID to parameters required for running the task. */
	tasks?: Record<string, PartialTaskConfig> | null;
}
