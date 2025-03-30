"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[69124],{25700:(e,n,t)=>{t.d(n,{A:()=>i});const i=t.p+"assets/images/v1.34-4f939703fe9212cf786df99c29a71feb.png"},43023:(e,n,t)=>{t.d(n,{R:()=>a,x:()=>r});var i=t(63696);const o={},s=i.createContext(o);function a(e){const n=i.useContext(s);return i.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function r(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(o):e.components||o:a(e.components),i.createElement(s.Provider,{value:n},e.children)}},43715:e=>{e.exports=JSON.parse('{"permalink":"/blog/moon-v1.34","editUrl":"https://github.com/moonrepo/moon/tree/master/website/blog/2025-03-31_moon-v1.34.mdx","source":"@site/blog/2025-03-31_moon-v1.34.mdx","title":"moon v1.34 - Action customization, faster globs, better Git, and more!","description":"With this release, we\'re introducing a handful of performance and customization improvements!","date":"2025-03-31T00:00:00.000Z","tags":[{"inline":true,"label":"moonbase","permalink":"/blog/tags/moonbase"},{"inline":true,"label":"remote","permalink":"/blog/tags/remote"},{"inline":true,"label":"cache","permalink":"/blog/tags/cache"},{"inline":true,"label":"action","permalink":"/blog/tags/action"},{"inline":true,"label":"pipeline","permalink":"/blog/tags/pipeline"},{"inline":true,"label":"glob","permalink":"/blog/tags/glob"},{"inline":true,"label":"git","permalink":"/blog/tags/git"},{"inline":true,"label":"experiment","permalink":"/blog/tags/experiment"}],"readingTime":4.945,"hasTruncateMarker":true,"authors":[{"name":"Miles Johnson","title":"Founder, developer","url":"https://github.com/milesj","imageURL":"/img/authors/miles.jpg","key":"milesj","page":null}],"frontMatter":{"slug":"moon-v1.34","title":"moon v1.34 - Action customization, faster globs, better Git, and more!","authors":["milesj"],"tags":["moonbase","remote","cache","action","pipeline","glob","git","experiment"],"image":"./img/moon/v1.34.png"},"unlisted":false,"nextItem":{"title":"moon v1.33 - Alpha support for toolchain WASM plugins","permalink":"/blog/moon-v1.33"}}')},59183:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>l,contentTitle:()=>r,default:()=>d,frontMatter:()=>a,metadata:()=>i,toc:()=>c});var i=t(43715),o=t(62540),s=t(43023);const a={slug:"moon-v1.34",title:"moon v1.34 - Action customization, faster globs, better Git, and more!",authors:["milesj"],tags:["moonbase","remote","cache","action","pipeline","glob","git","experiment"],image:"./img/moon/v1.34.png"},r=void 0,l={image:t(25700).A,authorsImageUrls:[void 0]},c=[{value:"moonbase has been sunset",id:"moonbase-has-been-sunset",level:2},{value:"Customize actions in the pipeline",id:"customize-actions-in-the-pipeline",level:2},{value:"New <code>--no-actions</code> flag",id:"new---no-actions-flag",level:3},{value:"New experiments",id:"new-experiments",level:2},{value:"Faster glob walking",id:"faster-glob-walking",level:3},{value:"Better Git integration",id:"better-git-integration",level:3},{value:"Other changes",id:"other-changes",level:2},{value:"What&#39;s next?",id:"whats-next",level:2}];function h(e){const n={a:"a",admonition:"admonition",code:"code",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.R)(),...e.components};return(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(n.p,{children:"With this release, we're introducing a handful of performance and customization improvements!"}),"\n",(0,o.jsx)(n.h2,{id:"moonbase-has-been-sunset",children:"moonbase has been sunset"}),"\n",(0,o.jsxs)(n.p,{children:["As mentioned in previous releases, we have sunset ",(0,o.jsx)(n.a,{href:"/moonbase",children:"moonbase"}),", our remote caching service.\nA while back we had internal discussions on whether to rework moonbase so that it could be\nself-hosted (on-premises), or adopt the\n",(0,o.jsx)(n.a,{href:"https://github.com/bazelbuild/remote-apis",children:"Bazel Remote Execution API"})," and utilize what the open\nsource community has to offer. We ultimately decided with the latter, as it frees up resources on\nour end to focus on moon and proto, and also provides a better path forward for moon adoption."]}),"\n",(0,o.jsxs)(n.p,{children:["If you are currently using moonbase, we suggest migrating to our new\n",(0,o.jsx)(n.a,{href:"/docs/guides/remote-cache",children:"remote caching options"}),". And if you have an active moonbase\nsubscription, it will be cancelled within the week, and any partial billing for this month will be\nreimbursed."]}),"\n",(0,o.jsx)(n.h2,{id:"customize-actions-in-the-pipeline",children:"Customize actions in the pipeline"}),"\n",(0,o.jsxs)(n.p,{children:["When a task is ran in moon, we create an ",(0,o.jsx)(n.a,{href:"/docs/how-it-works/action-graph",children:"action graph"})," (not a task\ngraph) of actions, which are operations required to run to ensure a successful execution. This\nincludes non-task related functionality, like project and workspace syncing, installing\ndependencies, and ensuring the toolchain has been setup. While these actions only take milliseconds\nto execute (on a cache hit), they can become quite a barrier and source of friction (on cache miss)."]}),"\n",(0,o.jsxs)(n.p,{children:["Until now, there wasn't anyway to disable/skip these extra actions -- besides some non-documented\nenvironment variables that didn't actually omit the actions (they were still in the graph), but\nsimply skipped the inner execution. In this release, we're introducing 4 new settings for\n",(0,o.jsx)(n.a,{href:"/docs/config/workspace#pipeline",children:(0,o.jsx)(n.code,{children:"pipeline"})})," (formerly ",(0,o.jsx)(n.code,{children:"runner"}),") in\n",(0,o.jsx)(n.a,{href:"/docs/config/workspace",children:(0,o.jsx)(n.code,{children:".moon/workspace.yml"})}),"."]}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"installDependencies"})," setting toggles the inclusion of the ",(0,o.jsx)(n.code,{children:"InstallWorkspaceDeps"})," and\n",(0,o.jsx)(n.code,{children:"InstallProjectDeps"})," actions, and can be scoped to toolchain IDs."]}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"syncProjects"})," setting toggles the inclusion of the ",(0,o.jsx)(n.code,{children:"SyncProject"})," actions, and can be scoped to\nproject IDs."]}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"syncProjectDependencies"})," setting toggles whether to recursively create ",(0,o.jsx)(n.code,{children:"SyncProject"})," actions for\neach dependency of a project, or just for itself."]}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"syncWorkspace"})," setting toggles the inclusion of the root ",(0,o.jsx)(n.code,{children:"SyncWorkspace"})," action."]}),"\n"]}),"\n",(0,o.jsx)(n.p,{children:"For example, if you want to disable all of these actions entirely, you can do this:"}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/workspace.yml"',children:"pipeline:\n  installDependencies: false\n  syncProjects: false\n  syncProjectDependencies: false\n  syncWorkspace: false\n"})}),"\n",(0,o.jsxs)(n.p,{children:["And as mentioned above, the ",(0,o.jsx)(n.code,{children:"installDependencies"})," and ",(0,o.jsx)(n.code,{children:"syncProjects"})," settings support configuring a\nlist of IDs, which acts as an allow list. Any IDs not listed here will not create actions."]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/workspace.yml"',children:"pipeline:\n  # Only install Node.js dependencies\n  installDependencies: ['node']\n  # Only sync the `app` project\n  syncProjects: ['app']\n"})}),"\n",(0,o.jsx)(n.admonition,{type:"info",children:(0,o.jsxs)(n.p,{children:["Even if you disable actions with the ",(0,o.jsx)(n.code,{children:"pipeline"})," setting, the ",(0,o.jsx)(n.a,{href:"/docs/commands/sync",children:(0,o.jsx)(n.code,{children:"moon sync"})}),"\ncommands can still be used to run sync operations, as they ignore that setting. This provides a\nsolution where you want to avoid the overhead when running a task, but still take advantage of\nmoon's syncing to ensure a healthy repository state."]})}),"\n",(0,o.jsxs)(n.h3,{id:"new---no-actions-flag",children:["New ",(0,o.jsx)(n.code,{children:"--no-actions"})," flag"]}),"\n",(0,o.jsxs)(n.p,{children:["To expand upon the above, we're introducing a ",(0,o.jsx)(n.code,{children:"--no-actions"})," flag to\n",(0,o.jsx)(n.a,{href:"/docs/commands/run",children:(0,o.jsx)(n.code,{children:"moon run"})}),", that will run the task without the other actions being added to\nthe graph. We suggest only using this flag once dependencies have been installed, and the toolchain\nhas been setup!"]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-shell",children:"$ moon run app:start --no-actions\n"})}),"\n",(0,o.jsx)(n.h2,{id:"new-experiments",children:"New experiments"}),"\n",(0,o.jsx)(n.p,{children:"It's been a while since we've added new experiments, and in this release, we've got 2! We encourage\neveryone to enable these experiments to ensure they are working correctly, but do note that these\nare a work in progress and may be buggy."}),"\n",(0,o.jsx)(n.h3,{id:"faster-glob-walking",children:"Faster glob walking"}),"\n",(0,o.jsx)(n.p,{children:"We've been monitoring glob performance for sometime now, as walking the filesystem has been one of\nthe largest bottlenecks, especially in large codebases. We felt it was about time to tackle the\nproblem."}),"\n",(0,o.jsx)(n.p,{children:"With this new implementation, we are doing a few things to increase performance. To start, we are\nparallelizing walking per directory, where previously this would happen serially. Next, we partition\nglobs based on a common ancestor directory, which reduces the amount of unnecessary walking. And\nlastly, we cache common globs to avoid walking all together."}),"\n",(0,o.jsxs)(n.p,{children:["In our benchmarks and tests (moon itself is already using it), we are seeing performance increases\nby 1.5-2x! To start using this new glob implementation, enable the new ",(0,o.jsx)(n.code,{children:"fasterGlobWalk"})," experiment."]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/workspace.yml"',children:"experiments:\n  fasterGlobWalk: true\n"})}),"\n",(0,o.jsx)(n.h3,{id:"better-git-integration",children:"Better Git integration"}),"\n",(0,o.jsx)(n.p,{children:"Our current Git integration works, assuming you're not doing anything complex, like using submodules\nor worktrees. If you are using the latter, things have been buggy. We're not happy about this, as we\nwant to support all the different ways a repository can be architected."}),"\n",(0,o.jsxs)(n.p,{children:["So we started over from scratch! We even created\n",(0,o.jsx)(n.a,{href:"https://github.com/moonrepo/git-test",children:"real repositories"})," to ensure our understanding and\nimplementation of these features is accurate. This new implementation achieves the following:"]}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:["Supports submodules, subtrees, and worktees (unique among build systems).","\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsx)(n.li,{children:'Our competitors don\'t support these, and we expect them to "borrow" our implementation in the\nfuture (like they have with other features).'}),"\n"]}),"\n"]}),"\n",(0,o.jsx)(n.li,{children:"Git commands are parallelized when applicable."}),"\n",(0,o.jsx)(n.li,{children:"Touched files within submodules are now properly extracted."}),"\n",(0,o.jsx)(n.li,{children:"File discovery and hashing is more performant."}),"\n"]}),"\n",(0,o.jsxs)(n.p,{children:["If you'd like to try this new Git implementation (moon itself already is), enable the ",(0,o.jsx)(n.code,{children:"gitV2"}),"\nexperiment."]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/workspace.yml"',children:"experiments:\n  gitV2: true\n"})}),"\n",(0,o.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,o.jsxs)(n.p,{children:["View the ",(0,o.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/v1.34.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:["Added a ",(0,o.jsx)(n.code,{children:"--json"})," flag to ",(0,o.jsx)(n.code,{children:"moon templates"}),"."]}),"\n",(0,o.jsx)(n.li,{children:"Integrated a new console rendering system with new terminal styles, prompts, and output."}),"\n",(0,o.jsx)(n.li,{children:"Improved the performance of environment variable substitution."}),"\n",(0,o.jsx)(n.li,{children:"Improved toolchain plugin loading to be on-demand."}),"\n",(0,o.jsx)(n.li,{children:"Improved sync cache invalidation for codeowners, config schemas, and VCS hooks."}),"\n"]}),"\n",(0,o.jsx)(n.h2,{id:"whats-next",children:"What's next?"}),"\n",(0,o.jsx)(n.p,{children:"Going forward, we plan to release new updates on a bi-weekly schedule, instead of a monthly\nschedule. This will result in less features each release, but will reduce the burden and complexity\nof large releases. With that said, this is what we have tentatively planned for the next release!"}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsx)(n.li,{children:"Migrate the Rust toolchain to a WASM plugin."}),"\n",(0,o.jsx)(n.li,{children:"Investigate a new args/command line parser."}),"\n",(0,o.jsx)(n.li,{children:"Add Poetry support for the Python toolchain."}),"\n"]})]})}function d(e={}){const{wrapper:n}={...(0,s.R)(),...e.components};return n?(0,o.jsx)(n,{...e,children:(0,o.jsx)(h,{...e})}):h(e)}}}]);