/**
 * INSTRUCTIONS:
 *
 * 1. Add an entry to the `THIRD_PARTY_TOOLS` constant below.
 *    The object key is the preferred tool identifier,
 *    while the value is a `ProtoTool` object.
 *
 * 2. For third-party tools, ensure that the `pluginLocator`
 *    field is set, so users know how to install the plugin.
 *
 * 3. If applicable, visit https://devicon.dev and grab the SVG
 *    content for your tool. If you have a custom SVG, use that.
 *    Copy the SVG to `website/static/img/tools`. Ensure the
 *    following changes are made:
 *      - Remove all unnecessary metadata (maybe use svgo).
 *      - All colors should be set to `currentColor`.
 *      - View box width/height should be 128 (if a square).
 */

export interface ProtoTool {
	name: string;
	description: string;
	homepageUrl?: string;
	repoUrl: string;
	noIcon?: boolean; // If no SVG

	// Plugin information:
	// https://moonrepo.dev/docs/proto/plugins#enabling-plugins
	pluginLocator?: string;
	pluginType: 'toml' | 'wasm';
	usageId?: string;

	// Availble global binaries/directories:
	// https://moonrepo.dev/docs/proto/wasm-plugin#locating-binaries
	bins?: string[];
	globalsDirs?: string[];

	// Version detection sources:
	// https://moonrepo.dev/docs/proto/wasm-plugin#detecting-versions
	detectionSources?: {
		file: string;
		label?: string;
		url?: string;
	}[];
}

export const BUILT_IN_TOOLS: Record<string, ProtoTool> = {
	bun: {
		bins: ['bun', 'bunx'],
		description:
			'Bun is an all-in-one runtime and toolset for JavaScript and TypeScript, powered by Zig and Webkit.',
		globalsDirs: ['~/.bun/bin'],
		homepageUrl: 'https://bun.sh',
		name: 'Bun',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/bun-plugin',
	},
	deno: {
		bins: ['deno'],
		description:
			"Deno is a secure runtime for JavaScript and TypeScript, powered by Rust and Chrome's V8 engine.",
		detectionSources: [{ file: '.dvmrc', url: 'https://github.com/justjavac/dvm' }],
		globalsDirs: ['$DENO_INSTALL_ROOT/bin', '$DENO_HOME/bin', '~/.deno/bin'],
		homepageUrl: 'https://deno.land',
		name: 'Deno',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/deno-plugin',
	},
	go: {
		bins: ['go'],
		description: 'Go is a simple, secure, and fast systems language.',
		detectionSources: [
			{ file: 'go.work', url: 'https://go.dev/doc/tutorial/workspaces' },
			{ file: 'go.mod', url: 'https://go.dev/doc/modules/gomod-ref' },
		],
		globalsDirs: ['$GOBIN', '$GOROOT/bin', '$GOPATH/bin', '~/go/bin'],
		homepageUrl: 'https://go.dev',
		name: 'Go',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/go-plugin',
	},
	node: {
		bins: ['node'],
		description: "Node.js is a JavaScript runtime built on Chrome's V8 engine.",
		detectionSources: [
			{ file: '.nvmrc', url: 'https://github.com/nvm-sh/nvm' },
			{ file: '.node-version', url: 'https://github.com/nodenv/nodenv' },
			{ file: 'package.json', label: 'engines' },
		],
		globalsDirs: ['~/.proto/tools/node/globals/bin'],
		homepageUrl: 'https://nodejs.org',
		name: 'Node.js',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/node-plugin',
	},
	node_depman: {
		bins: ['npm', 'npx', 'pnpm', 'pnpx', 'yarn', 'node-gyp'],
		description: 'proto supports all popular Node.js package managers.',
		detectionSources: [{ file: 'package.json', label: 'engines / package manager' }],
		globalsDirs: ['~/.proto/tools/node/globals/bin'],
		name: 'npm, pnpm, yarn',
		noIcon: true,
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/node-plugin',
		usageId: '<manager>',
	},
	python: {
		bins: ['python', 'pip'],
		description: 'Python is a high-level, general-purpose programming language.',
		detectionSources: [{ file: '.python-version', url: 'https://github.com/pyenv/pyenv' }],
		globalsDirs: ['~/.local/bin'],
		homepageUrl: 'https://www.python.org/',
		name: 'Python (experimental)',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/python-plugin',
	},
	rust: {
		description: `Rust is a blazingly fast and memory-efficient systems language.`,
		detectionSources: [{ file: 'rust-toolchain.toml' }, { file: 'rust-toolchain' }],
		globalsDirs: ['~/.cargo/bin'],
		homepageUrl: 'https://www.rust-lang.org/',
		name: 'Rust',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/rust-plugin',
	},
};

export const THIRD_PARTY_TOOLS: Record<string, ProtoTool> = {
	buf: {
		bins: ['buf'],
		description: 'A new way of working with Protocol Buffers.',
		homepageUrl: 'https://buf.build',
		name: 'buf',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/buf.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/buf.toml',
	},
	gojq: {
		bins: ['gojq'],
		description: 'Pure Go implementation of jq.',
		homepageUrl: 'https://github.com/itchyny/gojq',
		name: 'gojq',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/gojq.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/gojq.toml',
	},
	helm: {
		bins: ['helm'],
		description: 'The Kubernetes Package Manager',
		homepageUrl: 'https://helm.sh',
		name: 'helm',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/helm.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/helm.toml',
	},
	helmfile: {
		bins: ['helmfile'],
		description: 'Deploy Kubernetes Helm Charts',
		homepageUrl: 'https://helmfile.readthedocs.io/en/latest',
		name: 'helmfile',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/helmfile.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/helmfile.toml',
	},
	kubectl: {
		bins: ['kubectl'],
		description: 'Kubernetes command line tool.',
		homepageUrl: 'https://kubernetes.io',
		name: 'kubectl',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/kubectl.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/kubectl.toml',
	},
	moon: {
		bins: ['moon'],
		description: 'moon is a multi-language build system and codebase management tool.',
		homepageUrl: 'https://moonrepo.dev/moon',
		name: 'moon',
		pluginLocator:
			'source:https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/moonrepo/moon/blob/master/proto-plugin.toml',
	},
	sops: {
		bins: ['sops'],
		description: 'Simple and flexible tool for managing secrets.',
		homepageUrl: 'https://github.com/getsops/sops',
		name: 'sops',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/sops.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/sops.toml',
	},
	terragrunt: {
		bins: ['terragrunt'],
		description:
			'Thin wrapper that provides extra tools for keeping your terraform configurations DRY.',
		homepageUrl: 'https://terragrunt.gruntwork.io',
		name: 'terragrunt',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/terragrunt.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/terragrunt.toml',
	},
	zig: {
		bins: ['zig'],
		description: 'Zig is a general-purpose programming language and toolchain.',
		homepageUrl: 'https://ziglang.org',
		name: 'zig',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/zig.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/zig.toml',
	},
};
