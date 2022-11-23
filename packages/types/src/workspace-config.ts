export interface GeneratorConfig {
	templates: string[];
}

export interface HasherConfig {
	optimization: 'accuracy' | 'performance';
}

export interface NotifierConfig {
	webhookUrl: string | null;
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

export interface YarnConfig extends NodePackageManagerConfig {
	plugins: string[];
}

export interface NodeConfig {
	addEnginesConstraint: boolean;
	aliasPackageNames: 'name-and-scope' | 'name-only' | null;
	binExecArgs: string[];
	dedupeOnLockfileChange: boolean;
	dependencyVersionFormat: NodeVersionFormat;
	inferTasksFromScripts: boolean;
	npm: NodePackageManagerConfig;
	packageManager: 'npm' | 'pnpm' | 'yarn';
	pnpm: NodePackageManagerConfig | null;
	syncProjectWorkspaceDependencies: boolean;
	syncVersionManagerConfig: 'nodenv' | 'nvm' | null;
	version: string;
	yarn: YarnConfig | null;
}

export interface RunnerConfig {
	cacheLifetime: string;
	implicitDeps: string[];
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
	defaultBranch: string;
	manager: 'git' | 'svn';
	remoteCandidates: string[];
}

export interface WorkspaceConfig {
	extends: string | null;
	generator: GeneratorConfig;
	hasher: HasherConfig;
	notifier: NotifierConfig;
	projects:
		| Record<string, string>
		| string[]
		| { globs: string[]; sources: Record<string, string> };
	runner: RunnerConfig;
	vcs: VcsConfig;
	// Languages
	node: NodeConfig | null;
	typescript: TypeScriptConfig | null;
}
