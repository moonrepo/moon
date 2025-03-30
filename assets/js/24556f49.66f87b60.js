"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[6450],{43023:(e,n,s)=>{s.d(n,{R:()=>i,x:()=>l});var t=s(63696);const r={},c=t.createContext(r);function i(e){const n=t.useContext(c);return t.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function l(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:i(e.components),t.createElement(c.Provider,{value:n},e.children)}},45882:(e,n,s)=>{s.r(n),s.d(n,{assets:()=>o,contentTitle:()=>l,default:()=>h,frontMatter:()=>i,metadata:()=>t,toc:()=>a});const t=JSON.parse('{"id":"concepts/file-pattern","title":"File patterns","description":"Globs","source":"@site/docs/concepts/file-pattern.mdx","sourceDirName":"concepts","slug":"/concepts/file-pattern","permalink":"/docs/concepts/file-pattern","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/concepts/file-pattern.mdx","tags":[],"version":"current","frontMatter":{"title":"File patterns"},"sidebar":"docs","previous":{"title":"File groups","permalink":"/docs/concepts/file-group"},"next":{"title":"Query language","permalink":"/docs/concepts/query-lang"}}');var r=s(62540),c=s(43023);const i={title:"File patterns"},l=void 0,o={},a=[{value:"Globs",id:"globs",level:2},{value:"Supported syntax",id:"supported-syntax",level:3},{value:"Examples",id:"examples",level:3},{value:"Project relative",id:"project-relative",level:2},{value:"Workspace relative",id:"workspace-relative",level:2}];function d(e){const n={a:"a",code:"code",em:"em",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,c.R)(),...e.components};return(0,r.jsxs)(r.Fragment,{children:[(0,r.jsx)(n.h2,{id:"globs",children:"Globs"}),"\n",(0,r.jsxs)(n.p,{children:["Globs in moon are ",(0,r.jsx)(n.a,{href:"https://github.com/olson-sean-k/wax",children:"Rust-based globs"}),", ",(0,r.jsx)(n.em,{children:"not"})," JavaScript-based.\nThis may result in different or unexpected results. The following guidelines must be met when using\nglobs:"]}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:["Must use forward slashes (",(0,r.jsx)(n.code,{children:"/"}),") for path separators, even on Windows."]}),"\n",(0,r.jsxs)(n.li,{children:["Must ",(0,r.jsx)(n.em,{children:"not"})," start with or use any relative path parts, ",(0,r.jsx)(n.code,{children:"."})," or ",(0,r.jsx)(n.code,{children:".."}),"."]}),"\n"]}),"\n",(0,r.jsx)(n.h3,{id:"supported-syntax",children:"Supported syntax"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"*"})," - Matches zero or more characters, but does not match the ",(0,r.jsx)(n.code,{children:"/"})," character. Will attempt to match\nthe longest possible text (eager)."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"$"})," - Like ",(0,r.jsx)(n.code,{children:"*"}),", but will attempt to match the shortest possible text (lazy)."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"**"})," - Matches zero or more directories."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"?"})," - Matches exactly one character, but not ",(0,r.jsx)(n.code,{children:"/"}),"."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"[abc]"})," - Matches one case-sensitive character listed in the brackets."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"[!xyz]"})," - Like the above, but will match any character ",(0,r.jsx)(n.em,{children:"not"})," listed."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"[a-z]"})," - Matches one case-sensitive character in range in the brackets."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"[!x-z]"})," - Like the above, but will match any character ",(0,r.jsx)(n.em,{children:"not"})," in range."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"{glob,glob}"})," - Matches one or more comma separated list of sub-glob patterns."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"<glob:n,n>"})," - Matches a sub-glob within a defined bounds."]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"!"})," - At the start of a pattern, will negate previous positive patterns."]}),"\n"]}),"\n",(0,r.jsx)(n.h3,{id:"examples",children:"Examples"}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-bash",children:"README.{md,mdx,txt}\nsrc/**/*\ntests/**/*.?js\n!**/__tests__/**/*\nlogs/<[0-9]:4>-<[0-9]:2>-<[0-9]:2>.log\n"})}),"\n",(0,r.jsx)(n.h2,{id:"project-relative",children:"Project relative"}),"\n",(0,r.jsxs)(n.p,{children:["When configuring ",(0,r.jsx)(n.a,{href:"../config/project#filegroups",children:(0,r.jsx)(n.code,{children:"fileGroups"})}),", ",(0,r.jsx)(n.a,{href:"../config/project#inputs",children:(0,r.jsx)(n.code,{children:"inputs"})}),",\nand ",(0,r.jsx)(n.a,{href:"../config/project#outputs",children:(0,r.jsx)(n.code,{children:"outputs"})}),", all listed file paths and globs are relative from the\nproject root they will be ran in. They ",(0,r.jsx)(n.em,{children:"must not"})," traverse upwards with ",(0,r.jsx)(n.code,{children:".."}),"."]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-bash",children:"# Valid\nsrc/**/*\n./src/**/*\npackage.json\n\n# Invalid\n../utils\n"})}),"\n",(0,r.jsx)(n.h2,{id:"workspace-relative",children:"Workspace relative"}),"\n",(0,r.jsxs)(n.p,{children:["When configuring ",(0,r.jsx)(n.a,{href:"../config/project#filegroups",children:(0,r.jsx)(n.code,{children:"fileGroups"})}),", ",(0,r.jsx)(n.a,{href:"../config/project#inputs",children:(0,r.jsx)(n.code,{children:"inputs"})}),",\nand ",(0,r.jsx)(n.a,{href:"../config/project#outputs",children:(0,r.jsx)(n.code,{children:"outputs"})}),", a listed file path or glob can be prefixed with ",(0,r.jsx)(n.code,{children:"/"})," to\nresolve relative from the workspace root, and ",(0,r.jsx)(n.em,{children:"not"})," the project root."]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-bash",children:"# In project\npackage.json\n\n# In workspace\n/package.json\n"})})]})}function h(e={}){const{wrapper:n}={...(0,c.R)(),...e.components};return n?(0,r.jsx)(n,{...e,children:(0,r.jsx)(d,{...e})}):d(e)}}}]);