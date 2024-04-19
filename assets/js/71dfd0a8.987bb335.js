"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[23615],{83518:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>l,contentTitle:()=>i,default:()=>c,frontMatter:()=>s,metadata:()=>r,toc:()=>h});var a=t(24246),o=t(71670);const s={slug:"moon-v1.11",title:"moon v1.11 - Next-generation project graph",authors:["milesj"],tags:["project-graph","project"],image:"./img/moon/v1.11.png"},i=void 0,r={permalink:"/blog/moon-v1.11",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2023-07-31_moon-v1.11.mdx",source:"@site/blog/2023-07-31_moon-v1.11.mdx",title:"moon v1.11 - Next-generation project graph",description:"With this release, we've focused heavily on rewriting our project graph for the next-generation of",date:"2023-07-31T00:00:00.000Z",tags:[{label:"project-graph",permalink:"/blog/tags/project-graph"},{label:"project",permalink:"/blog/tags/project"}],readingTime:4.095,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"moon-v1.11",title:"moon v1.11 - Next-generation project graph",authors:["milesj"],tags:["project-graph","project"],image:"./img/moon/v1.11.png"},unlisted:!1,prevItem:{title:"proto v0.14 - Node.js and Rust now powered by WASM plugins",permalink:"/blog/proto-v0.14"},nextItem:{title:"proto v0.13 - Bun, Deno, and Go now powered by WASM plugins",permalink:"/blog/proto-v0.13"}},l={image:t(16725).Z,authorsImageUrls:[void 0]},h=[{value:"New project graph",id:"new-project-graph",level:2},{value:"Old implementation",id:"old-implementation",level:3},{value:"New implementation",id:"new-implementation",level:3},{value:"Unlocked features",id:"unlocked-features",level:3},{value:"Other changes",id:"other-changes",level:2}];function d(e){const n={a:"a",blockquote:"blockquote",code:"code",em:"em",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",strong:"strong",ul:"ul",...(0,o.a)(),...e.components};return(0,a.jsxs)(a.Fragment,{children:[(0,a.jsx)(n.p,{children:"With this release, we've focused heavily on rewriting our project graph for the next-generation of\nmoon."}),"\n",(0,a.jsx)(n.h2,{id:"new-project-graph",children:"New project graph"}),"\n",(0,a.jsx)(n.p,{children:"One of the first features that was built for moon was the project graph, as this was required to\ndetermine relationships between tasks and projects. Its initial implementation was rather simple, as\nit was a basic directed acyclic graph (DAG). However, as moon grew in complexity, so did the project\ngraph, and overtime, it has accrued a lot of cruft and technical debt."}),"\n",(0,a.jsxs)(n.p,{children:["One of the biggest pain points has been the project graph cache, and correctly invalidating the\ncache for all necessary scenarios. If you've been using moon for a long time, you're probably aware\nof all the hot fixes and patches that have been released. Another problem with the cache, is that it\nincluded hard-coded ",(0,a.jsx)(n.a,{href:"https://github.com/moonrepo/moon/issues/937",children:"file system paths"})," and\n",(0,a.jsx)(n.a,{href:"https://github.com/moonrepo/moon/issues/896",children:"environment variables"}),", both of which would not\ninvalidate the cache when changed."]}),"\n",(0,a.jsxs)(n.p,{children:["We felt it was time to rebuild the project graph from the ground up. Some of this work has already\nlanded in ",(0,a.jsx)(n.a,{href:"./moon-v1.9#rewritten-task-inheritance",children:"previous releases"}),"."]}),"\n",(0,a.jsx)(n.h3,{id:"old-implementation",children:"Old implementation"}),"\n",(0,a.jsx)(n.p,{children:"For those of you who are interested in the technical details, here's a quick overview of how the old\nproject graph worked. To start, the graph was composed around the following phases:"}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:[(0,a.jsx)(n.strong,{children:"Build"})," - Projects are loaded into the graph (nodes), relationships are linked (edges),\nconfigurations are read, tasks are inherited, and platform/language rules are applied."]}),"\n",(0,a.jsxs)(n.li,{children:[(0,a.jsx)(n.strong,{children:"Expand"})," - In all tasks, token variables and functions are expanded/substituted, dependencies\nare expanded (",(0,a.jsx)(n.code,{children:"^:deps"}),", etc), ",(0,a.jsx)(n.code,{children:".env"})," files are read (when applicable), so on and so forth."]}),"\n",(0,a.jsxs)(n.li,{children:[(0,a.jsx)(n.strong,{children:"Validate"})," - Enforces project and task boundaries and constraints."]}),"\n"]}),"\n",(0,a.jsxs)(n.p,{children:["This is quite a lot of work, and it was all done in ",(0,a.jsx)(n.em,{children:"a single pass"}),". What this means is that for\neach project loaded into the graph, we would recursively build -> expand -> validate, until all\nprojects have been loaded, synchronously at once in the same thread. Because this is a rather\nexpensive operation, the project graph cache was introduced to avoid having to do this work on every\nrun."]}),"\n",(0,a.jsx)(n.p,{children:"Makes sense, right? For the most part yes, but there is a core problem with the solution above, and\nif you've noticed it already, amazing! The problem is that out of these 3 phases, only the build\nphase is truly cacheable, as the expand and validate phases are far too dynamic and dependent on the\nenvironment. This means that the cache is only partially effective, and in some cases, entirely\nbroken."}),"\n",(0,a.jsx)(n.p,{children:"Another unrelated problem with this solution, is that because everything is built in a single pass,\nadvanced functionality that requires multiple passes is not possible and has been stuck on the\nbacklog."}),"\n",(0,a.jsx)(n.h3,{id:"new-implementation",children:"New implementation"}),"\n",(0,a.jsxs)(n.p,{children:["For backwards compatibility, the new project graph works in a similar manner, but has none of the\nshortcomings of the old implementation (hopefully). To start, the new project graph still has the\nsame 3 phases, but they are ",(0,a.jsx)(n.em,{children:"no longer processed in a single pass"}),", instead..."]}),"\n",(0,a.jsxs)(n.p,{children:["The build phase is now asynchronous, enabling deeper interoperability with the rest of the\nasync-aware codebase. However, the critical change is that the project graph cache is now written\n",(0,a.jsx)(n.em,{children:"after"})," the build phase (and read ",(0,a.jsx)(n.em,{children:"before"}),"), instead of after the entire graph being generated."]}),"\n",(0,a.jsxs)(n.blockquote,{children:["\n",(0,a.jsxs)(n.p,{children:["The new cache file is ",(0,a.jsx)(n.code,{children:".moon/cache/states/partialProjectGraph.json"}),", and is named partial because\ntasks have not been expanded. Use ",(0,a.jsx)(n.code,{children:"moon project-graph --json"})," for a fully expanded graph."]}),"\n"]}),"\n",(0,a.jsxs)(n.p,{children:["The expand phase has changed quite a bit. Instead of expanding everything at once, projects and\ntasks are only expanded when they are needed. For example, if only running a single target, we'll\nnow only expand that project and task, instead of ",(0,a.jsx)(n.em,{children:"everything"})," in the graph. With this change, you\nshould potentially see performance increases, unless you're using ",(0,a.jsx)(n.code,{children:"moon ci"})," or ",(0,a.jsx)(n.code,{children:"moon check --all"}),"."]}),"\n",(0,a.jsx)(n.p,{children:"And lastly, validation is still the same, but has been reworked so that we can easily extend it with\nmore validation rules in the future."}),"\n",(0,a.jsx)(n.h3,{id:"unlocked-features",children:"Unlocked features"}),"\n",(0,a.jsx)(n.p,{children:"With these changes to building and expanding, we've unlocked a few new features that were not\npossible before."}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsx)(n.li,{children:"Task dependencies can now reference tag based targets. For example, say we want to build all React\nprojects before starting our application."}),"\n"]}),"\n",(0,a.jsx)(n.pre,{children:(0,a.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  dev:\n    command: 'next dev'\n    deps:\n      - '#react:build'\n"})}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:["Task commands and arguments will now substitute environment variables, by first checking ",(0,a.jsx)(n.code,{children:"env"}),",\nthen those from the system."]}),"\n"]}),"\n",(0,a.jsx)(n.pre,{children:(0,a.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  build:\n    command: 'docker build --build-arg pkg=$PKG_NAME'\n  env:\n    PKG_NAME: 'foo-bar'\n"})}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:["Project dependencies can now mark relationships as ",(0,a.jsx)(n.code,{children:"build"}),". This is only applicable for languages\nthat support build dependencies, like Rust."]}),"\n"]}),"\n",(0,a.jsx)(n.pre,{children:(0,a.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"dependsOn:\n  - id: 'foo'\n    scope: 'build'\n"})}),"\n",(0,a.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,a.jsxs)(n.p,{children:["View the ",(0,a.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/v1.11.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:["Identifiers (project names, file groups, etc) can now be prefixed with underscores (",(0,a.jsx)(n.code,{children:"_"}),")."]}),"\n",(0,a.jsx)(n.li,{children:"Added Poetry detection support for Python projects."}),"\n",(0,a.jsxs)(n.li,{children:["Added an ",(0,a.jsx)(n.code,{children:"experiments"})," setting to ",(0,a.jsx)(n.code,{children:".moon/workspace.yml"}),"."]}),"\n"]})]})}function c(e={}){const{wrapper:n}={...(0,o.a)(),...e.components};return n?(0,a.jsx)(n,{...e,children:(0,a.jsx)(d,{...e})}):d(e)}},16725:(e,n,t)=>{t.d(n,{Z:()=>a});const a=t.p+"assets/images/v1.11-8730d5c4531586c014cef4253f41baa2.png"},71670:(e,n,t)=>{t.d(n,{Z:()=>r,a:()=>i});var a=t(27378);const o={},s=a.createContext(o);function i(e){const n=a.useContext(s);return a.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function r(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(o):e.components||o:i(e.components),a.createElement(s.Provider,{value:n},e.children)}}}]);