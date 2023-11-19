// Automatically generated by schematic. DO NOT MODIFY!

/* eslint-disable */

/** Docs: https://moonrepo.dev/docs/config/toolchain#bun */
export interface PartialBunConfig {
	plugin?: string | null;
	version?: string | null;
}

export interface PartialBinConfig {
	bin?: string | null;
	force?: boolean | null;
	local?: boolean | null;
	name?: string | null;
}

export type PartialBinEntry = string | PartialBinConfig;

/** Docs: https://moonrepo.dev/docs/config/toolchain#deno */
export interface PartialDenoConfig {
	bins?: PartialBinEntry[] | null;
	/** @default 'deps.ts' */
	depsFile?: string | null;
	lockfile?: boolean | null;
	plugin?: string | null;
}

export type NodeProjectAliasFormat = 'name-and-scope' | 'name-only';

export interface PartialBunpmConfig {
	plugin?: string | null;
	version?: string | null;
}

export type NodeVersionFormat =
	| 'file'
	| 'link'
	| 'star'
	| 'version'
	| 'version-caret'
	| 'version-tilde'
	| 'workspace'
	| 'workspace-caret'
	| 'workspace-tilde';

export interface PartialNpmConfig {
	plugin?: string | null;
	version?: string | null;
}

export type NodePackageManager = 'bun' | 'npm' | 'pnpm' | 'yarn';

export interface PartialPnpmConfig {
	plugin?: string | null;
	version?: string | null;
}

export type NodeVersionManager = 'nodenv' | 'nvm';

export interface PartialYarnConfig {
	plugin?: string | null;
	plugins?: string[] | null;
	version?: string | null;
}

/** Docs: https://moonrepo.dev/docs/config/toolchain#node */
export interface PartialNodeConfig {
	/** @default true */
	addEnginesConstraint?: boolean | null;
	aliasPackageNames?: NodeProjectAliasFormat | null;
	binExecArgs?: string[] | null;
	bun?: PartialBunpmConfig | null;
	/** @default true */
	dedupeOnLockfileChange?: boolean | null;
	dependencyVersionFormat?: NodeVersionFormat | null;
	inferTasksFromScripts?: boolean | null;
	npm?: PartialNpmConfig | null;
	packageManager?: NodePackageManager | null;
	plugin?: string | null;
	pnpm?: PartialPnpmConfig | null;
	/** @default true */
	syncProjectWorkspaceDependencies?: boolean | null;
	syncVersionManagerConfig?: NodeVersionManager | null;
	version?: string | null;
	yarn?: PartialYarnConfig | null;
}

/** Docs: https://moonrepo.dev/docs/config/toolchain#rust */
export interface PartialRustConfig {
	bins?: PartialBinEntry[] | null;
	components?: string[] | null;
	plugin?: string | null;
	syncToolchainConfig?: boolean | null;
	targets?: string[] | null;
	version?: string | null;
}

/** Docs: https://moonrepo.dev/docs/config/toolchain#typescript */
export interface PartialTypeScriptConfig {
	/** @default true */
	createMissingConfig?: boolean | null;
	includeProjectReferenceSources?: boolean | null;
	includeSharedTypes?: boolean | null;
	/** @default 'tsconfig.json' */
	projectConfigFileName?: string | null;
	/** @default '.' */
	root?: string | null;
	/** @default 'tsconfig.json' */
	rootConfigFileName?: string | null;
	/** @default 'tsconfig.options.json' */
	rootOptionsConfigFileName?: string | null;
	routeOutDirToCache?: boolean | null;
	/** @default true */
	syncProjectReferences?: boolean | null;
	syncProjectReferencesToPaths?: boolean | null;
}

/** Docs: https://moonrepo.dev/docs/config/toolchain */
export interface PartialToolchainConfig {
	/** @default 'https://moonrepo.dev/schemas/toolchain.json' */
	$schema?: string | null;
	bun?: PartialBunConfig | null;
	deno?: PartialDenoConfig | null;
	extends?: string | null;
	node?: PartialNodeConfig | null;
	rust?: PartialRustConfig | null;
	typescript?: PartialTypeScriptConfig | null;
}

/** Docs: https://moonrepo.dev/docs/config/toolchain#bun */
export interface BunConfig {
	plugin: string | null;
	version: string | null;
}

export interface BinConfig {
	bin: string;
	force: boolean;
	local: boolean;
	name: string | null;
}

export type BinEntry = string | BinConfig;

/** Docs: https://moonrepo.dev/docs/config/toolchain#deno */
export interface DenoConfig {
	bins: BinEntry[];
	/** @default 'deps.ts' */
	depsFile: string;
	lockfile: boolean;
	plugin: string | null;
}

export interface BunpmConfig {
	plugin: string | null;
	version: string | null;
}

export interface NpmConfig {
	plugin: string | null;
	version: string | null;
}

export interface PnpmConfig {
	plugin: string | null;
	version: string | null;
}

export interface YarnConfig {
	plugin: string | null;
	plugins: string[];
	version: string | null;
}

/** Docs: https://moonrepo.dev/docs/config/toolchain#node */
export interface NodeConfig {
	/** @default true */
	addEnginesConstraint: boolean;
	aliasPackageNames: NodeProjectAliasFormat;
	binExecArgs: string[];
	bun: BunpmConfig | null;
	/** @default true */
	dedupeOnLockfileChange: boolean;
	dependencyVersionFormat: NodeVersionFormat;
	inferTasksFromScripts: boolean;
	npm: NpmConfig;
	packageManager: NodePackageManager;
	plugin: string | null;
	pnpm: PnpmConfig | null;
	/** @default true */
	syncProjectWorkspaceDependencies: boolean;
	syncVersionManagerConfig: NodeVersionManager | null;
	version: string | null;
	yarn: YarnConfig | null;
}

/** Docs: https://moonrepo.dev/docs/config/toolchain#rust */
export interface RustConfig {
	bins: BinEntry[];
	components: string[];
	plugin: string | null;
	syncToolchainConfig: boolean;
	targets: string[];
	version: string | null;
}

/** Docs: https://moonrepo.dev/docs/config/toolchain#typescript */
export interface TypeScriptConfig {
	/** @default true */
	createMissingConfig: boolean;
	includeProjectReferenceSources: boolean;
	includeSharedTypes: boolean;
	/** @default 'tsconfig.json' */
	projectConfigFileName: string;
	/** @default '.' */
	root: string;
	/** @default 'tsconfig.json' */
	rootConfigFileName: string;
	/** @default 'tsconfig.options.json' */
	rootOptionsConfigFileName: string;
	routeOutDirToCache: boolean;
	/** @default true */
	syncProjectReferences: boolean;
	syncProjectReferencesToPaths: boolean;
}

/** Docs: https://moonrepo.dev/docs/config/toolchain */
export interface ToolchainConfig {
	/** @default 'https://moonrepo.dev/schemas/toolchain.json' */
	$schema: string;
	bun: BunConfig | null;
	deno: DenoConfig | null;
	extends: string | null;
	node: NodeConfig | null;
	rust: RustConfig | null;
	typescript: TypeScriptConfig | null;
}
