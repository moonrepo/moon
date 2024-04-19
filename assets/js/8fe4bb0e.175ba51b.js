"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[11312],{24220:(e,n,o)=>{o.r(n),o.d(n,{assets:()=>l,contentTitle:()=>a,default:()=>h,frontMatter:()=>i,metadata:()=>r,toc:()=>c});var t=o(24246),s=o(71670);const i={slug:"moon-v1.18",title:"moon v1.18 - New task execution flow and custom project names",authors:["milesj"],tags:["toolchain","shell","id","name","project","init","onboarding"],image:"./img/moon/v1.18.png"},a=void 0,r={permalink:"/blog/moon-v1.18",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2023-12-12_moon-v1.18.mdx",source:"@site/blog/2023-12-12_moon-v1.18.mdx",title:"moon v1.18 - New task execution flow and custom project names",description:"With this release, we've focused heavily on 2 important aspects: task execution, and our onboarding",date:"2023-12-12T00:00:00.000Z",tags:[{label:"toolchain",permalink:"/blog/tags/toolchain"},{label:"shell",permalink:"/blog/tags/shell"},{label:"id",permalink:"/blog/tags/id"},{label:"name",permalink:"/blog/tags/name"},{label:"project",permalink:"/blog/tags/project"},{label:"init",permalink:"/blog/tags/init"},{label:"onboarding",permalink:"/blog/tags/onboarding"}],readingTime:4.01,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"moon-v1.18",title:"moon v1.18 - New task execution flow and custom project names",authors:["milesj"],tags:["toolchain","shell","id","name","project","init","onboarding"],image:"./img/moon/v1.18.png"},unlisted:!1,prevItem:{title:"proto v0.26 (rc) - Release candidate available for testing!",permalink:"/blog/proto-v0.26-rc"},nextItem:{title:"proto v0.25 - Linux arm64 gnu and musl support",permalink:"/blog/proto-v0.25"}},l={image:o(51691).Z,authorsImageUrls:[void 0]},c=[{value:"New path based task execution",id:"new-path-based-task-execution",level:2},{value:"Dependency executables",id:"dependency-executables",level:3},{value:"Multi-command tasks",id:"multi-command-tasks",level:3},{value:"What&#39;s next?",id:"whats-next",level:3},{value:"Customize the project name in <code>moon.yml</code>",id:"customize-the-project-name-in-moonyml",level:2},{value:"Improved onboarding flow",id:"improved-onboarding-flow",level:2},{value:"Other changes",id:"other-changes",level:2}];function d(e){const n={a:"a",admonition:"admonition",code:"code",em:"em",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.a)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsx)(n.p,{children:"With this release, we've focused heavily on 2 important aspects: task execution, and our onboarding\nflow."}),"\n",(0,t.jsx)(n.h2,{id:"new-path-based-task-execution",children:"New path based task execution"}),"\n",(0,t.jsxs)(n.p,{children:["Since moon's inception, before we would execute a task's command, we would locate its executable on\nthe file system and execute it directly. We went with this approach as we wanted to avoid all of the\nbaggage and \"unknown behavior\" that came with executing through a shell, and to ensure a more\ndeterministic outcome. This approach worked very well for stand-alone binaries, like ",(0,t.jsx)(n.code,{children:"node"}),",\n",(0,t.jsx)(n.code,{children:"cargo"}),", and built-in commands like ",(0,t.jsx)(n.code,{children:"rm"}),", ",(0,t.jsx)(n.code,{children:"mkdir"}),", and ",(0,t.jsx)(n.code,{children:"git"}),"."]}),"\n",(0,t.jsxs)(n.p,{children:["However, it was very problematic in 2 scenarios: executables from language dependencies (Node.js\nmodules, etc), and multi-command based tasks (using ",(0,t.jsx)(n.code,{children:"&&"}),"). To remedy this situation, we're no longer\nlocating the executables ourselves, and instead are prepending ",(0,t.jsx)(n.code,{children:"PATH"})," with the locations in which we\nknow these executables should exist. We're also loosening the restriction on the\n",(0,t.jsxs)(n.a,{href:"/docs/config/project#shell",children:[(0,t.jsx)(n.code,{children:"shell"})," task option"]}),", which can now be enabled for ",(0,t.jsx)(n.em,{children:"all"})," tasks, not\njust system tasks."]}),"\n",(0,t.jsx)(n.h3,{id:"dependency-executables",children:"Dependency executables"}),"\n",(0,t.jsxs)(n.p,{children:["For the 1st scenario, let's talk about Node.js modules. When we encountered an unknown task command,\nlike ",(0,t.jsx)(n.code,{children:"eslint"})," or ",(0,t.jsx)(n.code,{children:"prettier"}),", we'd attempt to locate its executable by traversing ",(0,t.jsx)(n.code,{children:"node_modules/.bin"}),"\ndirectories, parsing Bash/PowerShell scripts, resolving the source ",(0,t.jsx)(n.code,{children:".js"})," files, and finally\nexecuting with ",(0,t.jsx)(n.code,{children:"node"}),". To demonstrate this, say you had the following task:"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  format:\n    command: 'prettier --write .'\n"})}),"\n",(0,t.jsx)(n.p,{children:"When finally executed, internally it would become something like this command:"}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:"~/.proto/tools/node/<version>/bin/node ../../node_modules/prettier/internal/cli.mjs --write .\n"})}),"\n",(0,t.jsxs)(n.p,{children:["This was required since our runtime is Rust and we don't have access to Node.js's module resolution\nalgorithm... but this approach was very brittle and error prone. It took us many releases to iron\nout all the bugs, and we're pretty sure there are still edge cases unaccounted for. So instead, as\nmentioned above, we now prepend ",(0,t.jsx)(n.code,{children:"PATH"}),", resulting in the following command:"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:'PATH="/path/to/node_modules/.bin:/path/to/proto/tools:$PATH" prettier --write .\n'})}),"\n",(0,t.jsx)(n.p,{children:"This is a much cleaner approach and is far easier to understand as a user."}),"\n",(0,t.jsx)(n.h3,{id:"multi-command-tasks",children:"Multi-command tasks"}),"\n",(0,t.jsxs)(n.p,{children:["While not officially supported in moon, it's been possible to run multiple commands in a single task\nusing ",(0,t.jsx)(n.code,{children:"&&"})," syntax. However, this approach did not work correctly with our integrated toolchain, as\nonly the 1st command in the list would have its binary be located and executed correctly."]}),"\n",(0,t.jsxs)(n.p,{children:["For example, say we wanted to run 2 npm packages, the following would ",(0,t.jsx)(n.em,{children:"not"})," work:"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  build:\n    command: 'rm -rf dist && vite build && tsc --build'\n"})}),"\n",(0,t.jsxs)(n.p,{children:["With this new ",(0,t.jsx)(n.code,{children:"PATH"})," based approach, this will now work correctly. And furthermore, this also\nenables executables within Bash and PowerShell scripts to be located and executed correctly as well."]}),"\n",(0,t.jsx)(n.h3,{id:"whats-next",children:"What's next?"}),"\n",(0,t.jsxs)(n.p,{children:["In the future, we'll continue to expand on this functionality, and our ultimate goal is to remove\nthe concept of ",(0,t.jsx)(n.code,{children:"platform"})," from tasks, which has been a bit confusing for new users."]}),"\n",(0,t.jsxs)(n.h2,{id:"customize-the-project-name-in-moonyml",children:["Customize the project name in ",(0,t.jsx)(n.code,{children:"moon.yml"})]}),"\n",(0,t.jsxs)(n.p,{children:["This has been a long requested feature, but thanks to the project graph rework and improvements over\nthe last few releases, this is now possible. In ",(0,t.jsx)(n.a,{href:"/docs/config/project",children:(0,t.jsx)(n.code,{children:"moon.yml"})}),", you can now\nconfigure the ",(0,t.jsx)(n.a,{href:"/docs/config/project#id",children:(0,t.jsx)(n.code,{children:"id"})})," setting to override the project name (identifier)\nderived from ",(0,t.jsx)(n.a,{href:"/docs/config/workspace#projects",children:(0,t.jsx)(n.code,{children:"projects"})})," in\n",(0,t.jsx)(n.a,{href:"/docs/config/workspace",children:(0,t.jsx)(n.code,{children:".moon/workspace.yml"})})," (most applicable to glob based project locations)."]}),"\n",(0,t.jsxs)(n.p,{children:["For example, say we have the following ",(0,t.jsx)(n.code,{children:"projects"})," glob."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/workspace.yml"',children:"projects:\n  - 'apps/*'\n  - 'packages/*'\n"})}),"\n",(0,t.jsxs)(n.p,{children:["By default, the project folder name becomes the project name. For the most part this is fine, but\nwhat if you have a very large monorepo? Or have conflicting project names? Or are migrating\nprojects? It becomes difficult to manage and organize. But now, simply configure ",(0,t.jsx)(n.code,{children:"id"}),"!"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-yaml",metastring:'title="<project>/moon.yml"',children:"id: 'custom-project-name'\n"})}),"\n",(0,t.jsx)(n.admonition,{type:"info",children:(0,t.jsx)(n.p,{children:"Be sure that all targets, project dependencies, task dependencies, and other references are using\nthe new identifier, otherwise an error will be triggered!"})}),"\n",(0,t.jsx)(n.h2,{id:"improved-onboarding-flow",children:"Improved onboarding flow"}),"\n",(0,t.jsxs)(n.p,{children:["While this doesn't affect current users, we still want to announce that we've made some slight\nchanges to our onboarding process and the ",(0,t.jsx)(n.a,{href:"/docs/commands/init",children:(0,t.jsx)(n.code,{children:"moon init"})})," command. The previous\ncommand prompted far too many questions, as we would attempt to detect what languages are currently\nin use, and integrate them into the toolchain."]}),"\n",(0,t.jsx)(n.p,{children:"This was confusing for new users, so starting with this release, we've simplified the process to\nonly create the moon workspace within a repository."}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:"moon init\n"})}),"\n",(0,t.jsx)(n.p,{children:"With that being said, you can still integrate tools into the toolchain, by passing the identifier of\na supported moon tool as an argument."}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:"moon init node # bun, rust, etc\n"})}),"\n",(0,t.jsx)(n.admonition,{type:"success",children:(0,t.jsxs)(n.p,{children:["We've also rewritten a good portion of the \"",(0,t.jsx)(n.a,{href:"/docs/setup-workspace",children:"Getting started"}),'" documentation\nto reflect these changes!']})}),"\n",(0,t.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,t.jsxs)(n.p,{children:["View the ",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/v1.18.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsx)(n.li,{children:"Improved string allocation and performance for queries, task tokens, and process commands."}),"\n",(0,t.jsx)(n.li,{children:"Improved remote caching flow and handling."}),"\n",(0,t.jsx)(n.li,{children:"Updated proto to v0.25."}),"\n"]})]})}function h(e={}){const{wrapper:n}={...(0,s.a)(),...e.components};return n?(0,t.jsx)(n,{...e,children:(0,t.jsx)(d,{...e})}):d(e)}},51691:(e,n,o)=>{o.d(n,{Z:()=>t});const t=o.p+"assets/images/v1.18-e8fd4c540c676c1ac2b787e8e4a17daa.png"},71670:(e,n,o)=>{o.d(n,{Z:()=>r,a:()=>a});var t=o(27378);const s={},i=t.createContext(s);function a(e){const n=t.useContext(i);return t.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function r(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:a(e.components),t.createElement(i.Provider,{value:n},e.children)}}}]);