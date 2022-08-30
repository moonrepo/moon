export interface HasherConfig {
	optimization: 'accuracy' | 'performance';
}

export type NodeVersionFormat =
	| 'file'
	| 'link'
	| 'star'
	| 'version-caret'
	| 'version-tilde'
	| 'version'
	| 'workspace-caret'
	| 'workspace-tilde'
	| 'workspace';

export interface NodePackageManagerConfig {
	version: string;
}

export interface NodeConfig {
	addEnginesConstraint: boolean;
	aliasPackageNames: 'name-and-scope' | 'name-only' | null;
	dedupeOnLockfileChange: boolean;
	dependencyVersionFormat: NodeVersionFormat;
	inferTasksFromScripts: boolean;
	npm: NodePackageManagerConfig;
	packageManager: 'npm' | 'pnpm' | 'yarn';
	pnpm: NodePackageManagerConfig | null;
	syncProjectWorkspaceDependencies: boolean;
	syncVersionManagerConfig: 'nodenv' | 'nvm' | null;
	version: string;
	yarn: NodePackageManagerConfig | null;
}

export interface RunnerConfig {
	cacheLifetime: string;
	implicitInputs: string[];
	inheritColorsForPipedTasks: boolean;
	logRunningCommand: boolean;
}

export interface TypeScriptConfig {
	createMissingConfig: boolean;
	projectConfigFileName: string;
	rootConfigFileName: string;
	rootOptionsConfigFileName: string;
	syncProjectReferences: boolean;
}

export interface VcsConfig {
	manager: 'git' | 'svn';
	defaultBranch: string;
}

export interface WorkspaceConfig {
	extends: string | null;
	hasher: HasherConfig;
	node: NodeConfig | null;
	projects: Record<string, string> | string[];
	runner: RunnerConfig;
	typescript: TypeScriptConfig | null;
	vcs: VcsConfig;
}
