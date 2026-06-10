import fs from 'fs';
import path from 'path';
import execa from 'execa';
import vscode from 'vscode';

export function isRealBin(binPath: string): boolean {
	const stats = fs.statSync(binPath);

	// When in the moonrepo/moon repository, the binary is actually fake,
	// so we need to account for that!
	return stats.isFile() && stats.size > 100;
}

export class Workspace {
	// Current moon binary path
	binPath: string | null = null;

	// Current moon config directory path
	configDir: string | null = null;

	// Current moon config directory name relative to workspace root
	// Supports either ".moon" or ".config/moon"
	configDirName: string = '.moon';

	// Current vscode workspace folder
	folder: vscode.WorkspaceFolder | null = null;

	// Channel for logging
	logger: vscode.LogOutputChannel;

	// Current moon workspace root
	root: string | null = null;

	rootPrefix: string = '';

	onDidChangeWorkspaceEmitter: vscode.EventEmitter<vscode.WorkspaceFolder | null>;

	disposables: vscode.Disposable[] = [];

	constructor() {
		this.logger = vscode.window.createOutputChannel('moon', { log: true });
		this.onDidChangeWorkspaceEmitter = new vscode.EventEmitter<vscode.WorkspaceFolder | null>();

		// Find moon workspace from default editor
		if (vscode.window.activeTextEditor) {
			this.findRoot(vscode.window.activeTextEditor.document.uri);
		} else {
			this.findDefaultRoot();
		}

		// When an editor is changed, attempt to find the moon workspace
		vscode.window.onDidChangeActiveTextEditor((editor) => {
			if (editor && editor.document.uri.scheme === 'file') {
				this.findRoot(editor.document.uri);
			}
		});
	}

	onDidChangeWorkspace(
		listener: (folder: vscode.WorkspaceFolder | null) => vscode.Disposable | void,
	) {
		this.onDidChangeWorkspaceEmitter.event((folder) => {
			const disposable = listener(folder);

			if (disposable) {
				this.disposables.push(disposable);
			}
		});
	}

	fireDidChangeWorkspace() {
		// Remove previous watchers
		this.disposables.forEach((disposable) => {
			disposable.dispose();
		});

		// Emit and add new watchers
		const { folder } = this;

		// Run in a timeout to ensure listeners have been registered,
		// otherwise this does nothing and the editor feels broken
		setTimeout(() => {
			this.onDidChangeWorkspaceEmitter.fire(folder);
		}, 0);
	}

	findDefaultRoot() {
		for (const folder of vscode.workspace.workspaceFolders ?? []) {
			this.findRoot(folder.uri);

			if (this.root) {
				break;
			}
		}
	}

	findRoot(openUri: vscode.Uri) {
		if (openUri.fsPath === 'moonrepo.moon-console.moon') {
			return;
		}

		if (this.root && openUri.fsPath.startsWith(this.root)) {
			return;
		}

		this.folder = null;
		this.root = null;
		this.rootPrefix = '';
		this.configDir = null;
		this.configDirName = '.moon';
		this.binPath = null;

		this.logger.appendLine(`Attempting to find a VSC workspace folder for ${openUri.fsPath}`);

		const workspaceFolder = vscode.workspace.getWorkspaceFolder(openUri);

		if (workspaceFolder) {
			this.folder = workspaceFolder;
			this.logger.appendLine(`Found workspace folder ${workspaceFolder.uri.fsPath}`);
			this.logger.appendLine('Attempting to find a moon installation');

			const rootPrefixes = vscode.workspace
				.getConfiguration('moon')
				.get('rootPrefixes', [] as string[]);

			// Always include "." at the end
			rootPrefixes.push('.');

			let foundRoot = false;

			for (const prefix of rootPrefixes) {
				const candidateRoot = path.join(workspaceFolder.uri.fsPath, prefix);

				// Moon v1 config dir: <workspace>/<prefix>/.moon
				const v1Path = path.join(candidateRoot, '.moon');

				// Moon v2 config dir: <workspace>/<prefix>/.config/moon
				const v2Path = path.join(candidateRoot, '.config', 'moon');

				const candidateDirs: Array<{ dir: string; name: string }> = [
					{ dir: v1Path, name: '.moon' },
					{ dir: v2Path, name: path.join('.config', 'moon') },
				];

				for (const { dir, name } of candidateDirs) {
					if (fs.existsSync(dir) && fs.statSync(dir).isDirectory()) {
						// IMPORTANT:
						// The workspace root is the candidate root, not dirname(dir).
						// For ".config/moon", dirname(dir) would incorrectly be "<root>/.config".
						this.root = candidateRoot;
						this.rootPrefix = prefix;
						this.configDir = dir;
						this.configDirName = name;
						this.binPath = this.findMoonBin();

						this.logger.appendLine(
							`Found moon workspace root at ${this.root} (config dir: ${this.configDir})`,
						);

						if (this.binPath) {
							this.logger.appendLine(`Found moon binary at ${this.binPath}`);
						}

						foundRoot = true;
						break;
					}
				}

				if (foundRoot) {
					break;
				}
			}

			this.fireDidChangeWorkspace();

			if (!foundRoot) {
				this.logger.appendLine('Did not find a moon installation, disabling');
			}
		} else {
			this.logger.appendLine('Did not find a workspace folder, disabling moon');
		}

		// Update context
		void vscode.commands.executeCommand('setContext', 'moon.inWorkspaceRoot', this.root !== null);
		void vscode.commands.executeCommand(
			'setContext',
			'moon.hasBinary',
			this.binPath !== null && isRealBin(this.binPath),
		);
	}

	findMoonBin(): string | null {
		if (!this.root) {
			return null;
		}

		const isWindows = process.platform === 'win32';
		let binPath = vscode.workspace.getConfiguration('moon').get('binPath', 'moon');

		if (isWindows && !binPath.endsWith('.exe')) {
			binPath += '.exe';
		}

		if (!path.isAbsolute(binPath)) {
			binPath = path.join(this.root, binPath);
		}

		if (fs.existsSync(binPath) && fs.statSync(binPath).isFile()) {
			return binPath;
		}

		const paths = process.env.PATH?.split(isWindows ? ';' : ':') ?? [];
		const binName = isWindows ? 'moon.exe' : 'moon';

		for (const dir of paths) {
			const globalBin = path.join(dir, binName);

			if (fs.existsSync(globalBin) && fs.statSync(globalBin).isFile()) {
				return globalBin;
			}
		}

		return null;
	}

	async execMoon(args: string[]): Promise<string> {
		if (!args.includes('--json')) {
			args.push('--log', vscode.workspace.getConfiguration('moon').get('logLevel', 'info'));
		}

		try {
			const result = await execa(this.binPath ?? 'moon', args, {
				cwd: this.root ?? process.cwd(),
			});

			return result.stdout;
		} catch (error: unknown) {
			console.error(error);

			throw error;
		}
	}

	getMoonDirPath(file: string, withPrefix: boolean = true): string {
		return path.join(withPrefix ? this.rootPrefix : '.', this.configDirName, file);
	}

	getMoonConfigPath(file: string): string {
		return this.configDir ? path.join(this.configDir, file) : this.getMoonDirPath(file, false);
	}

	async getMoonVersion(): Promise<string> {
		try {
			const result = await this.execMoon(['--version']);

			// Output is: moon 0.0.0
			const parts = result.split(' ');

			return parts.at(-1)!;
		} catch {
			return '0.0.0';
		}
	}
}
