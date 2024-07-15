"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[52753],{98842:(e,n,s)=>{s.r(n),s.d(n,{assets:()=>l,contentTitle:()=>a,default:()=>u,frontMatter:()=>o,metadata:()=>c,toc:()=>d});var t=s(24246),r=s(71670),i=s(79022);const o={title:"query tasks",sidebar_label:"tasks"},a=void 0,c={id:"commands/query/tasks",title:"query tasks",description:"Use the moon query tasks sub-command to query task information for all projects in the project",source:"@site/docs/commands/query/tasks.mdx",sourceDirName:"commands/query",slug:"/commands/query/tasks",permalink:"/docs/commands/query/tasks",draft:!1,unlisted:!1,editUrl:"https://github.com/moonrepo/moon/tree/master/website/docs/commands/query/tasks.mdx",tags:[],version:"current",frontMatter:{title:"query tasks",sidebar_label:"tasks"},sidebar:"docs",previous:{title:"projects",permalink:"/docs/commands/query/projects"},next:{title:"touched-files",permalink:"/docs/commands/query/touched-files"}},l={},d=[{value:"Arguments",id:"arguments",level:3},{value:"Options",id:"options",level:3},{value:"Filters",id:"filters",level:4},{value:"Configuration",id:"configuration",level:3}];function h(e){const n={a:"a",code:"code",em:"em",h3:"h3",h4:"h4",li:"li",p:"p",pre:"pre",ul:"ul",...(0,r.a)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsxs)(n.p,{children:["Use the ",(0,t.jsx)(n.code,{children:"moon query tasks"})," sub-command to query task information for all projects in the project\ngraph. The tasks list can be filtered by passing a ",(0,t.jsx)(n.a,{href:"../../concepts/query-lang",children:"query statement"})," as\nan argument, or by using ",(0,t.jsx)(n.a,{href:"#options",children:"options"})," arguments."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:'# Find all tasks grouped by project\n$ moon query tasks\n\n# Find all tasks from projects with an id that matches "react"\n$ moon query tasks --id react\n$ moon query tasks "task~react"\n'})}),"\n",(0,t.jsx)(n.p,{children:"By default, this will output a list of projects, and tasks within the project being indented (with a\ntab) on their own line."}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{children:"web\n\t:lint | eslint\n\t:test | jest\napp\n\t:format | prettier\n"})}),"\n",(0,t.jsxs)(n.p,{children:["The tasks can also be output in JSON (",(0,t.jsx)(n.a,{href:"/api/types/interface/Task",children:"which contains all data"}),") by\npassing the ",(0,t.jsx)(n.code,{children:"--json"})," flag. The output has the following structure:"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-ts",children:"{\n\ttasks: Record<string, Record<string, Task>>,\n\toptions: QueryOptions,\n}\n"})}),"\n",(0,t.jsx)(n.h3,{id:"arguments",children:"Arguments"}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"[query]"})," - An optional ",(0,t.jsx)(n.a,{href:"../../concepts/query-lang",children:"query statement"})," to filter projects with. When\nprovided, all ",(0,t.jsx)(n.a,{href:"#filters",children:"filter options"})," are ignored. ",(0,t.jsx)(i.Z,{version:"1.4.0"})]}),"\n"]}),"\n",(0,t.jsx)(n.h3,{id:"options",children:"Options"}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--affected"})," - Filter projects that have been affected by touched files. This will only filter\nbased on files, and ",(0,t.jsx)(n.em,{children:"does not"})," include upstream or downstream dependencies."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--json"})," - Display the projects in JSON format."]}),"\n"]}),"\n",(0,t.jsx)(n.h4,{id:"filters",children:"Filters"}),"\n",(0,t.jsx)(n.p,{children:"All option values are case-insensitive regex patterns."}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--alias <regex>"})," - Filter projects that match this alias."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--id <regex>"})," - Filter projects that match this ID/name."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--language <regex>"})," - Filter projects of this programming language."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--source <regex>"})," - Filter projects that match this source path."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--tasks <regex>"})," - Filter projects that have the following tasks."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--type <regex>"})," - Filter project of this type."]}),"\n"]}),"\n",(0,t.jsx)(n.h3,{id:"configuration",children:"Configuration"}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.a,{href:"../../config/workspace#projects",children:(0,t.jsx)(n.code,{children:"projects"})})," in ",(0,t.jsx)(n.code,{children:".moon/workspace.yml"})]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.a,{href:"../../config/project#tasks",children:(0,t.jsx)(n.code,{children:"tasks"})})," in ",(0,t.jsx)(n.code,{children:"moon.yml"})]}),"\n"]})]})}function u(e={}){const{wrapper:n}={...(0,r.a)(),...e.components};return n?(0,t.jsx)(n,{...e,children:(0,t.jsx)(h,{...e})}):h(e)}},79022:(e,n,s)=>{s.d(n,{Z:()=>i});var t=s(9619),r=s(24246);function i(e){let{header:n,inline:s,updated:i,version:o}=e;return(0,r.jsx)(t.Z,{text:`v${o}`,variant:i?"success":"info",className:n?"absolute right-0 top-1.5":s?"inline-block":"ml-2"})}},9619:(e,n,s)=>{s.d(n,{Z:()=>a});var t=s(40624),r=s(31792),i=s(24246);const o={failure:"bg-red-100 text-red-900",info:"bg-pink-100 text-pink-900",success:"bg-green-100 text-green-900",warning:"bg-orange-100 text-orange-900"};function a(e){let{className:n,icon:s,text:a,variant:c}=e;return(0,i.jsxs)("span",{className:(0,t.Z)("inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase",c?o[c]:"bg-gray-100 text-gray-800",n),children:[s&&(0,i.jsx)(r.Z,{icon:s,className:"mr-1"}),a]})}},71670:(e,n,s)=>{s.d(n,{Z:()=>a,a:()=>o});var t=s(27378);const r={},i=t.createContext(r);function o(e){const n=t.useContext(i);return t.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function a(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:o(e.components),t.createElement(i.Provider,{value:n},e.children)}}}]);