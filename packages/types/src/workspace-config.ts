export interface GeneratorConfig {
	templates: string[];
}

export interface HasherConfig {
	optimization: 'accuracy' | 'performance';
}

export interface NotifierConfig {
	webhookUrl: string | null;
}

export interface RunnerConfig {
	archivableTargets: string[];
	cacheLifetime: string;
	implicitDeps: string[];
	implicitInputs: string[];
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
	generator: GeneratorConfig;
	hasher: HasherConfig;
	notifier: NotifierConfig;
	projects:
		| Record<string, string>
		| string[]
		| { globs: string[]; sources: Record<string, string> };
	runner: RunnerConfig;
	vcs: VcsConfig;
	versionConstraint: string | null;
}
