import fs from 'fs';
import path from 'path';
import { satisfies } from 'semver';
import vscode, { ShellExecution, Task, TaskScope } from 'vscode';
import { GraphVisualizerView } from './graphVisualizerView';
import type { Workspace } from './workspace';

export async function checkProject(
	project: string,
	workspace: Workspace,
	modifier?: (task: Task) => void,
) {
	if (!workspace.root || !workspace.binPath) {
		return;
	}

	const task = new Task(
		{ project, type: 'moon' },
		TaskScope.Workspace,
		`moon check ${project}`,
		'moon',
		new ShellExecution(
			workspace.binPath,
			[
				'check',
				project,
				'--log',
				vscode.workspace.getConfiguration('moon').get('logLevel', 'info'),
			],
			{
				cwd: workspace.root,
			},
		),
	);

	modifier?.(task);

	await vscode.tasks.executeTask(task);
}

export async function runTask(
	target: string,
	workspace: Workspace,
	modifier?: (task: Task) => void,
) {
	if (!workspace.root || !workspace.binPath || !target) {
		return;
	}

	const task = new Task(
		{ target, type: 'moon' },
		TaskScope.Workspace,
		`moon run ${target}`,
		'moon',
		new ShellExecution(
			workspace.binPath,
			[
				'run',
				...target.split(' '),
				'--log',
				vscode.workspace.getConfiguration('moon').get('logLevel', 'info'),
			],
			{
				cwd: workspace.root,
			},
		),
	);

	modifier?.(task);

	await vscode.tasks.executeTask(task);
}

export async function runTaskByInput(workspace: Workspace) {
	if (!workspace.root || !workspace.binPath) {
		return;
	}

	const target = await vscode.window.showInputBox({
		prompt: 'In the format of "scope:task" or ":task".',
		title: 'Target(s)',
	});

	if (target) {
		await runTask(target, workspace);
	}
}

export async function viewActionGraph(context: vscode.ExtensionContext, workspace: Workspace) {
	await new GraphVisualizerView(context, workspace, 'action-graph').renderPanel();
}

export async function viewProjectGraph(context: vscode.ExtensionContext, workspace: Workspace) {
	await new GraphVisualizerView(context, workspace, 'project-graph').renderPanel();
}

export async function viewTaskGraph(context: vscode.ExtensionContext, workspace: Workspace) {
	const version = await workspace.getMoonVersion();

	if (satisfies(version, '<1.30.0')) {
		await vscode.window.showErrorMessage(`Task graph requires moon >= 1.30.0, found ${version}`);

		return;
	}

	await new GraphVisualizerView(context, workspace, 'task-graph').renderPanel();
}

export async function appendSchemasConfig(context: vscode.ExtensionContext, workspace: Workspace) {
	const version = await workspace.getMoonVersion();

	if (satisfies(version, '<1.27.0')) {
		await vscode.window.showErrorMessage(`YAML schemas require moon >= 1.27.0, found ${version}`);

		return;
	}

	const vscodeDir = path.join(workspace.folder!.uri.fsPath, '.vscode');
	const settingsPath = path.join(vscodeDir, 'settings.json');

	const settings = (
		fs.existsSync(settingsPath) ? JSON.parse(fs.readFileSync(settingsPath, 'utf8')) : {}
	) as Record<string, unknown>;

	const schemas =
		typeof settings['yaml.schemas'] === 'object' && Boolean(settings['yaml.schemas'])
			? settings['yaml.schemas']
			: {};

	settings['yaml.schemas'] = {
		...schemas,
		'./.moon/cache/schemas/extensions.json': [
			path.join(workspace.rootPrefix, '.moon/extensions.yml'),
		],
		'./.moon/cache/schemas/project.json': ['**/moon.yml'],
		'./.moon/cache/schemas/tasks.json': [
			path.join(workspace.rootPrefix, '.moon/tasks.yml'),
			path.join(workspace.rootPrefix, '.moon/tasks/**/*.yml'),
		],
		'./.moon/cache/schemas/template.json': ['**/template.yml'],
		'./.moon/cache/schemas/toolchain.json': [
			path.join(workspace.rootPrefix, '.moon/toolchain.yml'),
		],
		'./.moon/cache/schemas/toolchains.json': [
			path.join(workspace.rootPrefix, '.moon/toolchains.yml'),
		],
		'./.moon/cache/schemas/workspace.json': [
			path.join(workspace.rootPrefix, '.moon/workspace.yml'),
		],
	};

	if (!fs.existsSync(vscodeDir)) {
		fs.mkdirSync(vscodeDir);
	}

	fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2));

	await vscode.window.showInformationMessage('Added `yaml.schemas` to `.vscode/settings.json`');
}
