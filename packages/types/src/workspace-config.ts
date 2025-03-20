// Automatically generated by schematic. DO NOT MODIFY!

/* eslint-disable */

import type { ExtendsFrom } from './common';
import type { PluginLocator } from './toolchain-config';

/** How to order ownership rules within the generated file. */
export type CodeownersOrderBy = 'file-source' | 'project-name';

/** Configures code ownership rules for generating a `CODEOWNERS` file. */
export interface CodeownersConfig {
	/**
	 * Paths that are applied globally to all projects. Can be relative
	 * from the workspace root, or a wildcard match for any depth.
	 */
	globalPaths: Record<string, string[]>;
	/**
	 * How to order ownership rules within the generated file.
	 *
	 * @default 'file-source'
	 * @type {'file-source' | 'project-name'}
	 */
	orderBy: CodeownersOrderBy;
	/**
	 * Bitbucket and GitLab only. The number of approvals required for the
	 * request to be satisfied. This will be applied to all paths.
	 */
	requiredApprovals: number | null;
	/**
	 * Generates a `CODEOWNERS` file after aggregating all ownership
	 * rules from each project in the workspace.
	 */
	syncOnRun: boolean;
}

/** Configures boundaries and constraints between projects. */
export interface ConstraintsConfig {
	/**
	 * Enforces relationships between projects based on each project's
	 * `type` setting.
	 *
	 * @default true
	 */
	enforceProjectTypeRelationships?: boolean;
	/**
	 * Enforces relationships between projects based on each project's
	 * `tags` setting. Requires a mapping of tags, to acceptable tags.
	 */
	tagRelationships: Record<string, string[]>;
}

/** Configures aspects of the Docker pruning process. */
export interface DockerPruneConfig {
	/**
	 * Automatically delete vendor directories (package manager
	 * dependencies, build targets, etc) while pruning.
	 *
	 * @default true
	 */
	deleteVendorDirectories?: boolean;
	/**
	 * Automatically install production dependencies for all required
	 * toolchain's of the focused projects within the Docker build.
	 *
	 * @default true
	 */
	installToolchainDeps?: boolean;
}

/** Configures aspects of the Docker scaffolding process. */
export interface DockerScaffoldConfig {
	/**
	 * Copy toolchain specific configs/manifests/files into
	 * the workspace skeleton.
	 *
	 * @default true
	 */
	copyToolchainFiles?: boolean;
	/**
	 * List of glob patterns, relative from the workspace root,
	 * to include (or exclude) in the workspace skeleton.
	 */
	include: string[];
}

/** Configures our Docker integration. */
export interface DockerConfig {
	/** Configures aspects of the Docker pruning process. */
	prune: DockerPruneConfig;
	/** Configures aspects of the Docker scaffolding process. */
	scaffold: DockerScaffoldConfig;
}

/** Configures experiments across the entire moon workspace. */
export interface ExperimentsConfig {
	/**
	 * @default true
	 * @deprecated
	 */
	actionPipelineV2?: boolean;
	/**
	 * Disallow task relationships with different `runInCI` options.
	 *
	 * @default true
	 */
	disallowRunInCiMismatch?: boolean;
	/**
	 * @default true
	 * @deprecated
	 */
	interweavedTaskInheritance?: boolean;
	/**
	 * @default true
	 * @deprecated
	 */
	strictProjectAliases?: boolean;
	/**
	 * Disallow referencing the original ID of a renamed project when
	 * building the project graph.
	 */
	strictProjectIds: boolean;
	/**
	 * @default true
	 * @deprecated
	 */
	taskOutputBoundaries?: boolean;
}

/** Configures an individual extension. */
export interface ExtensionConfig {
	/** Arbitrary configuration that'll be passed to the WASM plugin. */
	config: Record<string, unknown>;
	/** Location of the WASM plugin to use. */
	plugin: PluginLocator | null;
}

/** Configures the generator for scaffolding from templates. */
export interface GeneratorConfig {
	/**
	 * The list of file paths, relative from the workspace root,
	 * in which to locate templates.
	 */
	templates?: string[];
}

/** The optimization to use when hashing. */
export type HasherOptimization = 'accuracy' | 'performance';

/** The strategy to use when walking the file system. */
export type HasherWalkStrategy = 'glob' | 'vcs';

/** Configures aspects of the content hashing engine. */
export interface HasherConfig {
	/**
	 * The number of files to include in each hash operation.
	 *
	 * @default 2500
	 * @deprecated
	 */
	batchSize?: number;
	/**
	 * When `warnOnMissingInputs` is enabled, filters missing file
	 * paths from logging a warning.
	 */
	ignoreMissingPatterns: string[];
	/**
	 * Filters file paths that match a configured glob pattern
	 * when a hash is being generated. Patterns are workspace relative,
	 * so prefixing with `**` is recommended.
	 */
	ignorePatterns: string[];
	/**
	 * The optimization to use when hashing.
	 *
	 * @default 'accuracy'
	 * @type {'accuracy' | 'performance'}
	 */
	optimization: HasherOptimization;
	/**
	 * The strategy to use when walking the file system.
	 *
	 * @default 'vcs'
	 * @type {'glob' | 'vcs'}
	 */
	walkStrategy: HasherWalkStrategy;
	/**
	 * Logs a warning when a task has configured an explicit file path
	 * input, and that file does not exist when hashing.
	 *
	 * @default true
	 */
	warnOnMissingInputs?: boolean;
}

/** Configures how and where notifications are sent. */
export interface NotifierConfig {
	/** A secure URL in which to send webhooks to. */
	webhookUrl: string | null;
}

/** Configures projects in the workspace, using both globs and explicit source paths. */
export interface WorkspaceProjectsConfig {
	/**
	 * A list of globs in which to locate project directories.
	 * Can be suffixed with `moon.yml` or `moon.pkl` to only find distinct projects.
	 */
	globs: string[];
	/** A mapping of project IDs to relative file paths to each project directory. */
	sources: Record<string, string>;
}

export type WorkspaceProjects = WorkspaceProjectsConfig | string[] | Record<string, string>;

/** Configures aspects of the task runner (also known as the action pipeline). */
export interface RunnerConfig {
	/**
	 * List of target's for tasks without outputs, that should be
	 * cached and persisted.
	 */
	archivableTargets: string[];
	/**
	 * Automatically clean the cache after every task run.
	 *
	 * @default true
	 */
	autoCleanCache?: boolean;
	/**
	 * The lifetime in which task outputs will be cached.
	 *
	 * @default '7 days'
	 */
	cacheLifetime?: string;
	/**
	 * Automatically inherit color settings for all tasks being ran.
	 *
	 * @default true
	 */
	inheritColorsForPipedTasks?: boolean;
	/**
	 * Threshold in milliseconds in which to force kill running child
	 * processes after the pipeline receives an external signal. A value
	 * of 0 will not kill the process and let them run to completion.
	 *
	 * @default 2000
	 */
	killProcessThreshold?: number;
	/** Logs the task's command and arguments when running the task. */
	logRunningCommand: boolean;
}

/** The API format of the remote service. */
export type RemoteApi = 'grpc' | 'http';

/** Configures basic HTTP authentication. */
export interface RemoteAuthConfig {
	/** HTTP headers to inject into every request. */
	headers: Record<string, string>;
	/** The name of an environment variable to use as a bearer token. */
	token: string | null;
}

/** Supported blob compression levels for gRPC APIs. */
export type RemoteCompression = 'none' | 'zstd';

/** Configures the action cache (AC) and content addressable cache (CAS). */
export interface RemoteCacheConfig {
	/**
	 * The compression format to use when uploading/downloading blobs.
	 *
	 * @default 'none'
	 * @type {'none' | 'zstd'}
	 */
	compression: RemoteCompression;
	/**
	 * Unique instance name for blobs. Will be used as a folder name.
	 *
	 * @default 'moon-outputs'
	 */
	instanceName?: string;
}

/** Configures for both server and client authentication with mTLS. */
export interface RemoteMtlsConfig {
	/**
	 * If true, assume that the server supports HTTP/2,
	 * even if it doesn't provide protocol negotiation via ALPN.
	 */
	assumeHttp2: boolean;
	/**
	 * A file path, relative from the workspace root, to the
	 * certificate authority PEM encoded X509 certificate.
	 */
	caCert: string;
	/**
	 * A file path, relative from the workspace root, to the
	 * client's PEM encoded X509 certificate.
	 */
	clientCert: string;
	/**
	 * A file path, relative from the workspace root, to the
	 * client's PEM encoded X509 private key.
	 */
	clientKey: string;
	/** The domain name in which to verify the TLS certificate. */
	domain: string | null;
}

/** Configures for server-only authentication with TLS. */
export interface RemoteTlsConfig {
	/**
	 * If true, assume that the server supports HTTP/2,
	 * even if it doesn't provide protocol negotiation via ALPN.
	 */
	assumeHttp2: boolean;
	/**
	 * A file path, relative from the workspace root, to the
	 * certificate authority PEM encoded X509 certificate.
	 */
	cert: string;
	/** The domain name in which to verify the TLS certificate. */
	domain: string | null;
}

/** Configures the remote service, powered by the Bazel Remote Execution API. */
export interface RemoteConfig {
	/**
	 * The API format of the remote service.
	 *
	 * @default 'grpc'
	 * @type {'grpc' | 'http'}
	 */
	api: RemoteApi;
	/** Connect to the host using basic HTTP authentication. */
	auth: RemoteAuthConfig | null;
	/** Configures the action cache (AC) and content addressable cache (CAS). */
	cache: RemoteCacheConfig;
	/**
	 * The remote host to connect and send requests to.
	 * Supports gRPC protocols.
	 */
	host: string;
	/**
	 * Connect to the host using server and client authentication with mTLS.
	 * This takes precedence over normal TLS.
	 */
	mtls: RemoteMtlsConfig | null;
	/** Connect to the host using server-only authentication with TLS. */
	tls: RemoteTlsConfig | null;
}

/** The format to use for generated VCS hook files. */
export type VcsHookFormat = 'bash' | 'native';

/** The VCS being utilized by the repository. */
export type VcsManager = 'git';

/**
 * The upstream version control provider, where the repository
 * source code is stored.
 */
export type VcsProvider = 'bitbucket' | 'github' | 'gitlab' | 'other';

/** Configures the version control system (VCS). */
export interface VcsConfig {
	/**
	 * The default branch / base.
	 *
	 * @default 'master'
	 */
	defaultBranch?: string;
	/**
	 * The format to use for generated VCS hook files.
	 *
	 * @default 'native'
	 * @type {'bash' | 'native'}
	 */
	hookFormat: VcsHookFormat;
	/** A mapping of hooks to commands to run when the hook is triggered. */
	hooks: Record<string, string[]>;
	/**
	 * The VCS client being utilized by the repository.
	 *
	 * @default 'git'
	 * @type {'git'}
	 */
	manager: VcsManager;
	/**
	 * The upstream version control provider, where the repository
	 * source code is stored.
	 *
	 * @default 'github'
	 * @type {'bitbucket' | 'github' | 'gitlab' | 'other'}
	 */
	provider: VcsProvider;
	/** List of remote's in which to compare branches against. */
	remoteCandidates?: string[];
	/** Generates hooks and scripts based on the `hooks` setting. */
	syncHooks: boolean;
}

/**
 * Configures all aspects of the moon workspace.
 * Docs: https://moonrepo.dev/docs/config/workspace
 */
export interface WorkspaceConfig {
	/** @default 'https://moonrepo.dev/schemas/workspace.json' */
	$schema?: string;
	/** Configures code ownership rules for generating a `CODEOWNERS` file. */
	codeowners: CodeownersConfig;
	/** Configures boundaries and constraints between projects. */
	constraints: ConstraintsConfig;
	/** Configures Docker integration for the workspace. */
	docker: DockerConfig;
	/** Configures experiments across the entire moon workspace. */
	experiments: ExperimentsConfig;
	/**
	 * Extends one or many workspace configuration file. Supports a relative
	 * file path or a secure URL.
	 */
	extends: ExtendsFrom | null;
	/** Configures extensions that can be executed with `moon ext`. */
	extensions: Record<string, ExtensionConfig>;
	/** Configures the generator for scaffolding from templates. */
	generator: GeneratorConfig;
	/** Configures aspects of the content hashing engine. */
	hasher: HasherConfig;
	/** Configures how and where notifications are sent. */
	notifier: NotifierConfig;
	/**
	 * Configures all projects within the workspace to create a project graph.
	 * Accepts a list of globs, a mapping of projects to relative file paths,
	 * or both values.
	 */
	projects: WorkspaceProjects;
	/** Configures aspects of the task runner (also known as the action pipeline). */
	runner: RunnerConfig;
	/**
	 * Collects anonymous usage information, and checks for new moon versions.
	 *
	 * @default true
	 */
	telemetry?: boolean;
	/** Configures aspects of the remote service. */
	unstable_remote: RemoteConfig | null;
	/** Configures the version control system (VCS). */
	vcs: VcsConfig;
	/** Requires a specific version of the `moon` binary. */
	versionConstraint: string | null;
}

/** Configures code ownership rules for generating a `CODEOWNERS` file. */
export interface PartialCodeownersConfig {
	/**
	 * Paths that are applied globally to all projects. Can be relative
	 * from the workspace root, or a wildcard match for any depth.
	 */
	globalPaths?: Record<string, string[]> | null;
	/**
	 * How to order ownership rules within the generated file.
	 *
	 * @default 'file-source'
	 */
	orderBy?: CodeownersOrderBy | null;
	/**
	 * Bitbucket and GitLab only. The number of approvals required for the
	 * request to be satisfied. This will be applied to all paths.
	 */
	requiredApprovals?: number | null;
	/**
	 * Generates a `CODEOWNERS` file after aggregating all ownership
	 * rules from each project in the workspace.
	 */
	syncOnRun?: boolean | null;
}

/** Configures boundaries and constraints between projects. */
export interface PartialConstraintsConfig {
	/**
	 * Enforces relationships between projects based on each project's
	 * `type` setting.
	 *
	 * @default true
	 */
	enforceProjectTypeRelationships?: boolean | null;
	/**
	 * Enforces relationships between projects based on each project's
	 * `tags` setting. Requires a mapping of tags, to acceptable tags.
	 */
	tagRelationships?: Record<string, string[]> | null;
}

/** Configures aspects of the Docker pruning process. */
export interface PartialDockerPruneConfig {
	/**
	 * Automatically delete vendor directories (package manager
	 * dependencies, build targets, etc) while pruning.
	 *
	 * @default true
	 */
	deleteVendorDirectories?: boolean | null;
	/**
	 * Automatically install production dependencies for all required
	 * toolchain's of the focused projects within the Docker build.
	 *
	 * @default true
	 */
	installToolchainDeps?: boolean | null;
}

/** Configures aspects of the Docker scaffolding process. */
export interface PartialDockerScaffoldConfig {
	/**
	 * Copy toolchain specific configs/manifests/files into
	 * the workspace skeleton.
	 *
	 * @default true
	 */
	copyToolchainFiles?: boolean | null;
	/**
	 * List of glob patterns, relative from the workspace root,
	 * to include (or exclude) in the workspace skeleton.
	 */
	include?: string[] | null;
}

/** Configures our Docker integration. */
export interface PartialDockerConfig {
	/** Configures aspects of the Docker pruning process. */
	prune?: PartialDockerPruneConfig | null;
	/** Configures aspects of the Docker scaffolding process. */
	scaffold?: PartialDockerScaffoldConfig | null;
}

/** Configures experiments across the entire moon workspace. */
export interface PartialExperimentsConfig {
	/**
	 * @default true
	 * @deprecated
	 */
	actionPipelineV2?: boolean | null;
	/**
	 * Disallow task relationships with different `runInCI` options.
	 *
	 * @default true
	 */
	disallowRunInCiMismatch?: boolean | null;
	/**
	 * @default true
	 * @deprecated
	 */
	interweavedTaskInheritance?: boolean | null;
	/**
	 * @default true
	 * @deprecated
	 */
	strictProjectAliases?: boolean | null;
	/**
	 * Disallow referencing the original ID of a renamed project when
	 * building the project graph.
	 */
	strictProjectIds?: boolean | null;
	/**
	 * @default true
	 * @deprecated
	 */
	taskOutputBoundaries?: boolean | null;
}

/** Configures an individual extension. */
export interface PartialExtensionConfig {
	/** Arbitrary configuration that'll be passed to the WASM plugin. */
	config?: Record<string, unknown> | null;
	/** Location of the WASM plugin to use. */
	plugin?: PluginLocator | null;
}

/** Configures the generator for scaffolding from templates. */
export interface PartialGeneratorConfig {
	/**
	 * The list of file paths, relative from the workspace root,
	 * in which to locate templates.
	 */
	templates?: string[] | null;
}

/** Configures aspects of the content hashing engine. */
export interface PartialHasherConfig {
	/**
	 * The number of files to include in each hash operation.
	 *
	 * @default 2500
	 * @deprecated
	 */
	batchSize?: number | null;
	/**
	 * When `warnOnMissingInputs` is enabled, filters missing file
	 * paths from logging a warning.
	 */
	ignoreMissingPatterns?: string[] | null;
	/**
	 * Filters file paths that match a configured glob pattern
	 * when a hash is being generated. Patterns are workspace relative,
	 * so prefixing with `**` is recommended.
	 */
	ignorePatterns?: string[] | null;
	/**
	 * The optimization to use when hashing.
	 *
	 * @default 'accuracy'
	 */
	optimization?: HasherOptimization | null;
	/**
	 * The strategy to use when walking the file system.
	 *
	 * @default 'vcs'
	 */
	walkStrategy?: HasherWalkStrategy | null;
	/**
	 * Logs a warning when a task has configured an explicit file path
	 * input, and that file does not exist when hashing.
	 *
	 * @default true
	 */
	warnOnMissingInputs?: boolean | null;
}

/** Configures how and where notifications are sent. */
export interface PartialNotifierConfig {
	/** A secure URL in which to send webhooks to. */
	webhookUrl?: string | null;
}

/** Configures projects in the workspace, using both globs and explicit source paths. */
export interface PartialWorkspaceProjectsConfig {
	/**
	 * A list of globs in which to locate project directories.
	 * Can be suffixed with `moon.yml` or `moon.pkl` to only find distinct projects.
	 */
	globs?: string[] | null;
	/** A mapping of project IDs to relative file paths to each project directory. */
	sources?: Record<string, string> | null;
}

export type PartialWorkspaceProjects =
	| PartialWorkspaceProjectsConfig
	| string[]
	| Record<string, string>;

/** Configures aspects of the task runner (also known as the action pipeline). */
export interface PartialRunnerConfig {
	/**
	 * List of target's for tasks without outputs, that should be
	 * cached and persisted.
	 */
	archivableTargets?: string[] | null;
	/**
	 * Automatically clean the cache after every task run.
	 *
	 * @default true
	 */
	autoCleanCache?: boolean | null;
	/**
	 * The lifetime in which task outputs will be cached.
	 *
	 * @default '7 days'
	 */
	cacheLifetime?: string | null;
	/**
	 * Automatically inherit color settings for all tasks being ran.
	 *
	 * @default true
	 */
	inheritColorsForPipedTasks?: boolean | null;
	/**
	 * Threshold in milliseconds in which to force kill running child
	 * processes after the pipeline receives an external signal. A value
	 * of 0 will not kill the process and let them run to completion.
	 *
	 * @default 2000
	 */
	killProcessThreshold?: number | null;
	/** Logs the task's command and arguments when running the task. */
	logRunningCommand?: boolean | null;
}

/** Configures basic HTTP authentication. */
export interface PartialRemoteAuthConfig {
	/** HTTP headers to inject into every request. */
	headers?: Record<string, string> | null;
	/** The name of an environment variable to use as a bearer token. */
	token?: string | null;
}

/** Configures the action cache (AC) and content addressable cache (CAS). */
export interface PartialRemoteCacheConfig {
	/**
	 * The compression format to use when uploading/downloading blobs.
	 *
	 * @default 'none'
	 */
	compression?: RemoteCompression | null;
	/**
	 * Unique instance name for blobs. Will be used as a folder name.
	 *
	 * @default 'moon-outputs'
	 */
	instanceName?: string | null;
}

/** Configures for both server and client authentication with mTLS. */
export interface PartialRemoteMtlsConfig {
	/**
	 * If true, assume that the server supports HTTP/2,
	 * even if it doesn't provide protocol negotiation via ALPN.
	 */
	assumeHttp2?: boolean | null;
	/**
	 * A file path, relative from the workspace root, to the
	 * certificate authority PEM encoded X509 certificate.
	 */
	caCert?: string | null;
	/**
	 * A file path, relative from the workspace root, to the
	 * client's PEM encoded X509 certificate.
	 */
	clientCert?: string | null;
	/**
	 * A file path, relative from the workspace root, to the
	 * client's PEM encoded X509 private key.
	 */
	clientKey?: string | null;
	/** The domain name in which to verify the TLS certificate. */
	domain?: string | null;
}

/** Configures for server-only authentication with TLS. */
export interface PartialRemoteTlsConfig {
	/**
	 * If true, assume that the server supports HTTP/2,
	 * even if it doesn't provide protocol negotiation via ALPN.
	 */
	assumeHttp2?: boolean | null;
	/**
	 * A file path, relative from the workspace root, to the
	 * certificate authority PEM encoded X509 certificate.
	 */
	cert?: string | null;
	/** The domain name in which to verify the TLS certificate. */
	domain?: string | null;
}

/** Configures the remote service, powered by the Bazel Remote Execution API. */
export interface PartialRemoteConfig {
	/**
	 * The API format of the remote service.
	 *
	 * @default 'grpc'
	 */
	api?: RemoteApi | null;
	/** Connect to the host using basic HTTP authentication. */
	auth?: PartialRemoteAuthConfig | null;
	/** Configures the action cache (AC) and content addressable cache (CAS). */
	cache?: PartialRemoteCacheConfig | null;
	/**
	 * The remote host to connect and send requests to.
	 * Supports gRPC protocols.
	 */
	host?: string | null;
	/**
	 * Connect to the host using server and client authentication with mTLS.
	 * This takes precedence over normal TLS.
	 */
	mtls?: PartialRemoteMtlsConfig | null;
	/** Connect to the host using server-only authentication with TLS. */
	tls?: PartialRemoteTlsConfig | null;
}

/** Configures the version control system (VCS). */
export interface PartialVcsConfig {
	/**
	 * The default branch / base.
	 *
	 * @default 'master'
	 */
	defaultBranch?: string | null;
	/**
	 * The format to use for generated VCS hook files.
	 *
	 * @default 'native'
	 */
	hookFormat?: VcsHookFormat | null;
	/** A mapping of hooks to commands to run when the hook is triggered. */
	hooks?: Record<string, string[]> | null;
	/**
	 * The VCS client being utilized by the repository.
	 *
	 * @default 'git'
	 */
	manager?: VcsManager | null;
	/**
	 * The upstream version control provider, where the repository
	 * source code is stored.
	 *
	 * @default 'github'
	 */
	provider?: VcsProvider | null;
	/** List of remote's in which to compare branches against. */
	remoteCandidates?: string[] | null;
	/** Generates hooks and scripts based on the `hooks` setting. */
	syncHooks?: boolean | null;
}

/**
 * Configures all aspects of the moon workspace.
 * Docs: https://moonrepo.dev/docs/config/workspace
 */
export interface PartialWorkspaceConfig {
	/** @default 'https://moonrepo.dev/schemas/workspace.json' */
	$schema?: string | null;
	/** Configures code ownership rules for generating a `CODEOWNERS` file. */
	codeowners?: PartialCodeownersConfig | null;
	/** Configures boundaries and constraints between projects. */
	constraints?: PartialConstraintsConfig | null;
	/** Configures Docker integration for the workspace. */
	docker?: PartialDockerConfig | null;
	/** Configures experiments across the entire moon workspace. */
	experiments?: PartialExperimentsConfig | null;
	/**
	 * Extends one or many workspace configuration file. Supports a relative
	 * file path or a secure URL.
	 */
	extends?: ExtendsFrom | null;
	/** Configures extensions that can be executed with `moon ext`. */
	extensions?: Record<string, PartialExtensionConfig> | null;
	/** Configures the generator for scaffolding from templates. */
	generator?: PartialGeneratorConfig | null;
	/** Configures aspects of the content hashing engine. */
	hasher?: PartialHasherConfig | null;
	/** Configures how and where notifications are sent. */
	notifier?: PartialNotifierConfig | null;
	/**
	 * Configures all projects within the workspace to create a project graph.
	 * Accepts a list of globs, a mapping of projects to relative file paths,
	 * or both values.
	 */
	projects?: PartialWorkspaceProjects | null;
	/** Configures aspects of the task runner (also known as the action pipeline). */
	runner?: PartialRunnerConfig | null;
	/**
	 * Collects anonymous usage information, and checks for new moon versions.
	 *
	 * @default true
	 */
	telemetry?: boolean | null;
	/** Configures aspects of the remote service. */
	unstable_remote?: PartialRemoteConfig | null;
	/** Configures the version control system (VCS). */
	vcs?: PartialVcsConfig | null;
	/** Requires a specific version of the `moon` binary. */
	versionConstraint?: string | null;
}
