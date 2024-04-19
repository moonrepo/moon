"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[20728],{92199:(e,n,i)=>{i.r(n),i.d(n,{assets:()=>d,contentTitle:()=>r,default:()=>h,frontMatter:()=>t,metadata:()=>l,toc:()=>a});var s=i(24246),o=i(71670);const t={slug:"proto-v0.20",title:"proto v0.20 - New shims and binaries management",authors:["milesj"],tags:["proto","bin","shim","checksum"]},r=void 0,l={permalink:"/blog/proto-v0.20",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2023-10-20_proto-v0.20.mdx",source:"@site/blog/2023-10-20_proto-v0.20.mdx",title:"proto v0.20 - New shims and binaries management",description:"In this release, we're reworking how shims and binaries work.",date:"2023-10-20T00:00:00.000Z",tags:[{label:"proto",permalink:"/blog/tags/proto"},{label:"bin",permalink:"/blog/tags/bin"},{label:"shim",permalink:"/blog/tags/shim"},{label:"checksum",permalink:"/blog/tags/checksum"}],readingTime:3,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"proto-v0.20",title:"proto v0.20 - New shims and binaries management",authors:["milesj"],tags:["proto","bin","shim","checksum"]},unlisted:!1,prevItem:{title:"proto v0.21 - Linux x64 musl support",permalink:"/blog/proto-v0.21"},nextItem:{title:"moon v1.15 - Next-generation action graph",permalink:"/blog/moon-v1.15"}},d={authorsImageUrls:[void 0]},a=[{value:"Shims <em>and</em> Binaries (breaking)",id:"shims-and-binaries-breaking",level:2},{value:"How it works",id:"how-it-works",level:3},{value:"Comparison",id:"comparison",level:3},{value:"Support for minisign checksums",id:"support-for-minisign-checksums",level:2},{value:"Other changes",id:"other-changes",level:2}];function c(e){const n={a:"a",code:"code",em:"em",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",strong:"strong",table:"table",tbody:"tbody",td:"td",th:"th",thead:"thead",tr:"tr",ul:"ul",...(0,o.a)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsx)(n.p,{children:"In this release, we're reworking how shims and binaries work."}),"\n",(0,s.jsxs)(n.h2,{id:"shims-and-binaries-breaking",children:["Shims ",(0,s.jsx)(n.em,{children:"and"})," Binaries (breaking)"]}),"\n",(0,s.jsx)(n.p,{children:"Since proto's inception, we've used shims as a way to execute installed tools. This allowed us to\nwrap the underlying tool binary to provide additional functionality, such as automatic version\ndetection, runtime hooks, and more. However, this approach has some limitations, such as:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsx)(n.li,{children:"Shims are forced onto you and there's no way to use proto without shims."}),"\n",(0,s.jsx)(n.li,{children:"Shims are slower than executing the native binary, upwards of 10x slower. While this equates in\nmilliseconds, it can be noticeable dependending on the tool."}),"\n",(0,s.jsxs)(n.li,{children:["For Windows, our shim files are ",(0,s.jsx)(n.code,{children:".cmd"})," ",(0,s.jsx)(n.em,{children:"and not"})," ",(0,s.jsx)(n.code,{children:".exe"}),". This causes a lot of weird and unexpected\nproblems when an environment expects a real executable, or uses a hard-coded ",(0,s.jsx)(n.code,{children:".exe"})," extension."]}),"\n"]}),"\n",(0,s.jsxs)(n.p,{children:["To remedy this, we're introducing both a shim and non-shim approach, which has resulted in a pretty\nbig breaking change. Shims are now generated in ",(0,s.jsx)(n.code,{children:"~/.proto/shims"})," (instead of ",(0,s.jsx)(n.code,{children:"~/.proto/bin"}),"), while\n",(0,s.jsx)(n.code,{children:"~/.proto/bin"})," will now store symlinks to native binaries. To migrate to this new pattern, we're\nintroducing a new ",(0,s.jsx)(n.code,{children:"proto migrate"})," command (this only needs to be ran once)."]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-shell",children:"$ proto upgrade\n$ proto migrate v0.20 --log debug\n"})}),"\n",(0,s.jsx)(n.h3,{id:"how-it-works",children:"How it works"}),"\n",(0,s.jsxs)(n.p,{children:["When installing proto for the first time, or running the ",(0,s.jsx)(n.code,{children:"proto migrate"})," command, we prepend ",(0,s.jsx)(n.code,{children:"PATH"}),"\nwith ",(0,s.jsx)(n.code,{children:"$PROTO_HOME/shims:$PROTO_HOME/bin"}),". This allows shims to be executed ",(0,s.jsx)(n.em,{children:"first"})," and fallthrough\nto native binaries if a shim does not exist (for example, ",(0,s.jsx)(n.code,{children:".exe"})," on Windows)."]}),"\n",(0,s.jsxs)(n.p,{children:["Furthermore, if you'd prefer to ",(0,s.jsx)(n.em,{children:"only use"})," shims, or ",(0,s.jsx)(n.em,{children:"only use"})," binaries, you can update ",(0,s.jsx)(n.code,{children:"PATH"})," and\nremove the unwanted directory path."]}),"\n",(0,s.jsx)(n.p,{children:"And lastly, if shims are causing problems, you can now easily reference the native binaries\ndirectly. This was rather complicated before."}),"\n",(0,s.jsx)(n.h3,{id:"comparison",children:"Comparison"}),"\n",(0,s.jsxs)(n.table,{children:[(0,s.jsx)(n.thead,{children:(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.th,{}),(0,s.jsx)(n.th,{children:"Shims"}),(0,s.jsx)(n.th,{children:"Binaries"})]})}),(0,s.jsxs)(n.tbody,{children:[(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.td,{children:(0,s.jsx)(n.strong,{children:"Location"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"~/.proto/shims"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"~/.proto/bin"})})]}),(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.td,{children:(0,s.jsx)(n.strong,{children:"Created as"})}),(0,s.jsxs)(n.td,{children:["Scripts that run ",(0,s.jsx)(n.code,{children:"proto run"})]}),(0,s.jsx)(n.td,{children:"Symlinks to the native binary"})]}),(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.td,{children:(0,s.jsx)(n.strong,{children:"Version executed"})}),(0,s.jsx)(n.td,{children:"Detects version at runtime"}),(0,s.jsx)(n.td,{children:"Last version that was installed + pinned"})]}),(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.td,{children:(0,s.jsx)(n.strong,{children:"Supported for"})}),(0,s.jsx)(n.td,{children:"All tools"}),(0,s.jsxs)(n.td,{children:["Only tools that support native execution (may not work for ",(0,s.jsx)(n.code,{children:".js"})," files)"]})]}),(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.td,{children:(0,s.jsx)(n.strong,{children:"Additional files"})}),(0,s.jsxs)(n.td,{children:["Creates extra files (like ",(0,s.jsx)(n.code,{children:"bunx"}),", ",(0,s.jsx)(n.code,{children:"node-gyp"}),", etc)"]}),(0,s.jsx)(n.td,{children:"Only links the primary binary"})]})]})]}),"\n",(0,s.jsx)(n.h2,{id:"support-for-minisign-checksums",children:"Support for minisign checksums"}),"\n",(0,s.jsxs)(n.p,{children:["When proto installs a tool, it runs a process known as checksum verification, where we ensure the\ndownload hasn't been modified maliciously in anyway. Historically we only supported SHA256\nchecksums, but now, we also support the new ",(0,s.jsx)(n.a,{href:"https://jedisct1.github.io/minisign/",children:"minisign"})," tool,\nused by popular tools like ",(0,s.jsx)(n.a,{href:"https://ziglang.org/",children:"Zig"}),"."]}),"\n",(0,s.jsxs)(n.p,{children:["If you're building a plugin for a tool that uses minisign, you can use the new\n",(0,s.jsx)(n.a,{href:"/docs/proto/wasm-plugin#downloading-pre-builts",children:(0,s.jsx)(n.code,{children:"checksum_public_key"})})," (WASM) or\n",(0,s.jsx)(n.a,{href:"/docs/proto/toml-plugin#downloading-and-installing",children:(0,s.jsx)(n.code,{children:"install.checksum-public-key"})})," (TOML) field to\nprovide the public key for use in verification."]}),"\n",(0,s.jsxs)(n.p,{children:["When the checksum URL ends in a ",(0,s.jsx)(n.code,{children:".minisig"})," extension, proto will automatically use minisign for\nchecksum verification!"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-toml",children:'[install]\nchecksum-url = "https://domain.com/some/path/to/checksum.minisig"\nchecksum-public-key = "untrusted comment: ..."\n'})}),"\n",(0,s.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,s.jsxs)(n.p,{children:["View the ",(0,s.jsx)(n.a,{href:"https://github.com/moonrepo/proto/releases/tag/v0.20.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["Updated ",(0,s.jsx)(n.code,{children:"proto use"})," to install tools in parallel."]}),"\n",(0,s.jsxs)(n.li,{children:["Updated ",(0,s.jsx)(n.code,{children:"proto plugins"})," and ",(0,s.jsx)(n.code,{children:"proto tools"})," to load plugins in parallel."]}),"\n",(0,s.jsxs)(n.li,{children:["Updated ",(0,s.jsx)(n.code,{children:"proto run"})," to error when the tool attempts to self-upgrade outside of proto."]}),"\n",(0,s.jsxs)(n.li,{children:["Rust plugin","\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["Will now attempt to install ",(0,s.jsx)(n.code,{children:"rustup"})," if it does not exist on the current machine."]}),"\n",(0,s.jsxs)(n.li,{children:["Will now respect the ",(0,s.jsx)(n.code,{children:"RUSTUP_HOME"})," environment variable when locating the ",(0,s.jsx)(n.code,{children:".rustup"})," store."]}),"\n"]}),"\n"]}),"\n",(0,s.jsxs)(n.li,{children:["Schema plugin","\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["Added ",(0,s.jsx)(n.code,{children:"install.checksum_public_key"})," for defining the public key used to verify checksums."]}),"\n",(0,s.jsxs)(n.li,{children:["Added ",(0,s.jsx)(n.code,{children:"metadata.self_upgrade_commands"})," for defining which sub-commands should be blocked for\nself-upgrades."]}),"\n"]}),"\n"]}),"\n"]})]})}function h(e={}){const{wrapper:n}={...(0,o.a)(),...e.components};return n?(0,s.jsx)(n,{...e,children:(0,s.jsx)(c,{...e})}):c(e)}},71670:(e,n,i)=>{i.d(n,{Z:()=>l,a:()=>r});var s=i(27378);const o={},t=s.createContext(o);function r(e){const n=s.useContext(t);return s.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function l(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(o):e.components||o:r(e.components),s.createElement(t.Provider,{value:n},e.children)}}}]);