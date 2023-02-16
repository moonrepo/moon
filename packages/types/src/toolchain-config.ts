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
	version: string | null;
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
	version: string | null;
	yarn: YarnConfig | null;
}

export interface TypeScriptConfig {
	createMissingConfig: boolean;
	projectConfigFileName: string;
	rootConfigFileName: string;
	rootOptionsConfigFileName: string;
	routeOutDirToCache: boolean;
	syncProjectReferences: boolean;
	syncProjectReferencesToPaths: boolean;
}

export interface ToolchainConfig {
	extends: string | null;
	node: NodeConfig | null;
	typescript: TypeScriptConfig | null;
}
