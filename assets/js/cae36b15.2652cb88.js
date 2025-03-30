"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[46045],{43023:(n,e,o)=>{o.d(e,{R:()=>i,x:()=>c});var t=o(63696);const r={},s=t.createContext(r);function i(n){const e=t.useContext(s);return t.useMemo((function(){return"function"==typeof n?n(e):{...e,...n}}),[e,n])}function c(n){let e;return e=n.disableParentContext?"function"==typeof n.components?n.components(r):n.components||r:i(n.components),t.createElement(s.Provider,{value:e},n.children)}},95534:(n,e,o)=>{o.r(e),o.d(e,{assets:()=>l,contentTitle:()=>c,default:()=>u,frontMatter:()=>i,metadata:()=>t,toc:()=>d});const t=JSON.parse('{"id":"proto/commands/run","title":"run","description":"The proto run  [version] (or proto r) command will run a tool after","source":"@site/docs/proto/commands/run.mdx","sourceDirName":"proto/commands","slug":"/proto/commands/run","permalink":"/docs/proto/commands/run","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/proto/commands/run.mdx","tags":[],"version":"current","frontMatter":{"title":"run"},"sidebar":"proto","previous":{"title":"regen","permalink":"/docs/proto/commands/regen"},"next":{"title":"setup","permalink":"/docs/proto/commands/setup"}}');var r=o(62540),s=o(43023);const i={title:"run"},c=void 0,l={},d=[{value:"Arguments",id:"arguments",level:3}];function a(n){const e={a:"a",code:"code",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.R)(),...n.components};return(0,r.jsxs)(r.Fragment,{children:[(0,r.jsxs)(e.p,{children:["The ",(0,r.jsx)(e.code,{children:"proto run <tool> [version]"})," (or ",(0,r.jsx)(e.code,{children:"proto r"}),") command will run a tool after\n",(0,r.jsx)(e.a,{href:"../detection",children:"detecting a version"})," from the environment."]}),"\n",(0,r.jsx)(e.pre,{children:(0,r.jsx)(e.code,{className:"language-shell",children:"# Run and detect version from environment\n$ proto run bun\n\n# Run with explicit version\n$ proto run bun 0.5.3\n\n# Run with version from environment variable\n$ PROTO_BUN_VERSION=0.5.3 proto run bun\n"})}),"\n",(0,r.jsxs)(e.p,{children:["Arguments can be passed to the underlying tool binary by providing additional arguments after ",(0,r.jsx)(e.code,{children:"--"}),"."]}),"\n",(0,r.jsx)(e.pre,{children:(0,r.jsx)(e.code,{className:"language-shell",children:"$ proto run bun -- run ./script.ts\n\n# When using the binary on PATH\n$ bun run ./script.ts\n"})}),"\n",(0,r.jsx)(e.h3,{id:"arguments",children:"Arguments"}),"\n",(0,r.jsxs)(e.ul,{children:["\n",(0,r.jsxs)(e.li,{children:[(0,r.jsx)(e.code,{children:"<tool>"})," - Type of tool."]}),"\n",(0,r.jsxs)(e.li,{children:[(0,r.jsx)(e.code,{children:"[version]"})," - Version of tool. If not provided, will attempt to detect the version from the\nenvironment."]}),"\n"]})]})}function u(n={}){const{wrapper:e}={...(0,s.R)(),...n.components};return e?(0,r.jsx)(e,{...n,children:(0,r.jsx)(a,{...n})}):a(n)}}}]);