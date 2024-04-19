"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[25425],{65179:(e,n,o)=>{o.r(n),o.d(n,{assets:()=>l,contentTitle:()=>r,default:()=>h,frontMatter:()=>a,metadata:()=>s,toc:()=>c});var t=o(24246),i=o(71670);const a={slug:"moon-v1.15",title:"moon v1.15 - Next-generation action graph",authors:["milesj"],tags:["action","dependency","graph","pipeline","railway"],image:"./img/moon/v1.15.png"},r=void 0,s={permalink:"/blog/moon-v1.15",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2023-10-09_moon-v1.15.mdx",source:"@site/blog/2023-10-09_moon-v1.15.mdx",title:"moon v1.15 - Next-generation action graph",description:"In this release, we've taken the next step in modernizing our action pipeline, by rewriting the",date:"2023-10-09T00:00:00.000Z",tags:[{label:"action",permalink:"/blog/tags/action"},{label:"dependency",permalink:"/blog/tags/dependency"},{label:"graph",permalink:"/blog/tags/graph"},{label:"pipeline",permalink:"/blog/tags/pipeline"},{label:"railway",permalink:"/blog/tags/railway"}],readingTime:4.69,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"moon-v1.15",title:"moon v1.15 - Next-generation action graph",authors:["milesj"],tags:["action","dependency","graph","pipeline","railway"],image:"./img/moon/v1.15.png"},unlisted:!1,prevItem:{title:"proto v0.20 - New shims and binaries management",permalink:"/blog/proto-v0.20"},nextItem:{title:"proto v0.19 - Version pinning and outdated checks",permalink:"/blog/proto-v0.19"}},l={image:o(97966).Z,authorsImageUrls:[void 0]},c=[{value:"Hello action graph, goodbye dependency graph",id:"hello-action-graph-goodbye-dependency-graph",level:2},{value:"A new performant thread pool",id:"a-new-performant-thread-pool",level:3},{value:"Automatic dependency linking (breaking)",id:"automatic-dependency-linking-breaking",level:3},{value:"New <code>moonrepo/setup-toolchain</code> GitHub action",id:"new-moonreposetup-toolchain-github-action",level:2},{value:"Now supported in Railway",id:"now-supported-in-railway",level:2},{value:"Other changes",id:"other-changes",level:2}];function d(e){const n={a:"a",admonition:"admonition",blockquote:"blockquote",code:"code",em:"em",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,i.a)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsx)(n.p,{children:"In this release, we've taken the next step in modernizing our action pipeline, by rewriting the\ndependency graph."}),"\n",(0,t.jsx)(n.h2,{id:"hello-action-graph-goodbye-dependency-graph",children:"Hello action graph, goodbye dependency graph"}),"\n",(0,t.jsxs)(n.p,{children:["For the past few months, we've been working on a rewrite of our action pipeline, which consists of\nthe project graph, dependency graph, task executor, process pipeline, and more. It's a slow process,\nwith many different pieces that must land in sequence, but we're almost done. The next step in this\nprocess is the ",(0,t.jsx)(n.a,{href:"/docs/how-it-works/action-graph",children:"introduction of the new action graph"}),", which\nreplaces the previous dependency graph."]}),"\n",(0,t.jsx)(n.p,{children:"For the most part, the graphs work in a similar fashion, but since we rewrote it from the ground up,\nwe were able to resolve any discrepancies and performance issues. The biggest changes between the\nnew and old graphs are:"}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:["All actions now depend on the ",(0,t.jsx)(n.code,{children:"SyncWorkspace"})," action, instead of this action running arbitrarily."]}),"\n",(0,t.jsx)(n.li,{children:"Cleaned up dependency chains between actions, greatly reducing the number of nodes in the graph."}),"\n",(0,t.jsxs)(n.li,{children:["Renamed ",(0,t.jsx)(n.code,{children:"RunTarget"})," to ",(0,t.jsx)(n.code,{children:"RunTask"}),", including interactive and persistent variants."]}),"\n",(0,t.jsx)(n.li,{children:"And lastly, we ditched our batched task approach for a ready queue. Continue reading for more\ninformation!"}),"\n"]}),"\n",(0,t.jsx)(n.h3,{id:"a-new-performant-thread-pool",children:"A new performant thread pool"}),"\n",(0,t.jsx)(n.p,{children:"In the old dependency graph, when we'd execute a task, we'd order the graph topologically and then\ngroup actions into batches (or buckets) based on their dependency chains. Batches would then be\nexecuted in order within the thread pool. This approach worked well, but had one major flaw: it\nwasn't as performant as could be. For example, if our thread pool size was 12, and a batch only had\n2 tasks in it, what were the other 10 threads doing? Absolutely nothing. They were sitting idly,\nwaiting for a task."}),"\n",(0,t.jsx)(n.p,{children:"And now with the new action graph, we take full advantage of all threads in the pool. Instead of the\nbatched approach above, we now use a topological task-ready queue, where a thread without work (or\nis waiting for work) can poll the graph for a new task to run. A task is considered ready to run if\nit either has no dependencies, or all of its dependencies (in the chain) have been ran."}),"\n",(0,t.jsx)(n.p,{children:"For large graphs, this should result in a significant performance improvement!"}),"\n",(0,t.jsx)(n.h3,{id:"automatic-dependency-linking-breaking",children:"Automatic dependency linking (breaking)"}),"\n",(0,t.jsxs)(n.blockquote,{children:["\n",(0,t.jsx)(n.p,{children:'In v1.17, we changed the scope from "peer" to "build" to reduce friction.'}),"\n"]}),"\n",(0,t.jsxs)(n.p,{children:['Because of these graph changes, we do have a minor "breaking change". Tasks that depend (via ',(0,t.jsx)(n.code,{children:"deps"}),')\non other tasks from arbitrary projects (the parent project doesn\'t implicitly or explicitly depend\non the other project), not including the root-level project, will now automatically mark that other\nproject as a "peer" dependency (if not already configured with ',(0,t.jsx)(n.code,{children:"dependsOn"}),'). For example, "b"\nbecomes a peer dependency for "a".']}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-yaml",metastring:'title="a/moon.yml"',children:"tasks:\n  build:\n    deps: ['b:build']\n"})}),"\n",(0,t.jsx)(n.p,{children:"Now internally becomes:"}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-yaml",metastring:'title="a/moon.yml"',children:"dependsOn:\n  - id: 'b'\n    scope: 'peer'\n\ntasks:\n  build:\n    deps: ['b:build']\n"})}),"\n",(0,t.jsxs)(n.p,{children:["If you'd prefer this dependency to ",(0,t.jsx)(n.em,{children:"not be"}),' a peer, you can explicitly configure it with a different\nscope. For Node.js projects, the "build" scope can be used as a no-op replacement.']}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-yaml",metastring:'title="a/moon.yml"',children:"dependsOn:\n  - id: 'b'\n    scope: 'build' # production, development\n\ntasks:\n  build:\n    deps: ['b:build']\n"})}),"\n",(0,t.jsxs)(n.p,{children:["We're marking this as a breaking change as this could subtly introduce cycles in the project graph\nthat weren't present before, and for Node.js projects, this may inject ",(0,t.jsx)(n.code,{children:"peerDependencies"}),". However,\nthis change was necessary to ensure accurate dependency chains in the graph."]}),"\n",(0,t.jsxs)(n.h2,{id:"new-moonreposetup-toolchain-github-action",children:["New ",(0,t.jsx)(n.code,{children:"moonrepo/setup-toolchain"})," GitHub action"]}),"\n",(0,t.jsxs)(n.p,{children:["We've begun a process to deprecate the\n",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/setup-moon-action",children:"moonrepo/setup-moon-action"})," and\n",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/setup-proto",children:"moonrepo/setup-proto"})," GitHub actions, and instead, combine\nand replace them with a new ",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/setup-toolchain",children:"moonrepo/setup-toolchain"}),"\naction. Why a new action instead of fixing the others?"]}),"\n",(0,t.jsx)(n.p,{children:"The biggest problem was that both previous actions shared about 90% of the same code, but were\nslightly different in how they installed the binaries and cached the toolchain. It was was also\nconfusing for consumers to understand and know which action to use (because they shouldn't be used\ntogether)."}),"\n",(0,t.jsxs)(n.p,{children:["To remedy this, we're prototyping the new\n",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/setup-toolchain",children:"moonrepo/setup-toolchain"})," action, which has been\nworking quite well. It aims to solve the following:"]}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:["Installs ",(0,t.jsx)(n.code,{children:"proto"})," globally so that installed tools can also be executed globally."]}),"\n",(0,t.jsxs)(n.li,{children:["Conditionally installs ",(0,t.jsx)(n.code,{children:"moon"})," globally if the repository is using moon (attempts to detect a\n",(0,t.jsx)(n.code,{children:".moon"})," directory)."]}),"\n",(0,t.jsxs)(n.li,{children:["Caches the toolchain (",(0,t.jsx)(n.code,{children:"~/.proto"}),") so subsequent runs are faster."]}),"\n",(0,t.jsxs)(n.li,{children:["Hashes ",(0,t.jsx)(n.code,{children:".prototools"})," and ",(0,t.jsx)(n.code,{children:".moon/toolchain.yml"})," files to generate a unique cache key."]}),"\n",(0,t.jsx)(n.li,{children:"Cleans the toolchain before caching to remove unused or stale tools."}),"\n",(0,t.jsx)(n.li,{children:"Can auto-install tools when used."}),"\n"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-diff",children:"# ...\njobs:\n  ci:\n    name: CI\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n        with:\n          fetch-depth: 0\n-      - uses: moonrepo/setup-moon-action@v1\n+      - uses: moonrepo/setup-toolchain@v0\n"})}),"\n",(0,t.jsx)(n.h2,{id:"now-supported-in-railway",children:"Now supported in Railway"}),"\n",(0,t.jsxs)(n.p,{children:["If you're a big fan of ",(0,t.jsx)(n.a,{href:"https://railway.app/",children:"Railway"})," (like we are), and you're deploying a Node.js\nbacked application, then you'll be happy to hear that Railway now officially and natively supports\nmoon! We spent some time over the past month\n",(0,t.jsx)(n.a,{href:"https://nixpacks.com/docs/providers/node",children:"integrating moon support into their Nixpacks architecture"}),"."]}),"\n",(0,t.jsxs)(n.p,{children:["To make use of this, set the ",(0,t.jsx)(n.code,{children:"NIXPACKS_MOON_APP_NAME"})," environment variable to the name of your moon\nproject that you want to be deployed. This will then automatically run ",(0,t.jsx)(n.code,{children:"moon run <app>:build"})," and\n",(0,t.jsx)(n.code,{children:"moon run <app>:start"})," respectively. To customize the task names, you can set the\n",(0,t.jsx)(n.code,{children:"NIXPACKS_MOON_BUILD_TASK"})," and ",(0,t.jsx)(n.code,{children:"NIXPACKS_MOON_START_TASK"})," environment variables."]}),"\n",(0,t.jsx)(n.admonition,{type:"info",children:(0,t.jsx)(n.p,{children:"This is currently only supported for Node.js projects, but will be expanded to other languages in\nthe future!"})}),"\n",(0,t.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,t.jsxs)(n.p,{children:["View the ",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/v1.15.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:["Added a ",(0,t.jsx)(n.code,{children:"moon action-graph"})," command."]}),"\n",(0,t.jsxs)(n.li,{children:["Added a ",(0,t.jsx)(n.code,{children:"--dependents"})," argument to ",(0,t.jsx)(n.code,{children:"moon action-graph"}),"."]}),"\n",(0,t.jsxs)(n.li,{children:["Added the ability to skip non-",(0,t.jsx)(n.code,{children:"RunTask"})," actions using environment variables."]}),"\n",(0,t.jsxs)(n.li,{children:["Deprecated the ",(0,t.jsx)(n.code,{children:"moon dep-graph"})," command."]}),"\n"]})]})}function h(e={}){const{wrapper:n}={...(0,i.a)(),...e.components};return n?(0,t.jsx)(n,{...e,children:(0,t.jsx)(d,{...e})}):d(e)}},97966:(e,n,o)=>{o.d(n,{Z:()=>t});const t=o.p+"assets/images/v1.15-24f6509aa7bcbfaf9ac48f4d63883483.png"},71670:(e,n,o)=>{o.d(n,{Z:()=>s,a:()=>r});var t=o(27378);const i={},a=t.createContext(i);function r(e){const n=t.useContext(a);return t.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function s(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(i):e.components||i:r(e.components),t.createElement(a.Provider,{value:n},e.children)}}}]);