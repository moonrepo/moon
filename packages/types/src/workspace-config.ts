export interface ConstraintsConfig {
	enforceProjectTypeRelationships: boolean;
	tagRelationships: Record<string, string[]>;
}

export interface GeneratorConfig {
	templates: string[];
}

export interface HasherConfig {
	optimization: 'accuracy' | 'performance';
	walkStrategy: 'glob' | 'vcs';
	warnOnMissingInputs: boolean;
}

export interface NotifierConfig {
	webhookUrl: string | null;
}

export interface RunnerConfig {
	archivableTargets: string[];
	cacheLifetime: string;
	inheritColorsForPipedTasks: boolean;
	logRunningCommand: boolean;
}

export interface VcsConfig {
	defaultBranch: string;
	manager: 'git' | 'svn';
	remoteCandidates: string[];
}

export interface WorkspaceConfig {
	extends: string | null;
	constraints: ConstraintsConfig;
	generator: GeneratorConfig;
	hasher: HasherConfig;
	notifier: NotifierConfig;
	projects:
		| Record<string, string>
		| string[]
		| { globs: string[]; sources: Record<string, string> };
	runner: RunnerConfig;
	telemetry: boolean;
	vcs: VcsConfig;
	versionConstraint: string | null;
}
