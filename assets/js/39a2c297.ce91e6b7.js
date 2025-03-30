"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[389],{2080:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>p,contentTitle:()=>a,default:()=>d,frontMatter:()=>l,metadata:()=>o,toc:()=>u});var o=t(17264),i=t(62540),s=t(43023),r=t(43067);const l={slug:"proto-v0.12",title:"proto v0.12 - Experimental WASM plugins",authors:["milesj"],tags:["proto","wasm","plugin"]},a=void 0,p={authorsImageUrls:[void 0]},u=[{value:"WASM plugins",id:"wasm-plugins",level:2},{value:"Using WASM plugins",id:"using-wasm-plugins",level:3},{value:"Example implementation",id:"example-implementation",level:3},{value:"Other changes",id:"other-changes",level:2}];function m(e){const n={a:"a",code:"code",h2:"h2",h3:"h3",p:"p",pre:"pre",...(0,s.R)(),...e.components};return(0,i.jsxs)(i.Fragment,{children:[(0,i.jsx)(n.p,{children:"After months of hard work, we're excited to release our first iteration of WASM plugins for proto."}),"\n",(0,i.jsx)(n.h2,{id:"wasm-plugins",children:"WASM plugins"}),"\n",(0,i.jsxs)(n.p,{children:["Three months ago, we ",(0,i.jsx)(n.a,{href:"https://github.com/moonrepo/proto/issues/73",children:"published an RFC"})," for supporting\nplugins in proto. Plugins are a must, as they allow consumers to easily extend proto with additional\ntools, instead of them being built into proto core (which is very time consuming)."]}),"\n",(0,i.jsxs)(n.p,{children:["Two months ago, we released support for ",(0,i.jsx)(n.a,{href:"/docs/proto/non-wasm-plugin",children:"TOML plugins"}),". These are very\nsimple plugins powered by static configuration files. They're great for simple tools like CLIs and\npre-built languages, as everything is powered by static HTTP URLs and file names. However, sometimes\nyou need dynamic control..."]}),"\n",(0,i.jsxs)(n.p,{children:["And after 2 months of development, and help from the ",(0,i.jsx)(n.a,{href:"https://extism.org/",children:"Extism team"}),", we're\nexcited to announce initial support for WASM plugins. WASM is a portable binary format, with\nsandboxed access to the file system (via WASI), and the ability to execute processes and fetch URLs.\nThis means that plugins can be written in any language that compiles to WASM, like Rust, C, C++, Go,\nTypeScript, and more. This removes the requirement of writing Rust and contributing to proto\ndirectly!"]}),"\n",(0,i.jsx)("div",{class:"flex justify-center",children:(0,i.jsx)(r.A,{label:"View WASM plugin guide",href:"/docs/proto/wasm-plugin",size:"lg"})}),"\n",(0,i.jsx)(n.h3,{id:"using-wasm-plugins",children:"Using WASM plugins"}),"\n",(0,i.jsxs)(n.p,{children:["Once the ",(0,i.jsx)(n.code,{children:".wasm"})," file is publicly available for download, we can configure it as a plugin in\n",(0,i.jsx)(n.a,{href:"/docs/proto/config",children:(0,i.jsx)(n.code,{children:".prototools"})}),"."]}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-toml",children:'[plugins]\nmy-plugin = "source:https://domain.com/path/to/wasm/plugin.wasm"\n'})}),"\n",(0,i.jsxs)(n.p,{children:["And execute all ",(0,i.jsx)(n.code,{children:"proto"})," commands using the configured plugin identifier."]}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-shell",children:"proto install my-plugin\n"})}),"\n",(0,i.jsx)(n.h3,{id:"example-implementation",children:"Example implementation"}),"\n",(0,i.jsx)(n.p,{children:"The official guide above walks you through creating a plugin, but to demonstrate the power of WASM\nplugins, here's an example function that defines parameters for downloading and installing Node.js.\nThis is written in Rust and using Extism's official PDK."}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-rust",children:'#[plugin_fn]\npub fn download_prebuilt(\n    Json(input): Json<DownloadPrebuiltInput>,\n) -> FnResult<Json<DownloadPrebuiltOutput>> {\n    let version = input.env.version;\n    let arch = input.env.arch;\n\n    let prefix = match input.env.os {\n        HostOS::Linux => format!("node-v{version}-linux-{arch}"),\n        HostOS::MacOS => format!("node-v{version}-darwin-{arch}"),\n        HostOS::Windows => format!("node-v{version}-win-{arch}"),\n        other => {\n            return Err(PluginError::UnsupportedPlatform {\n                tool: NAME.into(),\n                platform: format!("{:?}", other),\n            })?;\n        }\n    };\n\n    let filename = if input.env.os == HostOS::Windows {\n        format!("{prefix}.zip")\n    } else {\n        format!("{prefix}.tar.xz")\n    };\n\n    Ok(Json(DownloadPrebuiltOutput {\n        archive_prefix: Some(prefix),\n        download_url: format!("https://nodejs.org/dist/v{version}/{filename}"),\n        download_name: Some(filename),\n        checksum_url: Some(format!("https://nodejs.org/dist/v{version}/SHASUMS256.txt")),\n        ..DownloadPrebuiltOutput::default()\n    }))\n}\n'})}),"\n",(0,i.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,i.jsxs)(n.p,{children:["View the ",(0,i.jsx)(n.a,{href:"https://github.com/moonrepo/proto/releases/tag/v0.12.0",children:"official release"})," for a full list\nof changes."]})]})}function d(e={}){const{wrapper:n}={...(0,s.R)(),...e.components};return n?(0,i.jsx)(n,{...e,children:(0,i.jsx)(m,{...e})}):m(e)}},17264:e=>{e.exports=JSON.parse('{"permalink":"/blog/proto-v0.12","editUrl":"https://github.com/moonrepo/moon/tree/master/website/blog/2023-07-07_proto-v0.12.mdx","source":"@site/blog/2023-07-07_proto-v0.12.mdx","title":"proto v0.12 - Experimental WASM plugins","description":"After months of hard work, we\'re excited to release our first iteration of WASM plugins for proto.","date":"2023-07-07T00:00:00.000Z","tags":[{"inline":true,"label":"proto","permalink":"/blog/tags/proto"},{"inline":true,"label":"wasm","permalink":"/blog/tags/wasm"},{"inline":true,"label":"plugin","permalink":"/blog/tags/plugin"}],"readingTime":1.885,"hasTruncateMarker":true,"authors":[{"name":"Miles Johnson","title":"Founder, developer","url":"https://github.com/milesj","imageURL":"/img/authors/miles.jpg","key":"milesj","page":null}],"frontMatter":{"slug":"proto-v0.12","title":"proto v0.12 - Experimental WASM plugins","authors":["milesj"],"tags":["proto","wasm","plugin"]},"unlisted":false,"prevItem":{"title":"moon v1.10 - Mid-year quality of life improvements","permalink":"/blog/moon-v1.10"},"nextItem":{"title":"moon v1.9 - VCS hooks management and improved task inheritance","permalink":"/blog/moon-v1.9"}}')},43023:(e,n,t)=>{t.d(n,{R:()=>r,x:()=>l});var o=t(63696);const i={},s=o.createContext(i);function r(e){const n=o.useContext(s);return o.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function l(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(i):e.components||i:r(e.components),o.createElement(s.Provider,{value:n},e.children)}}}]);