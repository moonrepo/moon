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
			moon: '`project.yml` or `package.json` scripts',
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
			moon: [SUPPORTED, 'files, globs'],
			nx: [SUPPORTED, 'files, globs, env vars, runtime'],
			turborepo: [SUPPORTED, 'files, globs'],
		},
	},
	{
		feature: 'Outputs',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Output logging style',
		support: {
			moon: PARTIALLY_SUPPORTED,
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
];

const techRows: Comparison[] = [
	{
		feature: 'git',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'svn',
		support: {
			moon: PARTIALLY_SUPPORTED,
		},
	},
	{
		feature: 'mercurial',
		support: {},
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

	const markdown = Array.isArray(content) ? content.join(' ') : content;

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
						<td>{row.feature}</td>
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

export const TasksTable = createTable(tasksRows);
export const TaskRunnerTable = createTable(taskRunnerRows);
export const TechTable = createTable(techRows);
