"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[45230],{36857:(e,o,n)=>{n.r(o),n.d(o,{assets:()=>a,contentTitle:()=>s,default:()=>h,frontMatter:()=>i,metadata:()=>l,toc:()=>c});var t=n(24246),r=n(71670);const i={slug:"proto-v0.36",title:"proto v0.36 - Static registry, plugin searching, and more",authors:["milesj"],tags:["registry"]},s=void 0,l={permalink:"/blog/proto-v0.36",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2024-06-03_proto-v0.36.mdx",source:"@site/blog/2024-06-03_proto-v0.36.mdx",title:"proto v0.36 - Static registry, plugin searching, and more",description:"In this release, we're taking the first step in supporting a plugin registry.",date:"2024-06-03T00:00:00.000Z",tags:[{label:"registry",permalink:"/blog/tags/registry"}],readingTime:1.995,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"proto-v0.36",title:"proto v0.36 - Static registry, plugin searching, and more",authors:["milesj"],tags:["registry"]},unlisted:!1,prevItem:{title:"proto v0.37 - Calver support and self diagnosis",permalink:"/blog/proto-v0.37"},nextItem:{title:"moon v1.25 - New task runner and console reporter",permalink:"/blog/moon-v1.25"}},a={authorsImageUrls:[void 0]},c=[{value:"New static registry",id:"new-static-registry",level:2},{value:"New <code>proto plugin search</code> command",id:"new-proto-plugin-search-command",level:2},{value:"New <code>proto unpin</code> command",id:"new-proto-unpin-command",level:2},{value:"Plugin locator syntax changes",id:"plugin-locator-syntax-changes",level:2},{value:"Other changes",id:"other-changes",level:2}];function d(e){const o={a:"a",blockquote:"blockquote",code:"code",em:"em",h2:"h2",li:"li",p:"p",pre:"pre",ul:"ul",...(0,r.a)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsx)(o.p,{children:"In this release, we're taking the first step in supporting a plugin registry."}),"\n",(0,t.jsx)(o.h2,{id:"new-static-registry",children:"New static registry"}),"\n",(0,t.jsxs)(o.p,{children:["Our long-term plan for proto is to provide a server-based registry in which users could publish and\nmanage plugins, and immediately make them available to the community. However, this is quite a\nmountain of work, and will take some time, but making plugins available ",(0,t.jsx)(o.em,{children:"now"})," is a priority."]}),"\n",(0,t.jsxs)(o.p,{children:["As a temporary solution, we're introducing a static registry, in which available plugins are defined\nin static JSON files, located in the official\n",(0,t.jsx)(o.a,{href:"https://github.com/moonrepo/proto/tree/master/registry",children:"moonrepo/proto"})," repository. This will help\nunblock new features moving forward."]}),"\n",(0,t.jsxs)(o.h2,{id:"new-proto-plugin-search-command",children:["New ",(0,t.jsx)(o.code,{children:"proto plugin search"})," command"]}),"\n",(0,t.jsxs)(o.p,{children:["Because of the static registry work above, we're now able to introduce a new command,\n",(0,t.jsx)(o.a,{href:"/docs/proto/commands/plugin/search",children:(0,t.jsx)(o.code,{children:"proto plugin search"})}),", that can be used to search for plugins\nprovided by the community. No longer will you need to browse the documentation, or search Google for\navailable plugins."]}),"\n",(0,t.jsx)(o.pre,{children:(0,t.jsx)(o.code,{children:"$ proto plugin search moon\n\nPlugins\nAvailable for query: moon\n\n Plugin  Author    Format  Description                                                          Locator\n moon    moonrepo  TOML    moon is a multi-language build system and codebase management tool.  https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml\n"})}),"\n",(0,t.jsxs)(o.h2,{id:"new-proto-unpin-command",children:["New ",(0,t.jsx)(o.code,{children:"proto unpin"})," command"]}),"\n",(0,t.jsxs)(o.p,{children:["Additionally, a command that probably should have existed from the start, but did not, is now\navailable. The ",(0,t.jsx)(o.a,{href:"/docs/proto/commands/unpin",children:(0,t.jsx)(o.code,{children:"proto unpin"})})," command does exactly as its name says, it\nunpins (removes) a version from a ",(0,t.jsx)(o.code,{children:".prototools"})," file."]}),"\n",(0,t.jsx)(o.h2,{id:"plugin-locator-syntax-changes",children:"Plugin locator syntax changes"}),"\n",(0,t.jsxs)(o.p,{children:["We've decided to slightly change the syntax of plugin locator strings by embracing the common\nprotocol syntax. Instead of ",(0,t.jsx)(o.code,{children:"source:"})," and ",(0,t.jsx)(o.code,{children:"github:"}),", we now use ",(0,t.jsx)(o.code,{children:"file://"}),", ",(0,t.jsx)(o.code,{children:"https://"}),", and\n",(0,t.jsx)(o.code,{children:"github://"}),". The former syntax will continue to work for the time being, but will be removed\nentirely in the future."]}),"\n",(0,t.jsxs)(o.ul,{children:["\n",(0,t.jsxs)(o.li,{children:[(0,t.jsx)(o.code,{children:"source:./file.wasm"})," -> ",(0,t.jsx)(o.code,{children:"file://./file.wasm"})]}),"\n",(0,t.jsxs)(o.li,{children:[(0,t.jsx)(o.code,{children:"source:https://url.com/file.wasm"})," -> ",(0,t.jsx)(o.code,{children:"https://url.com/file.wasm"})]}),"\n",(0,t.jsxs)(o.li,{children:[(0,t.jsx)(o.code,{children:"github:org/repo"})," -> ",(0,t.jsx)(o.code,{children:"github://org/repo"})]}),"\n"]}),"\n",(0,t.jsxs)(o.blockquote,{children:["\n",(0,t.jsxs)(o.p,{children:["If a ",(0,t.jsx)(o.code,{children:"proto"})," command modifies a ",(0,t.jsx)(o.code,{children:".prototools"})," file, the file will be saved with the new syntax.\nDon't be surprised when this happens!"]}),"\n"]}),"\n",(0,t.jsx)(o.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,t.jsxs)(o.p,{children:["View the ",(0,t.jsx)(o.a,{href:"https://github.com/moonrepo/proto/releases/tag/v0.36.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,t.jsxs)(o.ul,{children:["\n",(0,t.jsxs)(o.li,{children:["Updated ",(0,t.jsx)(o.code,{children:"proto uninstall"})," to also remove entries from ",(0,t.jsx)(o.code,{children:".prototools"})," if the version was\nuninstalled."]}),"\n",(0,t.jsx)(o.li,{children:"Updated some error messages to include copy for work arounds."}),"\n",(0,t.jsxs)(o.li,{children:["We now lock the bin/shims directory when creating/removing files.","\n",(0,t.jsxs)(o.ul,{children:["\n",(0,t.jsx)(o.li,{children:"This is an experiment to help avoid race conditions where multiple proto processes are all\ntrying to write to the same location."}),"\n",(0,t.jsx)(o.li,{children:"If this results in too large of a performance hit, we'll remove the locking."}),"\n"]}),"\n"]}),"\n"]})]})}function h(e={}){const{wrapper:o}={...(0,r.a)(),...e.components};return o?(0,t.jsx)(o,{...e,children:(0,t.jsx)(d,{...e})}):d(e)}},71670:(e,o,n)=>{n.d(o,{Z:()=>l,a:()=>s});var t=n(27378);const r={},i=t.createContext(r);function s(e){const o=t.useContext(i);return t.useMemo((function(){return"function"==typeof e?e(o):{...o,...e}}),[o,e])}function l(e){let o;return o=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:s(e.components),t.createElement(i.Provider,{value:o},e.children)}}}]);