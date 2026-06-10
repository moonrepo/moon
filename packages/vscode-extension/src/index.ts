import vscode from 'vscode';
import {
	appendSchemasConfig,
	runTaskByInput,
	viewActionGraph,
	viewProjectGraph,
	viewTaskGraph,
} from './commands';
import { LastRunProvider } from './lastRunView';
import { ProjectsProvider } from './projectsView';
import { Workspace } from './workspace';

export function activate(context: vscode.ExtensionContext) {
	const workspace = new Workspace();
	const projectsProvider = new ProjectsProvider(context, workspace, 'category');
	const tagsProvider = new ProjectsProvider(context, workspace, 'tag');
	const didChangeEmitter = new vscode.EventEmitter<void>();

	context.subscriptions.push(
		vscode.commands.registerCommand('moon.openSettings', () =>
			vscode.commands.executeCommand('workbench.action.openSettings', '@ext:moonrepo.moon-console'),
		),

		// Create commands
		vscode.commands.registerCommand('moon.runTaskByInput', () => runTaskByInput(workspace)),
		vscode.commands.registerCommand('moon.viewActionGraph', () =>
			viewActionGraph(context, workspace),
		),
		vscode.commands.registerCommand('moon.viewProjectGraph', () =>
			viewProjectGraph(context, workspace),
		),
		vscode.commands.registerCommand('moon.viewTaskGraph', () => viewTaskGraph(context, workspace)),
		vscode.commands.registerCommand('moon.appendSchemasConfig', () =>
			appendSchemasConfig(context, workspace),
		),

		// Create a tree view for all moon projects
		vscode.window.createTreeView('moonProjects', {
			showCollapseAll: true,
			treeDataProvider: projectsProvider,
		}),
		vscode.window.createTreeView('moonProjectsExternal', {
			showCollapseAll: true,
			treeDataProvider: projectsProvider,
		}),
		vscode.window.createTreeView('moonTags', {
			showCollapseAll: true,
			treeDataProvider: tagsProvider,
		}),

		// Create a webview for last run report
		vscode.window.registerWebviewViewProvider(
			'moonLastRun',
			new LastRunProvider(context, workspace),
		),

		// Support MCP
		vscode.lm.registerMcpServerDefinitionProvider('moonMcpProvider', {
			onDidChangeMcpServerDefinitions: didChangeEmitter.event,
			provideMcpServerDefinitions: async () => {
				return [
					new vscode.McpStdioServerDefinition('moon', 'moon', ['mcp'], {
						MOON_WORKSPACE_ROOT: '${workspaceFolder}',
					}),
				];
			},
		}),
	);
}

export function deactivate() {}
