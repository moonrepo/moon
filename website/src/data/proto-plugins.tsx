export interface ProtoTool {
	bins?: string[];
	detectionSources?: {
		file: string;
		label?: string;
		url?: string;
	}[];
	description: string;
	globalsDirs?: string[];
	homepageUrl?: string;
	name: string;
	noIcon?: boolean;
	pluginLocator?: string;
	pluginType: 'toml' | 'wasm';
	repoUrl: string;
	usageId?: string;
}

export const BUILTIN_TOOLS: Record<string, ProtoTool> = {
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
		bins: ['node', 'npx'],
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
		bins: ['npm', 'pnpm', 'yarn', 'node-gyp'],
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
		name: 'Python (experimental)',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/python-plugin',
	},
	rust: {
		description: `Rust is a blazingly fast and memory-efficient systems language.`,
		detectionSources: [{ file: 'rust-toolchain.toml' }, { file: 'rust-toolchain' }],
		globalsDirs: ['~/.cargo/bin'],
		name: 'Rust',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/rust-plugin',
	},
};

export const THIRD_PARTY_TOOLS: Record<string, ProtoTool> = {
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
};
