"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[16507],{2845:(e,n,s)=>{s.r(n),s.d(n,{assets:()=>i,contentTitle:()=>c,default:()=>l,frontMatter:()=>r,metadata:()=>o,toc:()=>h});const o=JSON.parse('{"id":"commands/query/hash","title":"query hash","description":"Use the moon query hash sub-command to inspect the contents and sources of a generated hash, also","source":"@site/docs/commands/query/hash.mdx","sourceDirName":"commands/query","slug":"/commands/query/hash","permalink":"/docs/commands/query/hash","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/commands/query/hash.mdx","tags":[],"version":"current","frontMatter":{"title":"query hash","sidebar_label":"hash"},"sidebar":"docs","previous":{"title":"query","permalink":"/docs/commands/query"},"next":{"title":"hash-diff","permalink":"/docs/commands/query/hash-diff"}}');var t=s(62540),a=s(43023);const r={title:"query hash",sidebar_label:"hash"},c=void 0,i={},h=[{value:"Options",id:"options",level:3},{value:"Configuration",id:"configuration",level:3}];function d(e){const n={a:"a",code:"code",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,a.R)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsxs)(n.p,{children:["Use the ",(0,t.jsx)(n.code,{children:"moon query hash"})," sub-command to inspect the contents and sources of a generated hash, also\nknown as the hash manifest. This is extremely useful in debugging task inputs."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:"$ moon query hash 0b55b234f1018581c45b00241d7340dc648c63e639fbafdaf85a4cd7e718fdde\n\n# Query hash using short form\n$ moon query hash 0b55b234\n"})}),"\n",(0,t.jsx)(n.p,{children:"By default, this will output the contents of the hash manifest (which is JSON), and the fully\nqualified resolved hash."}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-json",children:'Hash: 0b55b234f1018581c45b00241d7340dc648c63e639fbafdaf85a4cd7e718fdde\n\n{\n  "command": "build",\n  "args": ["./build"]\n  // ...\n}\n'})}),"\n",(0,t.jsxs)(n.p,{children:["The command can also be output raw JSON by passing the ",(0,t.jsx)(n.code,{children:"--json"})," flag."]}),"\n",(0,t.jsx)(n.h3,{id:"options",children:"Options"}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--json"})," - Display the diff in JSON format."]}),"\n"]}),"\n",(0,t.jsx)(n.h3,{id:"configuration",children:"Configuration"}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.a,{href:"../../config/workspace#hasher",children:(0,t.jsx)(n.code,{children:"hasher"})})," in ",(0,t.jsx)(n.code,{children:".moon/workspace.yml"})]}),"\n"]})]})}function l(e={}){const{wrapper:n}={...(0,a.R)(),...e.components};return n?(0,t.jsx)(n,{...e,children:(0,t.jsx)(d,{...e})}):d(e)}},43023:(e,n,s)=>{s.d(n,{R:()=>r,x:()=>c});var o=s(63696);const t={},a=o.createContext(t);function r(e){const n=o.useContext(a);return o.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function c(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(t):e.components||t:r(e.components),o.createElement(a.Provider,{value:n},e.children)}}}]);