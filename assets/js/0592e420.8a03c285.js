"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[36474],{28235:(e,n,t)=>{t.d(n,{A:()=>o});const o=t.p+"assets/images/v1.20-7d4631df55aca0cc0a529ac2f5b55ca3.png"},43023:(e,n,t)=>{t.d(n,{R:()=>a,x:()=>r});var o=t(63696);const s={},i=o.createContext(s);function a(e){const n=o.useContext(i);return o.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function r(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:a(e.components),o.createElement(i.Provider,{value:n},e.children)}},71106:e=>{e.exports=JSON.parse('{"permalink":"/blog/moon-v1.20","editUrl":"https://github.com/moonrepo/moon/tree/master/website/blog/2024-01-26_moon-v1.20.mdx","source":"@site/blog/2024-01-26_moon-v1.20.mdx","title":"moon v1.20 - Extension plugins, default task options, and more","description":"In this release, we\'re excited to introduce extensions, our first type of plugin!","date":"2024-01-26T00:00:00.000Z","tags":[{"inline":true,"label":"task","permalink":"/blog/tags/task"},{"inline":true,"label":"options","permalink":"/blog/tags/options"},{"inline":true,"label":"extensions","permalink":"/blog/tags/extensions"},{"inline":true,"label":"plugins","permalink":"/blog/tags/plugins"}],"readingTime":2.685,"hasTruncateMarker":true,"authors":[{"name":"Miles Johnson","title":"Founder, developer","url":"https://github.com/milesj","imageURL":"/img/authors/miles.jpg","key":"milesj","page":null}],"frontMatter":{"slug":"moon-v1.20","title":"moon v1.20 - Extension plugins, default task options, and more","authors":["milesj"],"tags":["task","options","extensions","plugins"],"image":"./img/moon/v1.20.png"},"unlisted":false,"prevItem":{"title":"moon v1.21 - Deno tier 3, file group improvements, task shells, and more!","permalink":"/blog/moon-v1.21"},"nextItem":{"title":"proto v0.29 - Better environment support","permalink":"/blog/proto-v0.29"}}')},81001:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>l,contentTitle:()=>r,default:()=>h,frontMatter:()=>a,metadata:()=>o,toc:()=>d});var o=t(71106),s=t(62540),i=t(43023);const a={slug:"moon-v1.20",title:"moon v1.20 - Extension plugins, default task options, and more",authors:["milesj"],tags:["task","options","extensions","plugins"],image:"./img/moon/v1.20.png"},r=void 0,l={image:t(28235).A,authorsImageUrls:[void 0]},d=[{value:"New extension plugins",id:"new-extension-plugins",level:2},{value:"Configure default options for tasks",id:"configure-default-options-for-tasks",level:2},{value:"Optional task dependencies",id:"optional-task-dependencies",level:2},{value:"Other changes",id:"other-changes",level:2}];function c(e){const n={a:"a",blockquote:"blockquote",code:"code",em:"em",h2:"h2",li:"li",p:"p",pre:"pre",ul:"ul",...(0,i.R)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsx)(n.p,{children:"In this release, we're excited to introduce extensions, our first type of plugin!"}),"\n",(0,s.jsx)(n.h2,{id:"new-extension-plugins",children:"New extension plugins"}),"\n",(0,s.jsxs)(n.p,{children:["In our ",(0,s.jsx)(n.a,{href:"./2024-roadmap",children:"2024 roadmap blog post"}),", we talked heavily about plugins, as we believe\nthey're the future of moon and its ecosystem. What we didn't talk about is that we plan to have\n",(0,s.jsx)(n.em,{children:"many types of plugins"}),", and not just language/platform specific ones. And with that, we're excited\nto introduce extensions!"]}),"\n",(0,s.jsx)(n.p,{children:"An extension is a WASM plugin that allows you to extend moon with additional functionality, have\nwhitelisted access to the file system, and receive partial information about the current workspace.\nExtensions are extremely useful in offering new and unique functionality that doesn't need to be\nbuilt into moon's core."}),"\n",(0,s.jsxs)(n.p,{children:["Once such extension is our built-in ",(0,s.jsx)(n.code,{children:"download"})," extension, which is a basic extension that simply\ndownloads a file from a URL into the current moon workspace."]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-shell",children:"$ moon ext download -- --url https://github.com/moonrepo/proto/releases/latest/download/proto_cli-aarch64-apple-darwin.tar.xz\n"})}),"\n",(0,s.jsx)(n.p,{children:"Shipping alongside extensions are the following new features:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["An ",(0,s.jsx)(n.a,{href:"/docs/guides/extensions",children:"official extensions guide"}),"!"]}),"\n",(0,s.jsxs)(n.li,{children:["An ",(0,s.jsx)(n.a,{href:"/docs/config/workspace#extensions",children:(0,s.jsx)(n.code,{children:"extensions"})})," setting for configuring new extensions."]}),"\n",(0,s.jsxs)(n.li,{children:["A ",(0,s.jsx)(n.a,{href:"/docs/commands/ext",children:(0,s.jsx)(n.code,{children:"moon ext"})})," command for executing a configured extension."]}),"\n",(0,s.jsx)(n.li,{children:"The required infrastructure for plugins!"}),"\n"]}),"\n",(0,s.jsx)(n.h2,{id:"configure-default-options-for-tasks",children:"Configure default options for tasks"}),"\n",(0,s.jsxs)(n.p,{children:[(0,s.jsx)(n.a,{href:"/docs/config/project#options",children:"Task options"})," provide a way to apply granular changes to a task's\nbehavior when running in the pipeline. However, they can become tedious when you need to apply them\nto many tasks, especially when inheritance is involved. To help with this, you can now configure the\n",(0,s.jsx)(n.a,{href:"/docs/config/tasks#taskoptions",children:(0,s.jsx)(n.code,{children:"taskOptions"})})," setting in task related configs, like\n",(0,s.jsx)(n.code,{children:".moon/tasks.yml"})," and ",(0,s.jsx)(n.code,{children:".moon/tasks/*.yml"}),", which acts as the base/default options for all inherited\ntasks."]}),"\n",(0,s.jsx)(n.p,{children:"For example, the following config:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks.yml"',children:"tasks:\n  build:\n    # ...\n    options:\n      outputStyle: 'stream'\n      retryCount: 2\n  lint:\n    # ...\n    options:\n      outputStyle: 'stream'\n      retryCount: 2\n  test:\n    # ...\n    options:\n      outputStyle: 'stream'\n      retryCount: 2\n"})}),"\n",(0,s.jsx)(n.p,{children:"Can simply be rewritten as:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks.yml"',children:"taskOptions:\n  outputStyle: 'stream'\n  retryCount: 2\n\ntasks:\n  build:\n    # ...\n  lint:\n    # ...\n  test:\n    # ...\n"})}),"\n",(0,s.jsx)(n.p,{children:"Because these options are defined at the workspace-level, they adhere to the same merge and\ninheritance rules as other settings. Just be aware that these options are inherited first in the\nchain, and can be overwritten by other layers, or by project-level tasks."}),"\n",(0,s.jsx)(n.h2,{id:"optional-task-dependencies",children:"Optional task dependencies"}),"\n",(0,s.jsxs)(n.p,{children:["By default, all task ",(0,s.jsx)(n.a,{href:"/docs/config/project#tasks-1",children:(0,s.jsx)(n.code,{children:"deps"})})," are required to exist when tasks are\nbeing built and expanded, but this isn't always true when dealing with composition and inheritance.\nFor example, say you're using\n",(0,s.jsx)(n.a,{href:"/docs/concepts/task-inheritance#scope-by-project-metadata",children:"tag-based inheritance"}),", and a global\ntask relies on one of these tagged tasks, but not all projects may define the appropriate tags. In\nprevious versions of moon, this is a hard failure, as the dependent task does not exist."]}),"\n",(0,s.jsxs)(n.p,{children:["To remedy this, we're introducing a new ",(0,s.jsx)(n.a,{href:"/docs/config/project#optional-1",children:(0,s.jsx)(n.code,{children:"optional"})})," flag for task\ndependencies. When set to ",(0,s.jsx)(n.code,{children:"true"}),", moon will no longer error when the task doesn't exist, and instead\nwill omit the dependency."]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks.yml" {4-6}',children:"tasks:\n  build:\n    command: 'vite'\n    deps:\n      - target: '#components:build'\n        optional: true\n"})}),"\n",(0,s.jsxs)(n.blockquote,{children:["\n",(0,s.jsxs)(n.p,{children:["Thanks to ",(0,s.jsx)(n.a,{href:"https://github.com/maastrich",children:"@maastrich"})," for this contribution!"]}),"\n"]}),"\n",(0,s.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,s.jsxs)(n.p,{children:["View the ",(0,s.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/v1.20.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsx)(n.li,{children:'Added a "Tags" view to the VSCode extension.'}),"\n",(0,s.jsx)(n.li,{children:"Updated proto to v0.29.1 (from v0.26.4)."}),"\n",(0,s.jsxs)(n.li,{children:["Updated proto installation to trigger for all applicable commands, not just ",(0,s.jsx)(n.code,{children:"moon run"}),",\n",(0,s.jsx)(n.code,{children:"moon check"}),", and ",(0,s.jsx)(n.code,{children:"moon ci"}),"."]}),"\n"]})]})}function h(e={}){const{wrapper:n}={...(0,i.R)(),...e.components};return n?(0,s.jsx)(n,{...e,children:(0,s.jsx)(c,{...e})}):c(e)}}}]);