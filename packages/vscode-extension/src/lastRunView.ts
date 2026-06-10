import fs from 'fs';
import path from 'path';
import vscode from 'vscode';
import { formatDuration, prepareReportActions } from '@moonrepo/report';
import type { RunReport } from '@moonrepo/types';
import type { Workspace } from './workspace';

const SLOW_THRESHOLD_SECS = 120;

export class LastRunProvider implements vscode.WebviewViewProvider {
	context: vscode.ExtensionContext;

	view?: vscode.WebviewView;

	workspace: Workspace;

	constructor(context: vscode.ExtensionContext, workspace: Workspace) {
		this.context = context;
		this.workspace = workspace;

		workspace.onDidChangeWorkspace((folder) => {
			this.renderView();

			if (!folder) {
				return undefined;
			}

			// When the report is changed, refresh view
			const watcher = vscode.workspace.createFileSystemWatcher(
				new vscode.RelativePattern(folder.uri, workspace.getMoonDirPath('cache/runReport.json')),
			);

			watcher.onDidChange(this.renderView, this);
			watcher.onDidCreate(this.renderView, this);
			watcher.onDidDelete(this.renderView, this);

			return watcher;
		});
	}

	resolveWebviewView(webviewView: vscode.WebviewView): Thenable<void> | void {
		webviewView.webview.options = {
			enableScripts: true,
			localResourceRoots: [this.context.extensionUri],
		};

		this.view = webviewView;
		this.renderView();
	}

	formatComments(comments: string[]): string {
		if (comments.length === 0) {
			return '';
		}

		const content = comments
			.map((comment) => comment.replaceAll(/\*\*(\w+)\*\*/g, (_, match) => `<b>${match}</b>`))
			.join(', ');

		return `| ${content}`;
	}

	renderHtml(content: string) {
		const cssUri = this.view?.webview.asWebviewUri(
			vscode.Uri.joinPath(this.context.extensionUri, 'assets/webview.css'),
		);

		return `<!DOCTYPE html>
			<html lang="en">
				<head>
					<meta charset="UTF-8">
					<meta name="viewport" content="width=device-width, initial-scale=1.0">
					<title>moon - Last run report</title>
					<script type="module" src="https://unpkg.com/@vscode/webview-ui-toolkit@latest"></script>
					<link href="${cssUri}" rel="stylesheet">
				</head>
				<body class="body">
					${content}
				</body>
			</html>`;
	}

	renderView() {
		if (!this.view?.webview || !this.workspace.root) {
			return;
		}

		const runReportPath = path.join(
			this.workspace.root,
			this.workspace.getMoonDirPath('cache/runReport.json', false),
		);

		if (fs.existsSync(runReportPath)) {
			const report = JSON.parse(fs.readFileSync(runReportPath, 'utf8')) as RunReport;

			const tableRows = prepareReportActions(report, SLOW_THRESHOLD_SECS).map(
				(action) => `
					<tr>
						<td>
							<span class="action-icon">${action.icon}</span>
						</td>
						<td>
							<span class="action-label">${action.label}</span><br />
							${action.time} | ${action.status} ${this.formatComments(action.comments)}
						</td>
					</tr>
				`,
			);

			this.view.webview.html = this.renderHtml(`Finished ${
				report.context.primaryTargets.length > 0
					? `<b>${report.context.primaryTargets.join(', ')}</b> `
					: ''
			}in ${formatDuration(report.duration)}.

				<br /><br />

				<table>
					${tableRows.join('\n')}
				</table>
			`);
		} else {
			this.view.webview.html = this.renderHtml(`
				No run report found. Run a task through the projects view or on the command line.
			`);
		}
	}
}
