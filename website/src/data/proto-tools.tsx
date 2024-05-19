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
	author: string;

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
		author: 'moonrepo',
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
		author: 'moonrepo',
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
		author: 'moonrepo',
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
		author: 'moonrepo',
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
		author: 'moonrepo',
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
		author: 'moonrepo',
		bins: ['python', 'pip'],
		description: 'Python is a high-level, general-purpose programming language.',
		detectionSources: [{ file: '.python-version', url: 'https://github.com/pyenv/pyenv' }],
		globalsDirs: ['~/.proto/tools/python/x.x.x/install/bin'],
		homepageUrl: 'https://www.python.org/',
		name: 'Python (experimental)',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/python-plugin',
	},
	rust: {
		author: 'moonrepo',
		description: `Rust is a blazingly fast and memory-efficient systems language.`,
		detectionSources: [{ file: 'rust-toolchain.toml' }, { file: 'rust-toolchain' }],
		globalsDirs: ['~/.cargo/bin'],
		homepageUrl: 'https://www.rust-lang.org/',
		name: 'Rust',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/moonrepo/rust-plugin',
	},
};

export const THIRD_PARTY_TOOLS: Record<string, ProtoTool | ProtoTool[]> = {
	act: {
		author: 'theomessin',
		bins: ['act'],
		description: 'Run your GitHub Actions locally.',
		homepageUrl: 'https://github.com/nektos/act',
		name: 'act',
		pluginLocator:
			'source:https://raw.githubusercontent.com/theomessin/proto-toml-plugins/master/act.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/theomessin/proto-toml-plugins/blob/master/act.toml',
	},
	actionlint: {
		author: 'Phault',
		bins: ['actionlint'],
		description: 'Static checker for GitHub Actions workflow files',
		homepageUrl: 'https://github.com/rhysd/actionlint',
		name: 'actionlint',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/actionlint/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	bazel: {
		author: 'Phault',
		bins: ['bazel'],
		description: 'A fast, scalable, multi-language and extensible build system',
		homepageUrl: 'https://bazel.build',
		name: 'Bazel',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/bazel/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	biome: {
		author: 'Phault',
		bins: ['biome'],
		description:
			'A performant toolchain for web projects, aiming to provide developer tools to maintain the health of said projects',
		homepageUrl: 'https://biomejs.dev',
		name: 'Biome',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/biome/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	black: {
		author: 'appthrust',
		bins: ['black'],
		description: 'The uncompromising Python code formatter',
		homepageUrl: 'https://black.readthedocs.io/en/stable/',
		name: 'Black',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/appthrust/proto-toml-plugins/main/black/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/appthrust/proto-toml-plugins',
	},
	buf: {
		author: 'stk0vrfl0w',
		bins: ['buf'],
		description: 'A new way of working with Protocol Buffers.',
		homepageUrl: 'https://buf.build',
		name: 'buf',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/buf.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/buf.toml',
	},
	caddy: {
		author: 'Phault',
		bins: ['caddy'],
		description: 'Fast and extensible multi-platform HTTP/1-2-3 web server with automatic HTTPS',
		homepageUrl: 'https://caddyserver.com',
		name: 'Caddy',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/caddy/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	cmake: {
		author: 'Phault',
		bins: ['cmake'],
		description: 'CMake is a cross-platform, open-source build system generator',
		homepageUrl: 'https://cmake.org',
		name: 'CMake',
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/cmake/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	cosign: {
		author: 'Phault',
		bins: ['cosign'],
		description: 'Code signing and transparency for containers and binaries',
		homepageUrl: 'https://github.com/sigstore/cosign',
		name: 'Cosign',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/cosign/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	dagger: {
		author: 'Phault',
		bins: ['dagger'],
		description:
			'Powerful, programmable open source CI/CD engine that runs your pipelines in containers',
		homepageUrl: 'https://dagger.io',
		name: 'Dagger',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/dagger/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	dotnet: {
		author: 'Phault',
		bins: ['dotnet'],
		description:
			'.NET is the free, open-source, cross-platform framework for building modern apps and powerful cloud services.',
		homepageUrl: 'https://dotnet.microsoft.com',
		name: '.NET',
		pluginLocator: 'github:Phault/proto-dotnet-plugin',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/Phault/proto-dotnet-plugin',
	},
	doppler: {
		author: 'Maktouch',
		bins: ['doppler'],
		description: 'The official CLI for interacting with your Doppler secrets and configuration.',
		homepageUrl: 'https://www.doppler.com/',
		name: 'Doppler',
		pluginLocator:
			'source:https://raw.githubusercontent.com/maktouch/proto-toml-plugins/main/plugins/doppler.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/maktouch/proto-toml-plugins',
	},
	dprint: {
		author: 'Phault',
		bins: ['dprint'],
		description: 'A pluggable and configurable code formatting platform written in Rust.',
		homepageUrl: 'https://dprint.dev',
		name: 'dprint',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/dprint/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	earthly: {
		author: 'theomessin',
		bins: ['earthly'],
		description: 'Like Dockerfile and Makefile had a baby.',
		homepageUrl: 'https://earthly.dev',
		name: 'earthly',
		pluginLocator:
			'source:https://raw.githubusercontent.com/theomessin/proto-toml-plugins/master/earthly.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/theomessin/proto-toml-plugins/blob/master/earthly.toml',
	},
	flyctl: {
		author: 'Phault',
		bins: ['fly'],
		description: 'A command-line interface for fly.io',
		homepageUrl: 'https://github.com/superfly/flyctl',
		name: 'flyctl',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/flyctl/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	gh: {
		author: 'Maktouch',
		bins: ['gh'],
		description: 'Take GitHub to the command line',
		homepageUrl: 'https://cli.github.com/',
		name: 'Github CLI',
		pluginLocator:
			'source:https://raw.githubusercontent.com/maktouch/proto-toml-plugins/main/plugins/gh.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/maktouch/proto-toml-plugins',
	},
	gitleaks: {
		author: 'Phault',
		bins: ['gitleaks'],
		description:
			'A fast, light-weight, portable, and open-source secret scanner for git repositories, files, and directories',
		homepageUrl: 'https://gitleaks.io',
		name: 'Gitleaks',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/gitleaks/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	gleam: {
		author: 'vancegillies',
		bins: ['gleam'],
		description: 'A statically typed language for the Erlang VM and JavaScript',
		homepageUrl: 'https://gleam.run/',
		name: 'gleam',
		pluginLocator:
			'source:https://raw.githubusercontent.com/vancegillies/proto-gleam-plugin/main/gleam.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/vancegillies/proto-gleam-plugin/blob/main/gleam.toml',
	},
	gojq: {
		author: 'stk0vrfl0w',
		bins: ['gojq'],
		description: 'Pure Go implementation of jq.',
		homepageUrl: 'https://github.com/itchyny/gojq',
		name: 'gojq',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/gojq.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/gojq.toml',
	},
	gum: {
		author: 'Phault',
		bins: ['gum'],
		description: 'A tool for glamorous shell scripts',
		homepageUrl: 'https://github.com/charmbracelet/gum',
		name: 'Gum',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/gum/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	helm: {
		author: 'stk0vrfl0w',
		bins: ['helm'],
		description: 'The Kubernetes Package Manager.',
		homepageUrl: 'https://helm.sh',
		name: 'helm',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/helm.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/helm.toml',
	},
	helmfile: {
		author: 'stk0vrfl0w',
		bins: ['helmfile'],
		description: 'Deploy Kubernetes Helm Charts.',
		homepageUrl: 'https://helmfile.readthedocs.io/en/latest',
		name: 'helmfile',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/helmfile.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/helmfile.toml',
	},
	hugo: [
		{
			author: 'z0rrn',
			bins: ['hugo'],
			description: 'The world’s fastest framework for building websites - standard version.',
			homepageUrl: 'https://gohugo.io',
			name: 'Hugo Standard',
			pluginLocator:
				'source:https://raw.githubusercontent.com/z0rrn/proto-plugins/main/hugo/plugin-standard.toml',
			pluginType: 'toml',
			repoUrl: 'https://github.com/z0rrn/proto-plugins',
		},
		{
			author: 'z0rrn',
			bins: ['hugo'],
			description: 'The world’s fastest framework for building websites - extended version.',
			homepageUrl: 'https://gohugo.io',
			name: 'Hugo Extended',
			pluginLocator:
				'source:https://raw.githubusercontent.com/z0rrn/proto-plugins/main/hugo/plugin-extended.toml',
			pluginType: 'toml',
			repoUrl: 'https://github.com/z0rrn/proto-plugins',
		},
	],
	hurl: {
		author: 'Phault',
		bins: ['hurl'],
		description:
			'A command line tool that runs HTTP requests defined in a simple plain text format',
		homepageUrl: 'https://hurl.dev/',
		name: 'Hurl',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/hurl/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	hyperfine: {
		author: 'Phault',
		bins: ['hyperfine'],
		description: 'A command-line benchmarking tool',
		homepageUrl: 'https://github.com/sharkdp/hyperfine',
		name: 'hyperfine',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/hyperfine/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	infisical: {
		author: 'Phault',
		bins: ['infisical'],
		description:
			'The command-line interface for the open source secret management platform Infisical"',
		homepageUrl: 'https://infisical.com',
		name: 'Infisical',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/infisical/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	jira: {
		author: 'Phault',
		bins: ['jira'],
		description: 'An interactive command line tool for Atlassian Jira',
		homepageUrl: 'https://github.com/ankitpokhrel/jira-cli',
		name: 'JiraCLI',
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/jira/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	just: {
		author: 'Phault',
		bins: ['just'],
		description: 'A handy way to save and run project-specific commands',
		homepageUrl: 'https://github.com/casey/just',
		name: 'just',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/just/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	k3d: {
		author: 'appthrust',
		bins: ['k3d'],
		description:
			'k3d is a lightweight wrapper to run k3s (Rancher Lab’s minimal Kubernetes distribution) in docker.',
		homepageUrl: 'https://k3d.io',
		name: 'k3d',
		pluginLocator:
			'source:https://raw.githubusercontent.com/appthrust/proto-toml-plugins/main/k3d/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/appthrust/proto-toml-plugins',
	},
	kubectl: {
		author: 'stk0vrfl0w',
		bins: ['kubectl'],
		description: 'Kubernetes command line tool.',
		homepageUrl: 'https://kubernetes.io',
		name: 'kubectl',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/kubectl.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/kubectl.toml',
	},
	mage: {
		author: 'Phault',
		bins: ['mage'],
		description: 'A make/rake-like build tool using Go',
		homepageUrl: 'https://magefile.org',
		name: 'Mage',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/mage/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	mkcert: {
		author: 'Phault',
		bins: ['mkcert'],
		description:
			"A simple zero-config tool to make locally trusted development certificates with any names you'd like",
		homepageUrl: 'https://github.com/FiloSottile/mkcert',
		name: 'mkcert',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/mkcert/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	moon: {
		author: 'moonrepo',
		bins: ['moon'],
		description: 'moon is a multi-language build system and codebase management tool.',
		homepageUrl: 'https://moonrepo.dev/moon',
		name: 'moon',
		pluginLocator:
			'source:https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/moonrepo/moon/blob/master/proto-plugin.toml',
	},
	ninja: {
		author: 'Phault',
		bins: ['ninja'],
		description: 'A small build system with a focus on speed',
		homepageUrl: 'https://ninja-build.org',
		name: 'Ninja',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/ninja/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	octopus: {
		author: 'Phault',
		bins: ['octopus'],
		description: 'Command Line Interface for Octopus Deploy',
		homepageUrl: 'https://octopus.com/',
		name: 'Octopus CLI',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/octopus/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	'openapi-changes': {
		author: 'ngoldack',
		bins: ['openapi-changes'],
		description:
			"The world's sexiest OpenAPI breaking changes detector. Discover what changed between two OpenAPI specs, or a single spec over time. Supports OpenAPI 3.1, 3.0 and Swagger",
		homepageUrl: 'https://github.com/pb33f/openapi-changes',
		name: 'openapi-changes',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/ngoldack/proto-tools/main/tools/openapi-changes/openapi-changes.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/ngoldack/proto-tools',
	},
	oxlint: {
		author: 'Phault',
		bins: ['oxlint'],
		description:
			'Oxlint is a JavaScript linter designed to catch erroneous or useless code without requiring any configurations by default.',
		homepageUrl: 'https://oxc-project.github.io',
		name: 'oxlint',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/oxlint/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	'rattler-build': {
		author: 'Phault',
		bins: ['rattler-build'],
		description: 'A fast Conda package builder',
		homepageUrl: 'https://prefix-dev.github.io/rattler-build/',
		name: 'rattler-build',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/rattler-build/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	ruff: {
		author: 'Phault',
		bins: ['ruff'],
		description: 'An extremely fast Python linter and code formatter',
		homepageUrl: 'https://docs.astral.sh/ruff/',
		name: 'Ruff',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/ruff/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	shellcheck: {
		author: 'Phault',
		bins: ['shellcheck'],
		description: 'A static analysis tool for shell scripts',
		homepageUrl: 'https://github.com/koalaman/shellcheck',
		name: 'ShellCheck',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/shellcheck/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	shfmt: {
		author: 'Phault',
		bins: ['shfmt'],
		description: 'A shell formatter for POSIX Shell, Bash and mksh',
		homepageUrl: 'https://github.com/mvdan/sh',
		name: 'shfmt',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/shfmt/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	sops: {
		author: 'stk0vrfl0w',
		bins: ['sops'],
		description: 'Simple and flexible tool for managing secrets.',
		homepageUrl: 'https://github.com/getsops/sops',
		name: 'sops',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/sops.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/sops.toml',
	},
	task: {
		author: 'Phault',
		bins: ['task'],
		description:
			'Task is a task runner / build tool that aims to be simpler and easier to use than, for example, GNU Make',
		homepageUrl: 'https://taskfile.dev',
		name: 'Task',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/task/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	terraform: {
		author: 'stk0vrfl0w',
		bins: ['terraform'],
		description: 'Provision & Manage any Infrastructure.',
		homepageUrl: 'https://www.terraform.io',
		name: 'terraform',
		pluginLocator:
			'source:https://raw.githubusercontent.com/theomessin/proto-toml-plugins/master/terraform.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/theomessin/proto-toml-plugins/blob/master/terraform.toml',
	},
	terragrunt: {
		author: 'stk0vrfl0w',
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
	tilt: {
		author: 'appthrust',
		bins: ['tilt'],
		description: 'A toolkit for fixing the pains of microservice development.',
		homepageUrl: 'https://tilt.dev',
		name: 'tilt',
		pluginLocator:
			'source:https://raw.githubusercontent.com/appthrust/proto-toml-plugins/main/tilt/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/appthrust/proto-toml-plugins',
	},
	timoni: {
		author: 'b4nst',
		bins: ['timoni'],
		description: 'Distribution and lifecycle management for cloud-native applications.',
		homepageUrl: 'https://timoni.sh/',
		name: 'timoni',
		pluginLocator:
			'source:https://raw.githubusercontent.com/stefanprodan/timoni/main/proto-plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/stefanprodan/timoni',
	},
	traefik: {
		author: 'Phault',
		bins: ['traefik'],
		description:
			'A modern HTTP reverse proxy and load balancer that makes deploying microservices easy',
		homepageUrl: 'https://traefik.io/',
		name: 'Traefik',
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/traefik/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	trufflehog: {
		author: 'Phault',
		bins: ['trufflehog'],
		description: 'Find and verify credentials',
		homepageUrl: 'https://github.com/trufflesecurity/trufflehog',
		name: 'TruffleHog',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/trufflehog/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	uv: {
		author: 'Phault',
		bins: ['uv'],
		description: 'An extremely fast Python package installer and resolver',
		homepageUrl: 'https://github.com/astral-sh/uv',
		name: 'uv',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/uv/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	wiretap: {
		author: 'ngoldack',
		bins: ['wiretap'],
		description:
			"The world's coolest API Validation and compliance tool. Validate APIs against OpenAPI specifications and much more",
		homepageUrl: 'https://github.com/pb33f/wiretap',
		name: 'wiretap',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/ngoldack/proto-tools/main/tools/wiretap/wiretap.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/ngoldack/proto-tools',
	},
	wizer: {
		author: 'Phault',
		bins: ['wizer'],
		description: 'The WebAssembly Pre-Initializer',
		homepageUrl: 'https://github.com/bytecodealliance/wizer',
		name: 'Wizer',
		noIcon: true,
		pluginLocator:
			'source:https://raw.githubusercontent.com/Phault/proto-toml-plugins/main/wizer/plugin.toml',
		pluginType: 'toml',
		repoUrl: 'https://github.com/Phault/proto-toml-plugins',
	},
	zig: [
		{
			author: 'stk0vrfl0w',
			bins: ['zig'],
			description: 'Zig is a general-purpose programming language and toolchain.',
			homepageUrl: 'https://ziglang.org',
			name: 'zig',
			pluginLocator:
				'source:https://raw.githubusercontent.com/stk0vrfl0w/proto-toml-plugins/main/plugins/zig.toml',
			pluginType: 'toml',
			repoUrl: 'https://github.com/stk0vrfl0w/proto-toml-plugins/blob/main/plugins/zig.toml',
		},
		{
			author: 'konomae',
			bins: ['zig'],
			description: 'Zig is a general-purpose programming language and toolchain.',
			homepageUrl: 'https://ziglang.org',
			name: 'zig',
			pluginLocator: 'github:konomae/zig-plugin',
			pluginType: 'wasm',
			repoUrl: 'https://github.com/konomae/zig-plugin',
		},
	],
	zls: {
		author: 'konomae',
		bins: ['zls'],
		description: 'The Zig language server for all your Zig editor.',
		homepageUrl: 'https://github.com/zigtools/zls',
		name: 'zls',
		pluginLocator: 'github:konomae/zls-plugin',
		pluginType: 'wasm',
		repoUrl: 'https://github.com/konomae/zls-plugin',
	},
};
