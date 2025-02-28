"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[51084],{23672:(e,o,n)=>{n.r(o),n.d(o,{assets:()=>l,contentTitle:()=>r,default:()=>c,frontMatter:()=>i,metadata:()=>a,toc:()=>h});var t=n(24246),s=n(71670);const i={title:"FAQ"},r=void 0,a={id:"proto/faq",title:"FAQ",description:"General",source:"@site/docs/proto/faq.mdx",sourceDirName:"proto",slug:"/proto/faq",permalink:"/docs/proto/faq",draft:!1,unlisted:!1,editUrl:"https://github.com/moonrepo/moon/tree/master/website/docs/proto/faq.mdx",tags:[],version:"current",frontMatter:{title:"FAQ"},sidebar:"proto",previous:{title:"versions",permalink:"/docs/proto/commands/versions"}},l={},h=[{value:"General",id:"general",level:2},{value:"Where did the name &quot;proto&quot; come from?",id:"where-did-the-name-proto-come-from",level:3},{value:"Are you worried about confusion with other tools like protobufs?",id:"are-you-worried-about-confusion-with-other-tools-like-protobufs",level:3},{value:"What is a tool?",id:"what-is-a-tool",level:3},{value:"What is a backend?",id:"what-is-a-backend",level:3},{value:"What is a plugin?",id:"what-is-a-plugin",level:3},{value:"Will you support more languages?",id:"will-you-support-more-languages",level:3},{value:"Will you support other kinds of tools?",id:"will-you-support-other-kinds-of-tools",level:3},{value:"Do you support &quot;build from source&quot;?",id:"do-you-support-build-from-source",level:3},{value:"How to run a canary release after installing it?",id:"how-to-run-a-canary-release-after-installing-it",level:3},{value:"What kind of features are supported for HTTP requests?",id:"what-kind-of-features-are-supported-for-http-requests",level:3},{value:"Troubleshooting",id:"troubleshooting",level:2},{value:"Network requests keep failing, how can I bypass?",id:"network-requests-keep-failing-how-can-i-bypass",level:3}];function d(e){const o={a:"a",code:"code",em:"em",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.a)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsx)(o.h2,{id:"general",children:"General"}),"\n",(0,t.jsx)(o.h3,{id:"where-did-the-name-proto-come-from",children:'Where did the name "proto" come from?'}),"\n",(0,t.jsxs)(o.p,{children:["We wanted to keep with the space theme, and spent quite some time digging through Wikipedia and\nultimately landed on the page for ",(0,t.jsx)(o.a,{href:"https://en.wikipedia.org/wiki/Protostar",children:"protostar"}),' (this is why\nour logo\'s a star). We really liked the definition of protostar, as it basically means "the\nbeginning phase of a star". Even the the prefix proto means "first" or "earliest form of".']}),"\n",(0,t.jsx)(o.p,{children:"This was great as that's the impression we had in mind for our tool. proto is the first piece\nrequired for setting up your developer environment. The toolchain is the first layer in the\nfoundation."}),"\n",(0,t.jsx)(o.p,{children:"From an aesthetic standpoint, proto's typography works well with moon and moonbase, as most of the\nletters are circle shaped. Double points for proto having two o's like the other products!"}),"\n",(0,t.jsx)(o.h3,{id:"are-you-worried-about-confusion-with-other-tools-like-protobufs",children:"Are you worried about confusion with other tools like protobufs?"}),"\n",(0,t.jsx)(o.p,{children:"Nah."}),"\n",(0,t.jsx)(o.h3,{id:"what-is-a-tool",children:"What is a tool?"}),"\n",(0,t.jsxs)(o.p,{children:["A tool in the context of proto is either a language, dependency/package manager (typically for a\nlanguage), or third-party CLI. The tool is something that can be downloaded and installed ",(0,t.jsx)(o.em,{children:"by\nversion"})," onto a machine."]}),"\n",(0,t.jsxs)(o.p,{children:["Furthermore, a tool should have a primary executable file that can be executed with ",(0,t.jsx)(o.code,{children:"proto run"})," or\nthrough proto's shims. Additionally, a tool can also provide secondary executable files. For\nexample, ",(0,t.jsx)(o.code,{children:"npm"})," (the primary) also provides ",(0,t.jsx)(o.code,{children:"npx"})," and ",(0,t.jsx)(o.code,{children:"node-gyp"})," (secondaries)."]}),"\n",(0,t.jsx)(o.h3,{id:"what-is-a-backend",children:"What is a backend?"}),"\n",(0,t.jsx)(o.p,{children:"A backend is a special type of tool that provides additional integration with 3rd-party plugins,\ngreatly expanding what can be installed and managed with proto."}),"\n",(0,t.jsx)(o.h3,{id:"what-is-a-plugin",children:"What is a plugin?"}),"\n",(0,t.jsx)(o.p,{children:"A plugin is a WASM (or JSON, TOML, YAML) file for a tool or backend."}),"\n",(0,t.jsx)(o.p,{children:"The terms tool and plugin are often used interchangeably, but plugin primarily refers to the WASM\nportion of a tool, while tool refers to the entire package: metadata, business logic, branding, so\non an so forth."}),"\n",(0,t.jsx)(o.h3,{id:"will-you-support-more-languages",children:"Will you support more languages?"}),"\n",(0,t.jsxs)(o.p,{children:["Yes! We'd love to support as many as possible, and if you'd like to help, join our Discord\ncommunity! Feel free to create a ",(0,t.jsx)(o.a,{href:"./plugins",children:"plugin"})," in the mean time."]}),"\n",(0,t.jsx)(o.h3,{id:"will-you-support-other-kinds-of-tools",children:"Will you support other kinds of tools?"}),"\n",(0,t.jsxs)(o.p,{children:["No, we will only support languages, dependency managers, and CLIs, which should be enough. However,\nyou can create a ",(0,t.jsx)(o.a,{href:"./plugins",children:"plugin"})," to support other kinds of tools."]}),"\n",(0,t.jsx)(o.h3,{id:"do-you-support-build-from-source",children:'Do you support "build from source"?'}),"\n",(0,t.jsxs)(o.p,{children:["As of version 0.45, we do! Simple pass ",(0,t.jsx)(o.code,{children:"--build"})," to ",(0,t.jsx)(o.code,{children:"proto install"}),". However, building from source\nis a complicated process and is unique per tool, so not all tools support it."]}),"\n",(0,t.jsx)(o.h3,{id:"how-to-run-a-canary-release-after-installing-it",children:"How to run a canary release after installing it?"}),"\n",(0,t.jsxs)(o.p,{children:["Once a tool has been installed with ",(0,t.jsx)(o.code,{children:"--canary"}),", the canary version can be explicitly referenced\nusing our ",(0,t.jsx)(o.a,{href:"./detection",children:"version detection rules"}),". The easiest approach is to prefix the shim with an\nenvironment variable:"]}),"\n",(0,t.jsx)(o.pre,{children:(0,t.jsx)(o.code,{className:"language-shell",children:"$ PROTO_BUN_VERSION=canary bun ./index.ts\n"})}),"\n",(0,t.jsxs)(o.p,{children:["Or to explicitly configure the version in ",(0,t.jsx)(o.a,{href:"./config",children:(0,t.jsx)(o.code,{children:".prototools"})}),":"]}),"\n",(0,t.jsx)(o.pre,{children:(0,t.jsx)(o.code,{className:"language-toml",children:'bun = "canary"\n'})}),"\n",(0,t.jsx)(o.h3,{id:"what-kind-of-features-are-supported-for-http-requests",children:"What kind of features are supported for HTTP requests?"}),"\n",(0,t.jsx)(o.p,{children:"proto makes a lot of HTTP requests, for information such as available versions/releases, and for\ndownloading the blobs/archives themselves. Because of this, we do our best to support all kinds of\ninternet connections, proxy and intranet usage, and more, through the following:"}),"\n",(0,t.jsxs)(o.ul,{children:["\n",(0,t.jsxs)(o.li,{children:["All GET and HEAD requests are cached to ",(0,t.jsx)(o.code,{children:"~/.proto/cache/requests"})," based on the\n",(0,t.jsx)(o.a,{href:"https://github.com/kornelski/rusty-http-cache-semantics",children:"HTTP cache semantics"})," and relevant RFCs."]}),"\n",(0,t.jsxs)(o.li,{children:["We support the\n",(0,t.jsx)(o.a,{href:"https://www.gnu.org/software/inetutils/manual/html_node/The-_002enetrc-file.html",children:"netrc file format"}),"\nand will automatically load ",(0,t.jsx)(o.code,{children:"~/.netrc"})," if it exists."]}),"\n",(0,t.jsxs)(o.li,{children:["We support an offline mode that will short-circuit certain workflows if there's no internet\nconnection. We check for a connection by pinging DNS endpoints, but this can be configured with\n",(0,t.jsx)(o.a,{href:"./config#settingsoffline",children:(0,t.jsx)(o.code,{children:"[settings.offline]"})}),"."]}),"\n",(0,t.jsxs)(o.li,{children:["We attempt to automatically load root and system certifications so that secure connections work\ncorrectly. This can be configured with ",(0,t.jsx)(o.a,{href:"./config#settingshttp",children:(0,t.jsx)(o.code,{children:"[settings.http]"})}),"."]}),"\n"]}),"\n",(0,t.jsx)(o.h2,{id:"troubleshooting",children:"Troubleshooting"}),"\n",(0,t.jsx)(o.h3,{id:"network-requests-keep-failing-how-can-i-bypass",children:"Network requests keep failing, how can I bypass?"}),"\n",(0,t.jsx)(o.p,{children:"When a tool is executed, we validate the version to ensure it's correct. We achieve this by making\nnetwork requests to a remote service to gather the list of valid versions. If you're having network\nissues, or the request is timing out, you can bypass these checks with the following:"}),"\n",(0,t.jsxs)(o.ul,{children:["\n",(0,t.jsxs)(o.li,{children:["\n",(0,t.jsx)(o.p,{children:"Pass a fully-qualified version as an environment variable. The version must be installed for this\nto work."}),"\n",(0,t.jsx)(o.pre,{children:(0,t.jsx)(o.code,{className:"language-shell",children:"PROTO_NODE_VERSION=20.0.0 node --version\n"})}),"\n",(0,t.jsxs)(o.p,{children:["If executing a Node.js package manager, you'll need to set versions for both Node.js and the\nmanager. This is required since manager's execute ",(0,t.jsx)(o.code,{children:"node"})," processes under the hood."]}),"\n",(0,t.jsx)(o.pre,{children:(0,t.jsx)(o.code,{className:"language-shell",children:"PROTO_NODE_VERSION=20.0.0 PROTO_NPM_VERSION=10.0.0 npm --version\n"})}),"\n"]}),"\n",(0,t.jsxs)(o.li,{children:["\n",(0,t.jsxs)(o.p,{children:["Pass the ",(0,t.jsx)(o.code,{children:"PROTO_BYPASS_VERSION_CHECK"})," environment variable. This will bypass the network request\nto load versions, but does not bypass other requests. However, this is typically enough."]}),"\n",(0,t.jsx)(o.pre,{children:(0,t.jsx)(o.code,{className:"language-shell",children:"PROTO_BYPASS_VERSION_CHECK=1 node --version\n"})}),"\n"]}),"\n"]})]})}function c(e={}){const{wrapper:o}={...(0,s.a)(),...e.components};return o?(0,t.jsx)(o,{...e,children:(0,t.jsx)(d,{...e})}):d(e)}},71670:(e,o,n)=>{n.d(o,{Z:()=>a,a:()=>r});var t=n(27378);const s={},i=t.createContext(s);function r(e){const o=t.useContext(i);return t.useMemo((function(){return"function"==typeof e?e(o):{...o,...e}}),[o,e])}function a(e){let o;return o=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:r(e.components),t.createElement(i.Provider,{value:o},e.children)}}}]);