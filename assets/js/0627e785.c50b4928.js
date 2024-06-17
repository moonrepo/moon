"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[74824],{45543:(e,n,o)=>{o.r(n),o.d(n,{assets:()=>a,contentTitle:()=>r,default:()=>p,frontMatter:()=>l,metadata:()=>c,toc:()=>d});var t=o(24246),s=o(71670),i=o(79022);const l={title:"Configuration",toc_max_heading_level:6},r=void 0,c={id:"proto/config",title:"Configuration",description:"We support configuration at both the project-level and user-level using a",source:"@site/docs/proto/config.mdx",sourceDirName:"proto",slug:"/proto/config",permalink:"/docs/proto/config",draft:!1,unlisted:!1,editUrl:"https://github.com/moonrepo/moon/tree/master/website/docs/proto/config.mdx",tags:[],version:"current",frontMatter:{title:"Configuration",toc_max_heading_level:6},sidebar:"proto",previous:{title:"Version detection",permalink:"/docs/proto/detection"},next:{title:"Supported tools",permalink:"/docs/proto/tools"}},a={},d=[{value:"Resolution order",id:"resolution-order",level:2},{value:"Environment mode<VersionLabel></VersionLabel>",id:"environment-mode",level:3},{value:"Pinning versions",id:"pinning-versions",level:2},{value:"Available settings",id:"available-settings",level:2},{value:"<code>[env]</code><VersionLabel></VersionLabel>",id:"env",level:3},{value:"<code>[settings]</code>",id:"settings",level:3},{value:"<code>auto-install</code>",id:"auto-install",level:4},{value:"<code>auto-clean</code>",id:"auto-clean",level:4},{value:"<code>detect-strategy</code>",id:"detect-strategy",level:4},{value:"<code>pin-latest</code>",id:"pin-latest",level:4},{value:"<code>telemetry</code>",id:"telemetry",level:4},{value:"<code>[settings.http]</code>",id:"settingshttp",level:3},{value:"<code>allow-invalid-certs</code>",id:"allow-invalid-certs",level:4},{value:"<code>proxies</code>",id:"proxies",level:4},{value:"<code>root-cert</code>",id:"root-cert",level:4},{value:"<code>[plugins]</code>",id:"plugins",level:3},{value:"Tool specific settings",id:"tool-specific-settings",level:2},{value:"<code>[tools.*]</code>",id:"tools",level:3},{value:"<code>[tools.*.aliases]</code>",id:"toolsaliases",level:3},{value:"<code>[tools.*.env]</code><VersionLabel></VersionLabel>",id:"toolsenv",level:3},{value:"GitHub Action",id:"github-action",level:2}];function h(e){const n={a:"a",blockquote:"blockquote",code:"code",em:"em",h2:"h2",h3:"h3",h4:"h4",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.a)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsxs)(n.p,{children:["We support configuration at both the project-level and user-level using a\n",(0,t.jsx)(n.a,{href:"https://toml.io/en/",children:"TOML"})," based ",(0,t.jsx)(n.code,{children:".prototools"})," file. This file can be used to pin versions of\ntools, provide tool specific configuration, enable new tools via plugins, define proto settings, and\nmore."]}),"\n",(0,t.jsx)(n.h2,{id:"resolution-order",children:"Resolution order"}),"\n",(0,t.jsxs)(n.p,{children:["When a ",(0,t.jsx)(n.code,{children:"proto"})," command or shim is ran, we load the ",(0,t.jsx)(n.code,{children:".prototools"})," file in the current directory, and\ntraverse upwards loading ",(0,t.jsx)(n.code,{children:".prototools"})," within each directory, until we reach the system root or the\nuser directory (",(0,t.jsx)(n.code,{children:"~"}),"), whichever comes first. Once complete, we then load the special\n",(0,t.jsx)(n.code,{children:"~/.proto/.prototools"})," file, which acts as configuration at the user-level and includes fallback\nsettings. This operation may look like the following:"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-text",children:"~/Projects/web-app/packages/components/src/.prototools\n~/Projects/web-app/packages/components/.prototools\n~/Projects/web-app/packages/.prototools\n~/Projects/web-app/.prototools\n~/Projects/.prototools\n~/.prototools\n~/.proto/.prototools\n"})}),"\n",(0,t.jsx)(n.p,{children:"We then deeply merge all of these configuration files into a final configuration object, with the\ncurrent directory taking highest precedence."}),"\n",(0,t.jsxs)(n.h3,{id:"environment-mode",children:["Environment mode",(0,t.jsx)(i.Z,{version:"0.29.0"})]}),"\n",(0,t.jsxs)(n.p,{children:["We also support environment specific configuration, such as ",(0,t.jsx)(n.code,{children:".prototools.production"})," or\n",(0,t.jsx)(n.code,{children:".prototools.development"}),", when the ",(0,t.jsx)(n.code,{children:"PROTO_ENV"})," environment variable is set. This is useful for\ndefining environment specific aliases, or tool specific configuration."]}),"\n",(0,t.jsxs)(n.p,{children:["These environment aware settings take precedence over the default ",(0,t.jsx)(n.code,{children:".prototools"})," file, for the\ndirectory it's located in, and are merged in the same way as the default configuration. For example,\nthe lookup order would be the following when ",(0,t.jsx)(n.code,{children:"PROTO_ENV=production"}),":"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-text",children:"...\n~/Projects/.prototools.production\n~/Projects/.prototools\n~/.prototools.production\n~/.prototools\n~/.proto/.prototools\n"})}),"\n",(0,t.jsxs)(n.blockquote,{children:["\n",(0,t.jsxs)(n.p,{children:["The global ",(0,t.jsx)(n.code,{children:"~/.proto/.prototools"})," file does not support environment modes."]}),"\n"]}),"\n",(0,t.jsx)(n.h2,{id:"pinning-versions",children:"Pinning versions"}),"\n",(0,t.jsxs)(n.p,{children:["proto supports pinning versions of tools on a per-directory basis through our ",(0,t.jsx)(n.code,{children:".prototools"}),"\nconfiguration file. This file takes precedence during ",(0,t.jsx)(n.a,{href:"./detection",children:"version detection"})," and can be\ncreated/updated with ",(0,t.jsx)(n.a,{href:"./commands/pin",children:(0,t.jsx)(n.code,{children:"proto pin"})}),"."]}),"\n",(0,t.jsx)(n.p,{children:"At its most basic level, you can map tools to specific versions, for the directory the file is\nlocated in. A version can either be a fully-qualified semantic version, a partial version, a range\nor requirement, or an alias."}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'node = "16.16.0"\nnpm = "9"\ngo = "~1.20"\nrust = "stable"\n'})}),"\n",(0,t.jsx)(n.h2,{id:"available-settings",children:"Available settings"}),"\n",(0,t.jsxs)(n.h3,{id:"env",children:[(0,t.jsx)(n.code,{children:"[env]"}),(0,t.jsx)(i.Z,{version:"0.29.0"})]}),"\n",(0,t.jsxs)(n.p,{children:["This setting is a map of environment variables that will be applied to ",(0,t.jsx)(n.em,{children:"all"})," tools when they are\nexecuted. Variables defined here ",(0,t.jsx)(n.em,{children:"will not"})," override existing environment variables (either passed\non the command line, or inherited from the shell)."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'[env]\nDEBUG = "*"\n'})}),"\n",(0,t.jsxs)(n.p,{children:["Additionally, ",(0,t.jsx)(n.code,{children:"false"})," can be provided as a value, which will ",(0,t.jsx)(n.em,{children:"remove"})," the environment variable when\nthe tool is executed. This is useful for removing inherited shell variables."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:"[env]\nDEBUG = false\n"})}),"\n",(0,t.jsxs)(n.p,{children:["Variables also support substitution using the syntax ",(0,t.jsx)(n.code,{children:"${VAR_NAME}"}),". When using substitution,\nvariables in the current process and merged ",(0,t.jsx)(n.code,{children:"[env]"})," can be referenced. Recursive substitution is not\nsupported!"]}),"\n",(0,t.jsxs)(n.blockquote,{children:["\n",(0,t.jsx)(n.p,{children:"This functionality enables per-directory environment variables!"}),"\n"]}),"\n",(0,t.jsx)(n.h3,{id:"settings",children:(0,t.jsx)(n.code,{children:"[settings]"})}),"\n",(0,t.jsx)(n.h4,{id:"auto-install",children:(0,t.jsx)(n.code,{children:"auto-install"})}),"\n",(0,t.jsxs)(n.p,{children:["When enabled, will automatically installing missing tools when ",(0,t.jsx)(n.a,{href:"./commands/run",children:(0,t.jsx)(n.code,{children:"proto run"})})," is run,\ninstead of erroring. Defaults to ",(0,t.jsx)(n.code,{children:"false"})," or ",(0,t.jsx)(n.code,{children:"PROTO_AUTO_INSTALL"}),"."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:"[settings]\nauto-install = true\n"})}),"\n",(0,t.jsx)(n.h4,{id:"auto-clean",children:(0,t.jsx)(n.code,{children:"auto-clean"})}),"\n",(0,t.jsxs)(n.p,{children:["When enabled, will automatically clean up the proto cache when ",(0,t.jsx)(n.a,{href:"./commands/use",children:(0,t.jsx)(n.code,{children:"proto use"})})," is run.\nDefaults to ",(0,t.jsx)(n.code,{children:"false"})," or ",(0,t.jsx)(n.code,{children:"PROTO_AUTO_CLEAN"}),"."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:"[settings]\nauto-clean = true\n"})}),"\n",(0,t.jsx)(n.h4,{id:"detect-strategy",children:(0,t.jsx)(n.code,{children:"detect-strategy"})}),"\n",(0,t.jsxs)(n.p,{children:["The strategy to use when ",(0,t.jsx)(n.a,{href:"./detection",children:"detecting versions"}),". Defaults to ",(0,t.jsx)(n.code,{children:"first-available"})," or\n",(0,t.jsx)(n.code,{children:"PROTO_DETECT_STRATEGY"}),"."]}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"first-available"})," - Will use the first available version that is found. Either from ",(0,t.jsx)(n.code,{children:".prototools"}),"\nor a tool specific file (",(0,t.jsx)(n.code,{children:".nvmrc"}),", etc)."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"prefer-prototools"})," - Prefer a ",(0,t.jsx)(n.code,{children:".prototools"})," version, even if found in a parent directory. If none\nfound, falls back to tool specific file."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"only-prototools"})," - Only use a version defined in ",(0,t.jsx)(n.code,{children:".prototools"}),". ",(0,t.jsx)(i.Z,{version:"0.34.0"})]}),"\n"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'[settings]\ndetect-strategy = "prefer-prototools"\n'})}),"\n",(0,t.jsx)(n.h4,{id:"pin-latest",children:(0,t.jsx)(n.code,{children:"pin-latest"})}),"\n",(0,t.jsxs)(n.p,{children:['When defined and a tool is installed with the "latest" version, will automatically pin the resolved\nversion to the configured location. Defaults to disabled or ',(0,t.jsx)(n.code,{children:"PROTO_PIN_LATEST"}),"."]}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"global"})," - Pins globally to ",(0,t.jsx)(n.code,{children:"~/.proto/.prototools"}),"."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"local"})," - Pins locally to ",(0,t.jsx)(n.code,{children:".prototools"})," in current directory."]}),"\n"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'[settings]\npin-latest = "local"\n'})}),"\n",(0,t.jsx)(n.h4,{id:"telemetry",children:(0,t.jsx)(n.code,{children:"telemetry"})}),"\n",(0,t.jsxs)(n.p,{children:["When enabled, we collect anonymous usage statistics for tool installs and uninstalls. This helps us\nprioritize which tools to support, what tools or their versions may be broken, the plugins currently\nin use, and more. Defaults to ",(0,t.jsx)(n.code,{children:"true"}),"."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:"[settings]\ntelemetry = false\n"})}),"\n",(0,t.jsxs)(n.blockquote,{children:["\n",(0,t.jsxs)(n.p,{children:["The data we track is publicly available and\n",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/proto/blob/master/legacy/cli/src/telemetry.rs",children:"can be found here"}),"."]}),"\n"]}),"\n",(0,t.jsx)(n.h3,{id:"settingshttp",children:(0,t.jsx)(n.code,{children:"[settings.http]"})}),"\n",(0,t.jsx)(n.p,{children:"Can be used to customize the HTTP client used by proto, primarily for requesting files to download,\navailable versions, and more."}),"\n",(0,t.jsx)(n.h4,{id:"allow-invalid-certs",children:(0,t.jsx)(n.code,{children:"allow-invalid-certs"})}),"\n",(0,t.jsxs)(n.p,{children:["When enabled, will allow invalid certificates instead of failing. This is an ",(0,t.jsx)(n.em,{children:"escape hatch"})," and\nshould only be used if other settings have failed. Be sure you know what you're doing! Defaults to\n",(0,t.jsx)(n.code,{children:"false"}),"."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:"[settings.http]\nallow-invalid-certs = true\n"})}),"\n",(0,t.jsx)(n.h4,{id:"proxies",children:(0,t.jsx)(n.code,{children:"proxies"})}),"\n",(0,t.jsxs)(n.p,{children:["A list of proxy URLs to use for requests. As an alternative, the ",(0,t.jsx)(n.code,{children:"HTTPS_PROXY"})," environment variable\ncan be set."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'[settings.http]\nproxies = ["https://internal.proxy", "https://corp.net/proxy"]\n'})}),"\n",(0,t.jsx)(n.h4,{id:"root-cert",children:(0,t.jsx)(n.code,{children:"root-cert"})}),"\n",(0,t.jsxs)(n.p,{children:["The path to a root certificate to use for requests. This is useful for overriding the native\ncertificate, or for using a self-signed certificate, especially when in a corporate/internal\nenvironment. Supports ",(0,t.jsx)(n.code,{children:"pem"})," and ",(0,t.jsx)(n.code,{children:"der"})," files."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'[settings.http]\nroot-cert = "/path/to/root/cert.pem"\n'})}),"\n",(0,t.jsx)(n.h3,{id:"plugins",children:(0,t.jsx)(n.code,{children:"[plugins]"})}),"\n",(0,t.jsxs)(n.p,{children:["Additional ",(0,t.jsx)(n.a,{href:"./plugins",children:"plugins"})," can be configured with the ",(0,t.jsx)(n.code,{children:"[plugins]"})," section.\n",(0,t.jsx)(n.a,{href:"./plugins#enabling-plugins",children:"Learn more about this syntax"}),"."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'[plugins]\nmy-tool = "https://raw.githubusercontent.com/my/tool/master/proto-plugin.toml"\n'})}),"\n",(0,t.jsx)(n.p,{children:"Once configured, you can run a plugin as if it was a built-in tool:"}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:"$ proto install my-tool\n"})}),"\n",(0,t.jsx)(n.h2,{id:"tool-specific-settings",children:"Tool specific settings"}),"\n",(0,t.jsx)(n.h3,{id:"tools",children:(0,t.jsx)(n.code,{children:"[tools.*]"})}),"\n",(0,t.jsxs)(n.p,{children:["Tools support custom configuration that will be passed to their WASM plugin, which can be used to\ncontrol the business logic within the plugin. Please refer to the ",(0,t.jsx)(n.a,{href:"./tools",children:"official documentation"}),"\nof each tool (typically on their repository) for a list of available settings."]}),"\n",(0,t.jsxs)(n.p,{children:["As an example, let's configure ",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/node-plugin",children:"Node.js"})," (using the ",(0,t.jsx)(n.code,{children:"node"}),"\nidentifier)."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:"[tools.node]\nbundled-npm = true\n\n[tools.npm]\nshared-globals-dir = true\n"})}),"\n",(0,t.jsx)(n.h3,{id:"toolsaliases",children:(0,t.jsx)(n.code,{children:"[tools.*.aliases]"})}),"\n",(0,t.jsxs)(n.p,{children:["Aliases are custom and unique labels that map to a specific version, and can be configured manually\nwithin ",(0,t.jsx)(n.code,{children:".prototools"}),", or by calling the ",(0,t.jsx)(n.a,{href:"./commands/alias",children:(0,t.jsx)(n.code,{children:"proto alias"})})," command."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'[tools.node.aliases]\nwork = "18"\noss = "20"\n'})}),"\n",(0,t.jsxs)(n.h3,{id:"toolsenv",children:[(0,t.jsx)(n.code,{children:"[tools.*.env]"}),(0,t.jsx)(i.Z,{version:"0.29.0"})]}),"\n",(0,t.jsxs)(n.p,{children:["This setting is a map of environment variables for a specific tool, and will be applied when that\ntool is executed. These variables will override those defined in ",(0,t.jsx)(n.code,{children:"[env]"}),". Refer to ",(0,t.jsx)(n.a,{href:"#env",children:(0,t.jsx)(n.code,{children:"[env]"})}),"\nfor usage examples."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'[tools.node.env]\nNODE_ENV = "production"\n'})}),"\n",(0,t.jsx)(n.h2,{id:"github-action",children:"GitHub Action"}),"\n",(0,t.jsxs)(n.p,{children:["To streamline GitHub CI workflows, we provide the\n",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/setup-toolchain",children:(0,t.jsx)(n.code,{children:"moonrepo/setup-toolchain"})})," action, which can be used\nto install ",(0,t.jsx)(n.code,{children:"proto"})," globally, and cache the toolchain found at ",(0,t.jsx)(n.code,{children:"~/.proto"}),"."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-yaml",metastring:'title=".github/workflows/ci.yml"',children:"# ...\njobs:\n  ci:\n    name: 'CI'\n    runs-on: 'ubuntu-latest'\n    steps:\n      - uses: 'actions/checkout@v4'\n      - uses: 'moonrepo/setup-toolchain@v0'\n        with:\n          auto-install: true\n"})})]})}function p(e={}){const{wrapper:n}={...(0,s.a)(),...e.components};return n?(0,t.jsx)(n,{...e,children:(0,t.jsx)(h,{...e})}):h(e)}},79022:(e,n,o)=>{o.d(n,{Z:()=>i});var t=o(9619),s=o(24246);function i(e){let{header:n,inline:o,updated:i,version:l}=e;return(0,s.jsx)(t.Z,{text:`v${l}`,variant:i?"success":"info",className:n?"absolute right-0 top-1.5":o?"inline-block":"ml-2"})}},9619:(e,n,o)=>{o.d(n,{Z:()=>r});var t=o(40624),s=o(31792),i=o(24246);const l={failure:"bg-red-100 text-red-900",info:"bg-pink-100 text-pink-900",success:"bg-green-100 text-green-900",warning:"bg-orange-100 text-orange-900"};function r(e){let{className:n,icon:o,text:r,variant:c}=e;return(0,i.jsxs)("span",{className:(0,t.Z)("inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase",c?l[c]:"bg-gray-100 text-gray-800",n),children:[o&&(0,i.jsx)(s.Z,{icon:o,className:"mr-1"}),r]})}},71670:(e,n,o)=>{o.d(n,{Z:()=>r,a:()=>l});var t=o(27378);const s={},i=t.createContext(s);function l(e){const n=t.useContext(i);return t.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function r(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:l(e.components),t.createElement(i.Provider,{value:n},e.children)}}}]);