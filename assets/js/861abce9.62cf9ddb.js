"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[15995],{71259:(e,n,s)=>{s.r(n),s.d(n,{assets:()=>l,contentTitle:()=>i,default:()=>u,frontMatter:()=>r,metadata:()=>a,toc:()=>d});var c=s(24246),o=s(71670),t=s(79022);const r={title:"sync projects",sidebar_label:"projects"},i=void 0,a={id:"commands/sync/projects",title:"sync projects",description:"The moon sync projects command will force sync all projects in the workspace to help achieve a",source:"@site/docs/commands/sync/projects.mdx",sourceDirName:"commands/sync",slug:"/commands/sync/projects",permalink:"/docs/commands/sync/projects",draft:!1,unlisted:!1,editUrl:"https://github.com/moonrepo/moon/tree/master/website/docs/commands/sync/projects.mdx",tags:[],version:"current",frontMatter:{title:"sync projects",sidebar_label:"projects"},sidebar:"docs",previous:{title:"hooks",permalink:"/docs/commands/sync/hooks"},next:{title:"task",permalink:"/docs/commands/task"}},l={},d=[{value:"Configuration",id:"configuration",level:3}];function p(e){const n={a:"a",blockquote:"blockquote",code:"code",em:"em",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,o.a)(),...e.components};return(0,c.jsxs)(c.Fragment,{children:[(0,c.jsx)(t.Z,{version:"1.8.0",header:!0}),"\n",(0,c.jsxs)(n.p,{children:["The ",(0,c.jsx)(n.code,{children:"moon sync projects"})," command will force sync ",(0,c.jsx)(n.em,{children:"all"})," projects in the workspace to help achieve a\n",(0,c.jsx)(n.a,{href:"../../faq#what-should-be-considered-the-source-of-truth",children:"healthy repository state"}),". This applies\nthe following:"]}),"\n",(0,c.jsxs)(n.ul,{children:["\n",(0,c.jsxs)(n.li,{children:["Ensures cross-project dependencies are linked based on\n",(0,c.jsx)(n.a,{href:"../../config/project#dependson",children:(0,c.jsx)(n.code,{children:"dependsOn"})}),"."]}),"\n",(0,c.jsxs)(n.li,{children:["Ensures language specific configuration files are present and accurate (",(0,c.jsx)(n.code,{children:"package.json"}),",\n",(0,c.jsx)(n.code,{children:"tsconfig.json"}),", etc)."]}),"\n",(0,c.jsx)(n.li,{children:"Ensures root configuration and project configuration are in sync."}),"\n",(0,c.jsx)(n.li,{children:"Any additional language specific semantics that may be required."}),"\n"]}),"\n",(0,c.jsx)(n.pre,{children:(0,c.jsx)(n.code,{className:"language-shell",children:"$ moon sync projects\n"})}),"\n",(0,c.jsxs)(n.blockquote,{children:["\n",(0,c.jsxs)(n.p,{children:["This command should rarely be ran, as ",(0,c.jsx)(n.a,{href:"../run",children:(0,c.jsx)(n.code,{children:"moon run"})})," will sync affected projects\nautomatically! However, when migrating or refactoring, manual syncing may be necessary."]}),"\n"]}),"\n",(0,c.jsx)(n.h3,{id:"configuration",children:"Configuration"}),"\n",(0,c.jsxs)(n.ul,{children:["\n",(0,c.jsxs)(n.li,{children:[(0,c.jsx)(n.a,{href:"../../config/workspace#projects",children:(0,c.jsx)(n.code,{children:"projects"})})," in ",(0,c.jsx)(n.code,{children:".moon/workspace.yml"})]}),"\n"]})]})}function u(e={}){const{wrapper:n}={...(0,o.a)(),...e.components};return n?(0,c.jsx)(n,{...e,children:(0,c.jsx)(p,{...e})}):p(e)}},79022:(e,n,s)=>{s.d(n,{Z:()=>t});var c=s(9619),o=s(24246);function t(e){let{header:n,inline:s,updated:t,version:r}=e;return(0,o.jsx)(c.Z,{text:`v${r}`,variant:t?"success":"info",className:n?"absolute right-0 top-1.5":s?"inline-block":"ml-2"})}},9619:(e,n,s)=>{s.d(n,{Z:()=>i});var c=s(40624),o=s(31792),t=s(24246);const r={failure:"bg-red-100 text-red-900",info:"bg-pink-100 text-pink-900",success:"bg-green-100 text-green-900",warning:"bg-orange-100 text-orange-900"};function i(e){let{className:n,icon:s,text:i,variant:a}=e;return(0,t.jsxs)("span",{className:(0,c.Z)("inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase",a?r[a]:"bg-gray-100 text-gray-800",n),children:[s&&(0,t.jsx)(o.Z,{icon:s,className:"mr-1"}),i]})}},71670:(e,n,s)=>{s.d(n,{Z:()=>i,a:()=>r});var c=s(27378);const o={},t=c.createContext(o);function r(e){const n=c.useContext(t);return c.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function i(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(o):e.components||o:r(e.components),c.createElement(t.Provider,{value:n},e.children)}}}]);