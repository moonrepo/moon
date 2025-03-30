"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[1582],{27328:(e,n,o)=>{o.r(n),o.d(n,{assets:()=>l,contentTitle:()=>c,default:()=>p,frontMatter:()=>a,metadata:()=>t,toc:()=>m});const t=JSON.parse('{"id":"proto/commands/debug/env","title":"debug env","description":"The proto debug env command will print information about your current proto environment. Primarily","source":"@site/docs/proto/commands/debug/env.mdx","sourceDirName":"proto/commands/debug","slug":"/proto/commands/debug/env","permalink":"/docs/proto/commands/debug/env","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/proto/commands/debug/env.mdx","tags":[],"version":"current","frontMatter":{"title":"debug env","sidebar_label":"env"},"sidebar":"proto","previous":{"title":"config","permalink":"/docs/proto/commands/debug/config"},"next":{"title":"diagnose","permalink":"/docs/proto/commands/diagnose"}}');var r=o(62540),s=o(43023),i=o(54291);const a={title:"debug env",sidebar_label:"env"},c=void 0,l={},m=[{value:"Options",id:"options",level:3}];function d(e){const n={code:"code",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.R)(),...e.components};return(0,r.jsxs)(r.Fragment,{children:[(0,r.jsx)(i.A,{version:"0.26.0",header:!0}),"\n",(0,r.jsxs)(n.p,{children:["The ",(0,r.jsx)(n.code,{children:"proto debug env"})," command will print information about your current proto environment. Primarily\nthe store location, relevant file paths, and environment variables."]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{children:"$ proto debug env\n\nStore \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\n\n  Root: /Users/name/.proto\n  Bins: /Users/name/.proto/bin\n  Shims: /Users/name/.proto/shims\n  Plugins: /Users/name/.proto/plugins\n  Tools: /Users/name/.proto/tools\n  Temp: /Users/name/.proto/temp\n\nEnvironment \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\n\n  Proto version: 0.44.0\n  Operating system: macos\n  Architecture: arm64\n  Config sources:\n    - /Users/name/Projects/example/.prototools\n    - /Users/name/.proto/.prototools\n  Virtual paths:\n    /userhome = /Users/name\n    /proto = /Users/name/.proto\n    /cwd = /Users/name/Projects/example\n  Environment variables:\n    PROTO_APP_LOG = proto=info,schematic=info,starbase=info,warpgate=info,extism::pdk=info\n    PROTO_HOME = /Users/name/.proto\n    PROTO_OFFLINE_TIMEOUT = 750\n    PROTO_VERSION = 0.44.0\n"})}),"\n",(0,r.jsx)(n.h3,{id:"options",children:"Options"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--json"})," - Print the list in JSON format."]}),"\n"]})]})}function p(e={}){const{wrapper:n}={...(0,s.R)(),...e.components};return n?(0,r.jsx)(n,{...e,children:(0,r.jsx)(d,{...e})}):d(e)}},43023:(e,n,o)=>{o.d(n,{R:()=>i,x:()=>a});var t=o(63696);const r={},s=t.createContext(r);function i(e){const n=t.useContext(s);return t.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function a(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:i(e.components),t.createElement(s.Provider,{value:n},e.children)}},54291:(e,n,o)=>{o.d(n,{A:()=>s});var t=o(59115),r=o(62540);function s(e){let{header:n,inline:o,updated:s,version:i}=e;return(0,r.jsx)(t.A,{text:`v${i}`,variant:s?"success":"info",className:n?"absolute right-0 top-1.5":o?"inline-block":"ml-2"})}},59115:(e,n,o)=>{o.d(n,{A:()=>a});var t=o(11750),r=o(20916),s=o(62540);const i={failure:"bg-red-100 text-red-900",info:"bg-pink-100 text-pink-900",success:"bg-green-100 text-green-900",warning:"bg-orange-100 text-orange-900"};function a(e){let{className:n,icon:o,text:a,variant:c}=e;return(0,s.jsxs)("span",{className:(0,t.A)("inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase",c?i[c]:"bg-gray-100 text-gray-800",n),children:[o&&(0,s.jsx)(r.A,{icon:o,className:"mr-1"}),a]})}}}]);