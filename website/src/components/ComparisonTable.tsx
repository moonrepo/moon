import React from 'react';
import { Markdown } from 'docusaurus-plugin-typedoc-api/lib/components/Markdown';

const SUPPORTED = '🟩';
const PARTIALLY_SUPPORTED = '🟨';
const SIMILARLY_SUPPORTED = '🟦';
const NOT_SUPPORTED = '🟥';

type Comparable = 'moon' | 'nx' | 'turborepo';

interface Comparison {
	feature: string;
	support: Partial<Record<Comparable, string[] | string>>;
}

const headers: Comparable[] = ['moon', 'nx', 'turborepo'];

const workspaceRows: Comparison[] = [
	{
		feature: 'Written in',
		support: {
			moon: 'Rust',
			nx: 'Node.js',
			turborepo: 'Go',
		},
	},
	{
		feature: 'Workspace configured with',
		support: {
			moon: '`.moon/workspace.yml`',
			nx: '`nx.json`',
			turborepo: '`turbo.json`',
		},
	},
	{
		feature: 'Projects configured in',
		support: {
			moon: '`.moon/workspace.yml`',
			nx: '`workspace.json`',
			turborepo: '`package.json` workspaces',
		},
	},
	{
		feature: 'Repo / folder structure',
		support: {
			moon: 'loose',
			nx: 'strict',
			turborepo: 'loose',
		},
	},
	{
		feature: 'Ignore file support',
		support: {
			nx: [SUPPORTED, '.nxignore'],
		},
	},
	{
		feature: 'Supports inputs inherited by all tasks',
		support: {
			moon: [SUPPORTED, 'via `implicitInputs`'],
			nx: [SUPPORTED, 'via `implicitDependencies`'],
			turborepo: [SUPPORTED, 'via `globalDependencies`'],
		},
	},
	{
		feature: 'Supports tasks inherited by all projects',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Integrates with a version control system',
		support: {
			moon: [SUPPORTED, 'git', PARTIALLY_SUPPORTED, 'svn'],
			nx: [SUPPORTED, 'git'],
			turborepo: [SUPPORTED, 'git'],
		},
	},
	{
		feature: 'Supports scaffolding / generators',
		support: {
			nx: SUPPORTED,
		},
	},
];

const toolchainRows: Comparison[] = [
	{
		feature: 'Supported languages',
		support: {
			moon: 'Bash, Batch, JavaScript, TypeScript',
			nx: 'JavaScript, TypeScript',
			turborepo: 'JavaScript, TypeScript',
		},
	},
	{
		feature: 'Supported package managers',
		support: {
			moon: 'npm, pnpm, yarn',
			nx: 'npm, pnpm, yarn',
			turborepo: 'npm, pnpm, yarn',
		},
	},
	{
		feature: 'Has a built-in toolchain',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Downloads and installs languages (when applicable)',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Configures explicit language/package manager versions',
		support: {
			moon: SUPPORTED,
		},
	},
];

const projectsRows: Comparison[] = [
	{
		feature: 'Dependencies on other projects',
		support: {
			moon: [SUPPORTED, 'explicitly defined or migrated from `package.json`'],
			nx: [SUPPORTED, 'inferred from `package.json` or via `implicitDependencies`'],
			turborepo: [SUPPORTED, 'inferred from `package.json`'],
		},
	},
	{
		feature: 'Ownership metadata',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Primary programming language',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Project type (app, lib, etc)',
		support: {
			moon: [SUPPORTED, 'app, lib, tool'],
			nx: [SUPPORTED, 'app, lib'],
		},
	},
	{
		feature: 'Project-level file groups',
		support: {
			moon: SUPPORTED,
			nx: [SUPPORTED, 'via `namedInputs`'],
		},
	},
	{
		feature: 'Project-level tasks',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Tags and scopes',
		support: {
			nx: SUPPORTED,
		},
	},
];

const tasksRows: Comparison[] = [
	{
		feature: 'Known as',
		support: {
			moon: 'tasks',
			nx: 'targets, executors',
			turborepo: 'tasks',
		},
	},
	{
		feature: 'Defines tasks in',
		support: {
			moon: '`moon.yml` or `package.json` scripts',
			nx: '`project.json` or `package.json` scripts',
			turborepo: '`package.json` scripts',
		},
	},
	{
		feature: 'Run a single task with',
		support: {
			moon: '`moon run project:task`',
			nx: '`nx run project:target`',
			turborepo: '`turbo run task --filter=project`',
		},
	},
	{
		feature: 'Run multiple tasks with',
		support: {
			moon: '`moon run :task`',
			nx: '`nx run-many --target=target`',
			turborepo: '`turbo run task`',
		},
	},
	{
		feature: 'Can define tasks globally',
		support: {
			moon: [SUPPORTED, 'with `.moon/project.yml`'],
		},
	},
	{
		feature: 'Merges or overrides global tasks',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Runs a command with args',
		support: {
			moon: SUPPORTED,
			nx: [SIMILARLY_SUPPORTED, 'behind an executor abstraction'],
			turborepo: [PARTIALLY_SUPPORTED, 'within the script'],
		},
	},
	{
		feature: 'Runs commands from',
		support: {
			moon: 'project or workspace root',
			nx: 'workspace root',
			turborepo: 'project root',
		},
	},
	{
		feature: 'Supports pipes, redirects, etc',
		support: {
			moon: [PARTIALLY_SUPPORTED, 'encapsulated in a file'],
			nx: [PARTIALLY_SUPPORTED, 'within the executor or script'],
			turborepo: [PARTIALLY_SUPPORTED, 'within the script'],
		},
	},
	{
		feature: 'Dependencies on other tasks',
		support: {
			moon: [SUPPORTED, 'via `deps`'],
			nx: [SUPPORTED, 'via `dependsOn`'],
			turborepo: [SUPPORTED, 'via `dependsOn`'],
		},
	},
	{
		feature: 'Runs task dependencies in parallel',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Can run task dependencies in serial',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'File groups',
		support: {
			moon: SUPPORTED,
			nx: [SIMILARLY_SUPPORTED, 'via `namedInputs`'],
		},
	},
	{
		feature: 'Environment variables',
		support: {
			moon: [SUPPORTED, 'via `env`'],
			nx: [PARTIALLY_SUPPORTED, 'within the executor or script'],
			turborepo: [PARTIALLY_SUPPORTED, 'within the script'],
		},
	},
	{
		feature: 'Inputs',
		support: {
			moon: [SUPPORTED, 'files, globs, env vars'],
			nx: [SUPPORTED, 'files, globs, env vars, runtime'],
			turborepo: [SUPPORTED, 'files, globs'],
		},
	},
	{
		feature: 'Outputs',
		support: {
			moon: [SUPPORTED, 'files'],
			nx: [SUPPORTED, 'files, globs'],
			turborepo: [SUPPORTED, 'files'],
		},
	},
	{
		feature: 'Output logging style',
		support: {
			nx: [SUPPORTED, 'via `--output-style`'],
			turborepo: [SUPPORTED, 'via `outputMode`'],
		},
	},
	{
		feature: 'Custom hash inputs',
		support: {
			nx: [SUPPORTED, 'via `runtimeCacheInputs`'],
			turborepo: [SUPPORTED, 'via `globalDependencies`'],
		},
	},
	{
		feature: 'Token substitution',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Configuration presets',
		support: {
			nx: [SUPPORTED, 'via `configurations`'],
		},
	},
	{
		feature: 'Configurable options',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
];

const taskRunnerRows: Comparison[] = [
	{
		feature: 'Known as',
		support: {
			moon: 'action or task runner',
			nx: 'task runner',
			turborepo: 'pipeline',
		},
	},
	{
		feature: 'Generates a dependency graph',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Runs in topological order',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Automatically retries failed tasks',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Caches task outputs via a unique hash',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Can customize the underlying runner',
		support: {
			nx: SUPPORTED,
		},
	},
	{
		feature: 'Can profile running tasks',
		support: {
			moon: [SUPPORTED, 'cpu, heap'],
			nx: [SUPPORTED, 'cpu'],
			turborepo: [SUPPORTED, 'cpu'],
		},
	},
	{
		feature: 'Continuous integration (CI) support',
		support: {
			moon: SUPPORTED,
			nx: PARTIALLY_SUPPORTED,
			turborepo: PARTIALLY_SUPPORTED,
		},
	},
	{
		feature: 'Continuous deployment (CD) support',
		support: {},
	},
	{
		feature: 'Remote / cloud caching and syncing',
		support: {
			nx: [SUPPORTED, 'with Nx cloud (paid)'],
			turborepo: [SUPPORTED, 'requires a Vercel account (free)'],
		},
	},
];

const javascriptRows: Comparison[] = [
	{
		feature: 'Will automatically install node modules when lockfile changes',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can automatically dedupe when lockfile changes',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can alias `package.json` names for projects',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can add `engines` constraint to root `package.json`',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can sync version manager configs (`.nvmrc`, etc)',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can sync cross-project dependencies to `package.json`',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can sync project references to applicable `tsconfig.json`',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can auto-create missing `tsconfig.json`',
		support: {
			moon: SUPPORTED,
		},
	},
];

function isSupported(data?: string[] | string): boolean {
	if (!data) {
		return false;
	}

	return (
		data === SUPPORTED ||
		(data !== NOT_SUPPORTED && data !== PARTIALLY_SUPPORTED && data !== SIMILARLY_SUPPORTED) ||
		// eslint-disable-next-line @typescript-eslint/prefer-string-starts-ends-with
		(Array.isArray(data) && data[0] === SUPPORTED)
	);
}

function Cell({ content }: { content: string[] | string | undefined }) {
	if (!content) {
		return <>{NOT_SUPPORTED}</>;
	}

	// nbsp
	const markdown = Array.isArray(content) ? content.join(' \u00A0') : content;

	if (markdown === SUPPORTED || markdown === PARTIALLY_SUPPORTED) {
		return <>{markdown}</>;
	}

	return <Markdown content={markdown} />;
}

function Table({ rows }: { rows: Comparison[] }) {
	return (
		<table width="100%">
			<thead>
				<tr>
					<th />
					{headers.map((header) => (
						<th key={header} align="center">
							{header} ({rows.filter((row) => isSupported(row.support[header])).length})
						</th>
					))}
				</tr>
			</thead>
			<tbody>
				{rows.map((row) => (
					<tr key={row.feature}>
						<td>
							<Markdown content={row.feature} />
						</td>
						{headers.map((header) => (
							<td key={row.feature + header} align="center">
								<Cell content={row.support[header]} />
							</td>
						))}
					</tr>
				))}
			</tbody>
		</table>
	);
}

function createTable(rows: Comparison[]) {
	return () => <Table rows={rows} />;
}

export const JavaScriptTable = createTable(javascriptRows);
export const ProjectsTable = createTable(projectsRows);
export const TasksTable = createTable(tasksRows);
export const TaskRunnerTable = createTable(taskRunnerRows);
export const ToolchainTable = createTable(toolchainRows);
export const WorkspaceTable = createTable(workspaceRows);
