"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[24720],{43023:(e,n,s)=>{s.d(n,{R:()=>o,x:()=>c});var i=s(63696);const r={},t=i.createContext(r);function o(e){const n=i.useContext(t);return i.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function c(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:o(e.components),i.createElement(t.Provider,{value:n},e.children)}},54291:(e,n,s)=>{s.d(n,{A:()=>t});var i=s(59115),r=s(62540);function t(e){let{header:n,inline:s,updated:t,version:o}=e;return(0,r.jsx)(i.A,{text:`v${o}`,variant:t?"success":"info",className:n?"absolute right-0 top-1.5":s?"inline-block":"ml-2"})}},59115:(e,n,s)=>{s.d(n,{A:()=>c});var i=s(11750),r=s(20916),t=s(62540);const o={failure:"bg-red-100 text-red-900",info:"bg-pink-100 text-pink-900",success:"bg-green-100 text-green-900",warning:"bg-orange-100 text-orange-900"};function c(e){let{className:n,icon:s,text:c,variant:d}=e;return(0,t.jsxs)("span",{className:(0,i.A)("inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase",d?o[d]:"bg-gray-100 text-gray-800",n),children:[s&&(0,t.jsx)(r.A,{icon:s,className:"mr-1"}),c]})}},89478:(e,n,s)=>{s.r(n),s.d(n,{assets:()=>l,contentTitle:()=>d,default:()=>u,frontMatter:()=>c,metadata:()=>i,toc:()=>a});const i=JSON.parse('{"id":"commands/run","title":"run","description":"The moon run (or moon r, or moonx) command will run one or many targets","source":"@site/docs/commands/run.mdx","sourceDirName":"commands","slug":"/commands/run","permalink":"/docs/commands/run","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/commands/run.mdx","tags":[],"version":"current","frontMatter":{"title":"run"},"sidebar":"docs","previous":{"title":"touched-files","permalink":"/docs/commands/query/touched-files"},"next":{"title":"setup","permalink":"/docs/commands/setup"}}');var r=s(62540),t=s(43023),o=s(54291);const c={title:"run"},d=void 0,l={},a=[{value:"Arguments",id:"arguments",level:3},{value:"Options",id:"options",level:3},{value:"Affected",id:"affected",level:4},{value:"Configuration",id:"configuration",level:3}];function h(e){const n={a:"a",admonition:"admonition",code:"code",em:"em",h3:"h3",h4:"h4",li:"li",p:"p",pre:"pre",ul:"ul",...(0,t.R)(),...e.components};return(0,r.jsxs)(r.Fragment,{children:[(0,r.jsxs)(n.p,{children:["The ",(0,r.jsx)(n.code,{children:"moon run"})," (or ",(0,r.jsx)(n.code,{children:"moon r"}),", or ",(0,r.jsx)(n.code,{children:"moonx"}),") command will run one or many ",(0,r.jsx)(n.a,{href:"../concepts/target",children:"targets"}),"\nand all of its dependencies in topological order. Each run will incrementally cache each task,\nimproving speed and development times... over time. View the official ",(0,r.jsx)(n.a,{href:"../run-task",children:"Run a task"})," and\n",(0,r.jsx)(n.a,{href:"../cheat-sheet#tasks",children:"Cheat sheet"})," articles for more information!"]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-shell",children:"# Run `lint` in project `app`\n$ moon run app:lint\n$ moonx app:lint\n\n# Run `dev` in project `client` and `server`\n$ moon run client:dev server:dev\n$ moonx client:dev server:dev\n\n# Run `test` in all projects\n$ moon run :test\n$ moonx :test\n\n# Run `test` in all projects with tag `frontend`\n$ moon run '#frontend:test'\n$ moonx '#frontend:test'\n\n# Run `format` in closest project (`client`)\n$ cd apps/client\n$ moon run format\n$ moonx format\n\n# Run `build` in projects matching the query\n$ moon run :build --query \"language=javascript && projectType=library\"\n"})}),"\n",(0,r.jsxs)(n.admonition,{type:"info",children:[(0,r.jsx)(n.p,{children:"How affected status is determined is highly dependent on whether the command is running locally, in\nCI, and what options are provided. The following scenarios are possible:"}),(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:["When ",(0,r.jsx)(n.code,{children:"--affected"})," is provided, will explicitly use ",(0,r.jsx)(n.code,{children:"--remote"})," to determine CI or local."]}),"\n",(0,r.jsxs)(n.li,{children:["When not provided, will use ",(0,r.jsx)(n.code,{children:"git diff"})," in CI, or ",(0,r.jsx)(n.code,{children:"git status"})," for local."]}),"\n",(0,r.jsxs)(n.li,{children:["To bypass affected logic entirely, use ",(0,r.jsx)(n.code,{children:"--force"}),"."]}),"\n"]})]}),"\n",(0,r.jsx)(n.admonition,{type:"info",children:(0,r.jsxs)(n.p,{children:["The default behavior for ",(0,r.jsx)(n.code,{children:"moon run"}),' is to "fail fast", meaning that any failed task will immediately\nabort execution of the entire action graph. Pass ',(0,r.jsx)(n.code,{children:"--no-bail"})," to execute as many tasks as safely\npossible (tasks with upstream failures will be skipped to avoid side effects). This is the default\nbehavior for ",(0,r.jsx)(n.code,{children:"moon ci"}),", and is also useful for pre-commit hooks."]})}),"\n",(0,r.jsx)(n.h3,{id:"arguments",children:"Arguments"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"...<target>"})," - ",(0,r.jsx)(n.a,{href:"../concepts/target",children:"Targets"})," or project relative tasks to run."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"[-- <args>]"})," - Additional arguments to\n",(0,r.jsx)(n.a,{href:"../run-task#passing-arguments-to-the-underlying-command",children:"pass to the underlying command"}),"."]}),"\n"]}),"\n",(0,r.jsx)(n.h3,{id:"options",children:"Options"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"-f"}),", ",(0,r.jsx)(n.code,{children:"--force"})," - Force run and ignore touched files and affected status. Will not query VCS."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--dependents"})," - Run downstream dependent targets (of the same task name) as well."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"-i"}),", ",(0,r.jsx)(n.code,{children:"--interactive"})," - Run the target in an interactive mode."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--profile <type>"})," - Record and ",(0,r.jsx)(n.a,{href:"../guides/profile",children:"generate a profile"})," for ran tasks.","\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:["Types: ",(0,r.jsx)(n.code,{children:"cpu"}),", ",(0,r.jsx)(n.code,{children:"heap"})]}),"\n"]}),"\n"]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--query"})," - Filter projects to run targets against using\n",(0,r.jsx)(n.a,{href:"../concepts/query-lang",children:"a query statement"}),". ",(0,r.jsx)(o.A,{version:"1.3.0"})]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--summary"})," - Display a summary and stats of the current run. ",(0,r.jsx)(o.A,{version:"1.25.0"})]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"-u"}),", ",(0,r.jsx)(n.code,{children:"--updateCache"})," - Bypass cache and force update any existing items."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--no-actions"})," - Run the task without running ",(0,r.jsx)(n.a,{href:"../how-it-works/action-graph",children:"other actions"})," in the\npipeline.","\n",(0,r.jsx)(o.A,{version:"1.34.0"}),"\n"]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"-n"}),", ",(0,r.jsx)(n.code,{children:"--no-bail"})," - When a task fails, continue executing other tasks instead of aborting\nimmediately"]}),"\n"]}),"\n",(0,r.jsx)(n.h4,{id:"affected",children:"Affected"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--affected"})," - Only run target if affected by changed files, ",(0,r.jsx)(n.em,{children:"otherwise"})," will always run."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--remote"})," - Determine affected against remote by comparing ",(0,r.jsx)(n.code,{children:"HEAD"})," against a base revision\n(default branch), ",(0,r.jsx)(n.em,{children:"otherwise"})," uses local changes.","\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:["Can control revisions with ",(0,r.jsx)(n.code,{children:"MOON_BASE"})," and ",(0,r.jsx)(n.code,{children:"MOON_HEAD"}),"."]}),"\n"]}),"\n"]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--status <type>"})," - Filter affected based on a change status. Can be passed multiple times.","\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:["Types: ",(0,r.jsx)(n.code,{children:"all"})," (default), ",(0,r.jsx)(n.code,{children:"added"}),", ",(0,r.jsx)(n.code,{children:"deleted"}),", ",(0,r.jsx)(n.code,{children:"modified"}),", ",(0,r.jsx)(n.code,{children:"staged"}),", ",(0,r.jsx)(n.code,{children:"unstaged"}),", ",(0,r.jsx)(n.code,{children:"untracked"})]}),"\n"]}),"\n"]}),"\n"]}),"\n",(0,r.jsx)(n.h3,{id:"configuration",children:"Configuration"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.a,{href:"../config/workspace#projects",children:(0,r.jsx)(n.code,{children:"projects"})})," in ",(0,r.jsx)(n.code,{children:".moon/workspace.yml"})]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.a,{href:"../config/tasks#tasks",children:(0,r.jsx)(n.code,{children:"tasks"})})," in ",(0,r.jsx)(n.code,{children:".moon/tasks.yml"})]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.a,{href:"../config/project#tasks",children:(0,r.jsx)(n.code,{children:"tasks"})})," in ",(0,r.jsx)(n.code,{children:"moon.yml"})]}),"\n"]})]})}function u(e={}){const{wrapper:n}={...(0,t.R)(),...e.components};return n?(0,r.jsx)(n,{...e,children:(0,r.jsx)(h,{...e})}):h(e)}}}]);