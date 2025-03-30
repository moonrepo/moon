"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[36084],{21828:(e,n,t)=>{t.d(n,{D:()=>o});var s=t(60359),a=t(51571),i=t.n(a);function o(e,n){return(0,s.A)({container:e,elements:n,layout:{fit:!0,name:"dagre",nodeDimensionsIncludeLabels:!0,spacingFactor:1},style:[{selector:"edges",style:{"arrow-scale":2,"curve-style":"straight","line-cap":"round","line-color":"#c9eef6","line-opacity":.25,"overlay-color":"#c9eef6","target-arrow-color":"#c9eef6","target-arrow-shape":"tee",width:3}},{selector:"node",style:{"background-fill":"linear-gradient","background-gradient-direction":"to-bottom-right","background-gradient-stop-colors":"#d7dfe9 #bdc9db #97a1af",color:"#fff",height:60,label:"data(label)","overlay-color":"#99aab7","overlay-shape":"ellipse",padding:"0",shape:"ellipse","text-halign":"center","text-margin-y":6,"text-valign":"bottom","underlay-shape":"ellipse",width:60}},{selector:'node[type="run-task"], node[type="sm"]',style:{"background-gradient-stop-colors":"#6e58d1 #4a2ec6 #3b259e"}},{selector:'node[type="run-target"], node[type="sm"]',style:{"background-gradient-stop-colors":"#6e58d1 #4a2ec6 #3b259e"}},{selector:'node[type="sync-project"], node[type="md"]',style:{"background-gradient-stop-colors":"#ffafff #ff79ff #cc61cc",height:80,width:80}},{selector:'node[type="install-deps"], node[type="lg"]',style:{"background-gradient-stop-colors":"#afe6f2 #79d5e9 #61aaba",height:100,width:100}},{selector:'node[type="setup-toolchain"], node[type="xl"]',style:{"background-gradient-stop-colors":"#ff9da6 #ff5b6b #cc4956",height:120,width:120}},{selector:'node[id="sync-workspace"]',style:{"background-gradient-stop-colors":"#b7a9f9 #9a87f7 #8c75f5",height:120,width:120}}]})}s.A.use(i())},90267:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>h,contentTitle:()=>d,default:()=>g,frontMatter:()=>l,metadata:()=>s,toc:()=>p});const s=JSON.parse('{"id":"how-it-works/action-graph","title":"Action graph","description":"When you run a task on the command line, we generate an action graph to","source":"@site/docs/how-it-works/action-graph.mdx","sourceDirName":"how-it-works","slug":"/how-it-works/action-graph","permalink":"/docs/how-it-works/action-graph","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/how-it-works/action-graph.mdx","tags":[],"version":"current","frontMatter":{"title":"Action graph"},"sidebar":"docs","previous":{"title":"Task graph","permalink":"/docs/how-it-works/task-graph"},"next":{"title":"Setup workspace","permalink":"/docs/setup-workspace"}}');var a=t(62540),i=t(43023),o=t(63696),r=t(21828);function c(){const e=(0,o.useRef)(null);return(0,o.useEffect)((()=>{e.current&&(0,r.D)(e.current,{edges:[{data:{source:"sync-workspace",target:"node-toolchain"}},{data:{source:"sync-workspace",target:"system-toolchain"}},{data:{source:"node-toolchain",target:"node-deps"}},{data:{source:"system-toolchain",target:"system-deps"}},{data:{source:"node-toolchain",target:"node-sync"}},{data:{source:"system-toolchain",target:"system-sync"}},{data:{source:"system-sync",target:"target-clean"}},{data:{source:"system-deps",target:"target-clean"}},{data:{source:"node-sync",target:"target-build"}},{data:{source:"node-deps",target:"target-build"}},{data:{source:"target-clean",target:"target-build"}},{data:{source:"target-build",target:"target-package"}}],nodes:[{data:{id:"sync-workspace",label:"SyncWorkspace"}},{data:{id:"node-toolchain",label:"SetupToolchain(node:18.0.0)",type:"xl"}},{data:{id:"system-toolchain",label:"SetupToolchain(system)",type:"xl"}},{data:{id:"node-deps",label:"InstallWorkspaceDeps(node:18.0.0)",type:"lg"}},{data:{id:"system-deps",label:"InstallProjectDeps(node:18.0.0, example)",type:"lg"}},{data:{id:"node-sync",label:"SyncProject(node, example)",type:"md"}},{data:{id:"system-sync",label:"SyncProject(system, example)",type:"md"}},{data:{id:"target-clean",label:"RunTask(example:clean)",type:"sm"}},{data:{id:"target-build",label:"RunTask(example:build)",type:"sm"}},{data:{id:"target-package",label:"RunTask(example:package)",type:"sm"}}]})}),[]),(0,a.jsx)("div",{id:"dep-graph",ref:e,className:"p-1 mb-2 rounded bg-slate-800",style:{height:"550px",width:"100%"}})}const l={title:"Action graph"},d=void 0,h={},p=[{value:"Actions",id:"actions",level:2},{value:"Sync workspace",id:"sync-workspace",level:3},{value:"Setup toolchain",id:"setup-toolchain",level:3},{value:"Install dependencies",id:"install-dependencies",level:3},{value:"Sync project",id:"sync-project",level:3},{value:"Run task",id:"run-task",level:3},{value:"Run interactive task",id:"run-interactive-task",level:3},{value:"Run persistent task",id:"run-persistent-task",level:3},{value:"What is the graph used for?",id:"what-is-the-graph-used-for",level:2}];function u(e){const n={a:"a",admonition:"admonition",blockquote:"blockquote",code:"code",em:"em",h2:"h2",h3:"h3",li:"li",p:"p",ul:"ul",...(0,i.R)(),...e.components};return(0,a.jsxs)(a.Fragment,{children:[(0,a.jsxs)(n.p,{children:["When you run a ",(0,a.jsx)(n.a,{href:"../config/project#tasks-1",children:"task"})," on the command line, we generate an action graph to\nensure ",(0,a.jsx)(n.a,{href:"../config/project#deps",children:"dependencies"})," of tasks have ran before running run the primary task."]}),"\n",(0,a.jsxs)(n.p,{children:["The action graph is a representation of all ",(0,a.jsx)(n.a,{href:"../concepts/task",children:"tasks"}),", derived from the\n",(0,a.jsx)(n.a,{href:"./project-graph",children:"project graph"})," and ",(0,a.jsx)(n.a,{href:"./task-graph",children:"task graph"}),", and is also represented internally\nas a directed acyclic graph (DAG)."]}),"\n",(0,a.jsx)(c,{}),"\n",(0,a.jsx)(n.h2,{id:"actions",children:"Actions"}),"\n",(0,a.jsx)(n.p,{children:"Unlike other task runners in the industry that represent each node in the graph as a task to run, we\nrepresent each node in the graph as an action to perform. This allows us to be more flexible and\nefficient with how we run tasks, and allows us to provide more functionality and automation than\nother runners."}),"\n",(0,a.jsx)(n.p,{children:"The following actions compose our action graph:"}),"\n",(0,a.jsx)(n.h3,{id:"sync-workspace",children:"Sync workspace"}),"\n",(0,a.jsx)(n.p,{children:"This is a common action that always runs and give's moon a chance to perform operations and health\nchecks across the entire workspace."}),"\n",(0,a.jsx)(n.admonition,{type:"info",children:(0,a.jsxs)(n.p,{children:["This action can be skipped by disabling the\n",(0,a.jsx)(n.a,{href:"../config/workspace#syncworkspace",children:(0,a.jsx)(n.code,{children:"pipeline.syncWorkspace"})})," setting."]})}),"\n",(0,a.jsx)(n.h3,{id:"setup-toolchain",children:"Setup toolchain"}),"\n",(0,a.jsx)(n.p,{children:"The most important action in the graph is the setup toolchain action, which downloads and installs a\ntier 3 language into the toolchain. For other tiers, this is basically a no-operation."}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsx)(n.li,{children:"When the tool has already been installed, this action will be skipped."}),"\n",(0,a.jsxs)(n.li,{children:["Actions will be scoped by language and version, also known as a runtime. For example,\n",(0,a.jsx)(n.code,{children:"SetupToolchain(node:18.1.0)"})," or ",(0,a.jsx)(n.code,{children:"SetupToolchain(deno:1.31.0)"}),"."]}),"\n",(0,a.jsxs)(n.li,{children:["Tools that require a global binary (found on ",(0,a.jsx)(n.code,{children:"PATH"}),') will display the version as "global". For\nexample, ',(0,a.jsx)(n.code,{children:"SetupToolchain(node:global)"}),"."]}),"\n"]}),"\n",(0,a.jsx)(n.admonition,{type:"info",children:(0,a.jsxs)(n.p,{children:["This action can be skipped by setting the ",(0,a.jsx)(n.code,{children:"MOON_SKIP_SETUP_TOOLCHAIN=true"})," environment variable. The\nskip can be scoped per tool by setting the value to the tool name (",(0,a.jsx)(n.code,{children:"node"}),"), and also by version\n(",(0,a.jsx)(n.code,{children:"node:20.0.0"}),"). Supports a comma-separated list."]})}),"\n",(0,a.jsx)(n.h3,{id:"install-dependencies",children:"Install dependencies"}),"\n",(0,a.jsxs)(n.p,{children:["Before we run a task, we ensure that all language dependencies (",(0,a.jsx)(n.code,{children:"node_modules"})," for example) have\nbeen installed, by automatically installing them if we detect changes since the last run. We achieve\nthis by comparing lockfile modified timestamps, parsing manifest files, and hashing resolved\ndependency versions."]}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:["When dependencies do ",(0,a.jsx)(n.em,{children:"not"})," need to be installed, this action will be skipped."]}),"\n",(0,a.jsxs)(n.li,{children:["Depending on the language and configuration, we may install dependencies in a project\n(",(0,a.jsx)(n.code,{children:"InstallProjectDeps"}),"), or in the workspace root for all projects (",(0,a.jsx)(n.code,{children:"InstallWorkspaceDeps"}),")."]}),"\n",(0,a.jsxs)(n.li,{children:["Actions will be scoped by language and version, also known as a runtime. For example,\n",(0,a.jsx)(n.code,{children:"InstallWorkspaceDeps(node:18.1.0)"})," or ",(0,a.jsx)(n.code,{children:"InstallProjectDeps(node:18.1.0, example)"}),"."]}),"\n"]}),"\n",(0,a.jsxs)(n.blockquote,{children:["\n",(0,a.jsx)(n.p,{children:"This action depends on the setup toolchain action, because we utilize the binaries in the\ntoolchain to install dependencies."}),"\n"]}),"\n",(0,a.jsx)(n.admonition,{type:"info",children:(0,a.jsxs)(n.p,{children:["This action can be skipped by disabling the\n",(0,a.jsx)(n.a,{href:"../config/workspace#installdependencies",children:(0,a.jsx)(n.code,{children:"pipeline.installDependencies"})})," setting."]})}),"\n",(0,a.jsx)(n.h3,{id:"sync-project",children:"Sync project"}),"\n",(0,a.jsxs)(n.p,{children:["To ensure a consistently healthy project and repository, we run a process known as syncing\n",(0,a.jsx)(n.em,{children:"everytime"})," a task is ran. Actions will be scoped by language, for example,\n",(0,a.jsx)(n.code,{children:"SyncProject(node, example)"}),"."]}),"\n",(0,a.jsx)(n.p,{children:"What is synced or considered healthcare is dependent on the language and its ecosystem."}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:["JavaScript","\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:["Syncs ",(0,a.jsx)(n.code,{children:"package.json"})," dependencies based on ",(0,a.jsx)(n.a,{href:"./project-graph",children:"project graph"})," dependencies."]}),"\n",(0,a.jsxs)(n.li,{children:["Applies ",(0,a.jsx)(n.a,{href:"../config/toolchain#deno",children:(0,a.jsx)(n.code,{children:"deno"})})," and ",(0,a.jsx)(n.a,{href:"../config/toolchain#node",children:(0,a.jsx)(n.code,{children:"node"})})," related\nsettings."]}),"\n"]}),"\n"]}),"\n",(0,a.jsxs)(n.li,{children:["TypeScript","\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:["Syncs project references based on ",(0,a.jsx)(n.a,{href:"./project-graph",children:"project graph"})," dependencies."]}),"\n",(0,a.jsxs)(n.li,{children:["Applies ",(0,a.jsx)(n.a,{href:"../config/toolchain#typescript",children:(0,a.jsx)(n.code,{children:"typescript"})})," related settings."]}),"\n"]}),"\n"]}),"\n"]}),"\n",(0,a.jsxs)(n.blockquote,{children:["\n",(0,a.jsx)(n.p,{children:"This action depends on the setup toolchain action, in case it requires binaries or functionality\nthat the toolchain provides."}),"\n"]}),"\n",(0,a.jsx)(n.admonition,{type:"info",children:(0,a.jsxs)(n.p,{children:["This action can be skipped by disabling the\n",(0,a.jsx)(n.a,{href:"../config/workspace#syncproject",children:(0,a.jsx)(n.code,{children:"pipeline.syncProject"})})," setting."]})}),"\n",(0,a.jsx)(n.h3,{id:"run-task",children:"Run task"}),"\n",(0,a.jsxs)(n.p,{children:["The primary action in the graph is the run ",(0,a.jsx)(n.a,{href:"../concepts/task",children:"task"})," action, which runs a project's\ntask as a child process, derived from a ",(0,a.jsx)(n.a,{href:"../concepts/target",children:"target"}),". Tasks can depend on other\ntasks, and they'll be effectively orchestrated and executed by running in topological order using a\nthread pool."]}),"\n",(0,a.jsxs)(n.blockquote,{children:["\n",(0,a.jsx)(n.p,{children:"This action depends on the previous actions, as the toolchain is used for running the task's\ncommand, and the outcome of the task is best when the project state is healthy and deterministic."}),"\n"]}),"\n",(0,a.jsx)(n.h3,{id:"run-interactive-task",children:"Run interactive task"}),"\n",(0,a.jsxs)(n.p,{children:["Like the base run task, but runs the ",(0,a.jsx)(n.a,{href:"../concepts/task#interactive",children:"task interactively"})," with stdin\ncapabilities. All interactive tasks are run in isolation in the graph."]}),"\n",(0,a.jsx)(n.h3,{id:"run-persistent-task",children:"Run persistent task"}),"\n",(0,a.jsxs)(n.p,{children:["Like the base run task, but runs the ",(0,a.jsx)(n.a,{href:"../concepts/task#persistent",children:"task in a persistent process"}),"\nthat never exits. All persistent tasks are run in parallel as the last batch in the graph."]}),"\n",(0,a.jsx)(n.h2,{id:"what-is-the-graph-used-for",children:"What is the graph used for?"}),"\n",(0,a.jsx)(n.p,{children:"Without the action graph, tasks would not efficiently run, or possibly at all! The graph helps to\nrun tasks in parallel, in the correct order, and to ensure a reliable outcome."})]})}function g(e={}){const{wrapper:n}={...(0,i.R)(),...e.components};return n?(0,a.jsx)(n,{...e,children:(0,a.jsx)(u,{...e})}):u(e)}}}]);