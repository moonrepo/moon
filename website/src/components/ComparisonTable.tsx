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
		feature: 'Core/CLI written in',
		support: {
			moon: 'Rust',
			nx: 'Node.js & Rust (for hot paths)',
			turborepo: 'Rust / Go',
		},
	},
	{
		feature: 'Plugins written in',
		support: {
			moon: 'WASM (any compatible language)',
			nx: 'TypeScript',
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
		feature: 'Project list configured in',
		support: {
			moon: '`.moon/workspace.yml`',
			nx: '`workspace.json` / `package.json` workspaces',
			turborepo: '`package.json` workspaces',
		},
	},
	{
		feature: 'Repo / folder structure',
		support: {
			moon: 'loose',
			nx: 'loose',
			turborepo: 'loose',
		},
	},
	{
		feature: 'Ignore file support',
		support: {
			moon: [SUPPORTED, 'via `hasher.ignorePatterns`'],
			nx: [SUPPORTED, '.nxignore'],
			turborepo: [SUPPORTED, 'via `--ignore`'],
		},
	},
	{
		feature: 'Supports dependencies inherited by all tasks',
		support: {
			moon: [SUPPORTED, 'via `implicitDeps`'],
			nx: [SUPPORTED, 'via `targetDefaults`'],
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
			nx: [SUPPORTED, 'via `plugins`'],
		},
	},
	{
		feature: 'Integrates with a version control system',
		support: {
			moon: [SUPPORTED, 'git'],
			nx: [SUPPORTED, 'git'],
			turborepo: [SUPPORTED, 'git'],
		},
	},
	{
		feature: 'Supports scaffolding / generators',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
];

const toolchainRows: Comparison[] = [
	{
		feature: 'Supported languages in task runner',
		support: {
			moon: 'All languages available on `PATH`',
			nx: 'All languages via plugins. OOTB TS/JS, existing plugins for Rust, Go, Dotnet and more',
			turborepo: 'JavaScriptTypeScript via `package.json` scripts',
		},
	},
	{
		feature: 'Supported dependency managers',
		support: {
			moon: 'npm, pnpm, yarn, bun',
			nx: 'npm, pnpm, yarn',
			turborepo: 'npm, pnpm, yarn',
		},
	},
	{
		feature: 'Supported toolchain languages (automatic dev envs)',
		support: {
			moon: 'Bun, Deno, Node.js, Rust',
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
		feature: 'Configures explicit language/dependency manager versions',
		support: {
			moon: SUPPORTED,
		},
	},
];

const projectsRows: Comparison[] = [
	{
		feature: 'Dependencies on other projects',
		support: {
			moon: [SUPPORTED, 'implicit from `package.json` or explicit in `moon.yml`'],
			nx: [
				SUPPORTED,
				'implicit from `package.json` or explicit in `project.json` and code imports/exports',
			],
			turborepo: [SUPPORTED, 'implicit from `package.json`'],
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
			moon: [SUPPORTED, 'app, lib, tool, automation, config, scaffold'],
			nx: [SUPPORTED, 'app, lib'],
		},
	},
	{
		feature: 'Project tech stack',
		support: {
			moon: [SUPPORTED, 'frontend, backend, infra, systems'],
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
		feature: 'Tags and scopes (boundaries)',
		support: {
			moon: [SUPPORTED, 'native for all languages'],
			nx: [SUPPORTED, 'boundaries via ESLint (TS and JS), tags for filtering for all languages'],
		},
	},
];

const tasksRows: Comparison[] = [
	{
		feature: 'Known as',
		support: {
			moon: 'tasks',
			nx: 'targets',
			turborepo: 'tasks',
		},
	},
	{
		feature: 'Defines tasks in',
		support: {
			moon: '`moon.yml` or `package.json` scripts',
			nx: '`nx.json`, `project.json` or `package.json` scripts',
			turborepo: '`package.json` scripts',
		},
	},
	{
		feature: 'Run a single task with',
		support: {
			moon: '`moon run project:task`',
			nx: '`nx target project` or `nx run project:target`',
			turborepo: '`turbo run task --filter=project`',
		},
	},
	{
		feature: 'Run multiple tasks with',
		support: {
			moon: '`moon run :task` or `moon run a:task b:task` or `moon check`',
			nx: '`nx run-many -t task1 task2 task3`',
			turborepo: '`turbo run task` or `turbo run a b c`',
		},
	},
	{
		feature: 'Run tasks based on a query/filter',
		support: {
			moon: '`moon run :task --query "..."`',
			nx: '`nx run-many -t task -p "tag:.." -p "dir/*" -p "name*" -p "!negation"`',
		},
	},
	{
		feature: 'Can define tasks globally',
		support: {
			moon: [SUPPORTED, 'with `.moon/tasks.yml`'],
			nx: [PARTIALLY_SUPPORTED, 'with `targetDefaults`'],
		},
	},
	{
		feature: 'Merges or overrides global tasks',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
		},
	},
	{
		feature: 'Runs a command with args',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: [PARTIALLY_SUPPORTED, 'within the script'],
		},
	},
	{
		feature: 'Runs commands from',
		support: {
			moon: 'project or workspace root',
			nx: 'current working directory, or wherever desired via config',
			turborepo: 'project root',
		},
	},
	{
		feature: 'Supports pipes, redirects, etc, in configured tasks',
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
		feature: 'Can provide extra params for task dependencies',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: NOT_SUPPORTED,
		},
	},
	{
		feature: 'Can mark a task dependency as optional',
		support: {
			moon: [SUPPORTED, 'via `optional`'],
		},
	},
	{
		feature: 'Can depend on arbitrary or unrelated tasks',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: [NOT_SUPPORTED, 'dependent projects only'],
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
			nx: [SUPPORTED, 'via `parallel=1`'],
			turborepo: [SUPPORTED, 'via `concurrency=1`'],
		},
	},
	{
		feature: 'File groups',
		support: {
			moon: SUPPORTED,
			nx: [SUPPORTED, 'via `namedInputs`'],
		},
	},
	{
		feature: 'Environment variables',
		support: {
			moon: [SUPPORTED, 'via `env`, `envFile`'],
			nx: [SUPPORTED, 'automatically via `.env` files and/or inherited from shell'],
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
			moon: [SUPPORTED, 'files, globs'],
			nx: [SUPPORTED, 'files, globs'],
			turborepo: [SUPPORTED, 'files, globs'],
		},
	},
	{
		feature: 'Output logging style',
		support: {
			moon: [SUPPORTED, 'via `outputStyle`'],
			nx: [SUPPORTED, 'via `--output-style`'],
			turborepo: [SUPPORTED, 'via `outputMode`'],
		},
	},
	{
		feature: 'Custom hash inputs',
		support: {
			nx: [SUPPORTED, 'via `runtime` inputs'],
			turborepo: [SUPPORTED, 'via `globalDependencies`'],
		},
	},
	{
		feature: 'Token substitution',
		support: {
			moon: [SUPPORTED, 'token functions and variable syntax'],
			nx: [
				SUPPORTED,
				'`{workspaceRoot}`, `{projectRoot}`, `{projectName}`, arbitrary patterns `namedInputs`',
			],
		},
	},
	{
		feature: 'Configuration presets',
		support: {
			moon: [SUPPORTED, 'via task `extends`'],
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
			moon: 'action pipeline',
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
			nx: [SUPPORTED, 'when flakiness detected on Nx Cloud'],
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
		feature: 'Can generate run reports',
		support: {
			moon: SUPPORTED,
			nx: [SUPPORTED, 'free in Nx Cloud & GitHub App Comment'],
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Continuous integration (CI) support',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: PARTIALLY_SUPPORTED,
		},
	},
	{
		feature: 'Continuous deployment (CD) support',
		support: {
			nx: [PARTIALLY_SUPPORTED, 'via `nx release`'],
		},
	},
	{
		feature: 'Remote / cloud caching and syncing',
		support: {
			moon: [SUPPORTED, 'with Bazel REAPI (free / paid)'],
			nx: [SUPPORTED, 'with nx.app Nx Cloud (free / paid)'],
			turborepo: [SUPPORTED, 'requires a Vercel account (free)'],
		},
	},
];

const generatorRows: Comparison[] = [
	{
		feature: 'Known as',
		support: {
			moon: 'generator',
			nx: 'generator',
			turborepo: 'generator',
		},
	},
	{
		feature: 'Templates are configured with a schema',
		support: {
			moon: [SUPPORTED, 'via `template.yml`'],
			nx: SUPPORTED,
		},
	},
	{
		feature: 'Template file extensions (optional)',
		support: {
			moon: [SUPPORTED, '.tera, .twig'],
			nx: [SUPPORTED, 'fully under user control, built in utility for .ejs templates'],
			turborepo: [SUPPORTED, '.hbs'],
		},
	},
	{
		feature: 'Template files support frontmatter',
		support: {
			moon: SUPPORTED,
			nx: [SUPPORTED, 'fully under user control'],
		},
	},
	{
		feature: 'Creates/copies files to destination',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Updates/merges with existing files',
		support: {
			moon: [SUPPORTED, 'JSON/YAML only'],
			nx: [SUPPORTED, 'via TypeScript/JavaScript plugins'],
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Renders with a template engine',
		support: {
			moon: [SUPPORTED, 'via Tera'],
			nx: [SUPPORTED, 'fully under user control, built in utility for .ejs templates'],
			turborepo: [SUPPORTED, 'via Handlebars'],
		},
	},
	{
		feature: 'Variable interpolation in file content',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Variable interpolation in file paths',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Can define variable values via interactive prompts',
		support: {
			moon: SUPPORTED,
			nx: [SUPPORTED, 'using JSON schema'],
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Can define variable values via command line args',
		support: {
			moon: SUPPORTED,
			nx: [SUPPORTED, 'using JSON schema'],
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Supports dry runs',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
		},
	},
	{
		feature: 'Supports render helpers, filters, and built-ins',
		support: {
			moon: SUPPORTED,
			nx: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Generators can compose other generators',
		support: {
			moon: [SUPPORTED, 'via `extends`'],
			nx: [SUPPORTED, 'fully under user control, author in TypeScript/JavaScript'],
			turborepo: [SUPPORTED, 'using JavaScript'],
		},
	},
];

const otherSystemRows: Comparison[] = [
	{
		feature: 'Can send webhooks for critical pipeline events',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Generates run reports with granular stats/metrics',
		support: {
			moon: SUPPORTED,
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Can define and manage code owners',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can generate a `CODEOWNERS` file',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can define and manage VCS (git) hooks',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Supports git worktrees',
		support: {
			moon: SUPPORTED,
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
			nx: SUPPORTED,
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
	{
		feature: 'Can sync dependencies as `paths` to `tsconfig.json`',
		support: {
			moon: SUPPORTED,
		},
	},
	{
		feature: 'Can route `outDir` to a shared cached in `tsconfig.json`',
		support: {
			moon: SUPPORTED,
		},
	},
];

const dockerRows: Comparison[] = [
	{
		feature: 'Efficient scaffolding for Dockerfile layer caching',
		support: {
			moon: SUPPORTED,
			nx: [SIMILARLY_SUPPORTED, 'via custom generator'],
			turborepo: SUPPORTED,
		},
	},
	{
		feature: 'Automatic production-only dependency installation',
		support: {
			moon: SUPPORTED,
			nx: [PARTIALLY_SUPPORTED, 'generated automatically by first party plugin'],
		},
	},
	{
		feature: 'Environment pruning to reduce image/container sizes',
		support: {
			moon: SUPPORTED,
			turborepo: SUPPORTED,
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

export const DockerTable = createTable(dockerRows);
export const GeneratorTable = createTable(generatorRows);
export const JavaScriptTable = createTable(javascriptRows);
export const OtherSystemsTable = createTable(otherSystemRows);
export const ProjectsTable = createTable(projectsRows);
export const TasksTable = createTable(tasksRows);
export const TaskRunnerTable = createTable(taskRunnerRows);
export const ToolchainTable = createTable(toolchainRows);
export const WorkspaceTable = createTable(workspaceRows);
