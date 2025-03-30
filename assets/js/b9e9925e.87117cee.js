"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[86253],{43023:(e,n,t)=>{t.d(n,{R:()=>o,x:()=>c});var s=t(63696);const i={},r=s.createContext(i);function o(e){const n=s.useContext(r);return s.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function c(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(i):e.components||i:o(e.components),s.createElement(r.Provider,{value:n},e.children)}},89563:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>l,contentTitle:()=>c,default:()=>u,frontMatter:()=>o,metadata:()=>s,toc:()=>a});const s=JSON.parse('{"id":"concepts/file-group","title":"File groups","description":"File groups are a mechanism for grouping similar types of files and environment variables within a","source":"@site/docs/concepts/file-group.mdx","sourceDirName":"concepts","slug":"/concepts/file-group","permalink":"/docs/concepts/file-group","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/concepts/file-group.mdx","tags":[],"version":"current","frontMatter":{"title":"File groups"},"sidebar":"docs","previous":{"title":"Cache","permalink":"/docs/concepts/cache"},"next":{"title":"File patterns","permalink":"/docs/concepts/file-pattern"}}');var i=t(62540),r=t(43023);const o={title:"File groups"},c=void 0,l={},a=[{value:"Configuration",id:"configuration",level:2},{value:"Token functions",id:"token-functions",level:3},{value:"Inheritance and merging",id:"inheritance-and-merging",level:2}];function d(e){const n={a:"a",code:"code",em:"em",h2:"h2",h3:"h3",p:"p",pre:"pre",...(0,r.R)(),...e.components};return(0,i.jsxs)(i.Fragment,{children:[(0,i.jsxs)(n.p,{children:["File groups are a mechanism for grouping similar types of files and environment variables within a\nproject using ",(0,i.jsx)(n.a,{href:"./file-pattern",children:"file glob patterns or literal file paths"}),". These groups are then used\nby ",(0,i.jsx)(n.a,{href:"./task",children:"tasks"})," to calculate functionality like cache computation, affected files since last\nchange, deterministic builds, and more."]}),"\n",(0,i.jsx)(n.h2,{id:"configuration",children:"Configuration"}),"\n",(0,i.jsxs)(n.p,{children:["File groups can be configured per project through ",(0,i.jsx)(n.a,{href:"../config/project",children:(0,i.jsx)(n.code,{children:"moon.yml"})}),", or for many\nprojects through ",(0,i.jsx)(n.a,{href:"../config/tasks",children:(0,i.jsx)(n.code,{children:".moon/tasks.yml"})}),"."]}),"\n",(0,i.jsx)(n.h3,{id:"token-functions",children:"Token functions"}),"\n",(0,i.jsxs)(n.p,{children:["File groups can be referenced in ",(0,i.jsx)(n.a,{href:"./task",children:"tasks"})," using ",(0,i.jsx)(n.a,{href:"./token",children:"token functions"}),". For example, the\n",(0,i.jsx)(n.code,{children:"@group(name)"})," token will expand to all paths configured in the ",(0,i.jsx)(n.code,{children:"sources"})," file group."]}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  build:\n    command: 'vite build'\n    inputs:\n      - '@group(sources)'\n"})}),"\n",(0,i.jsx)(n.h2,{id:"inheritance-and-merging",children:"Inheritance and merging"}),"\n",(0,i.jsxs)(n.p,{children:["When a file group of the same name exists in both ",(0,i.jsx)(n.a,{href:"#configuration",children:"configuration files"}),", the\nproject-level group will override the workspace-level group, and all other workspace-level groups\nwill be inherited as-is."]}),"\n",(0,i.jsxs)(n.p,{children:["A primary scenario in which to define file groups at the project-level is when you want to\n",(0,i.jsx)(n.em,{children:"override"})," file groups defined at the workspace-level. For example, say we want to override the\n",(0,i.jsx)(n.code,{children:"sources"}),' file group because our source folder is named "lib" and not "src", we would define our\nfile groups as followed.']}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks.yml"',children:"fileGroups:\n  sources:\n    - 'src/**/*'\n    - 'types/**/*'\n  tests:\n    - 'tests/**/*.test.*'\n    - '**/__tests__/**/*'\n"})}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"fileGroups:\n  # Overrides global\n  sources:\n    - 'lib/**/*'\n    - 'types/**/*'\n  # Inherited as-is\n  tests:\n    - 'tests/**/*.test.*'\n    - '**/__tests__/**/*'\n"})})]})}function u(e={}){const{wrapper:n}={...(0,r.R)(),...e.components};return n?(0,i.jsx)(n,{...e,children:(0,i.jsx)(d,{...e})}):d(e)}}}]);