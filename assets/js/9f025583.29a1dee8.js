"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[77321],{13077:(e,o,n)=>{n.r(o),n.d(o,{assets:()=>l,contentTitle:()=>c,default:()=>u,frontMatter:()=>r,metadata:()=>t,toc:()=>d});const t=JSON.parse('{"id":"proto/commands/alias","title":"alias","description":"The proto alias    (or proto a) command will define a custom alias that","source":"@site/docs/proto/commands/alias.mdx","sourceDirName":"proto/commands","slug":"/proto/commands/alias","permalink":"/docs/proto/commands/alias","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/proto/commands/alias.mdx","tags":[],"version":"current","frontMatter":{"title":"alias"},"sidebar":"proto","previous":{"title":"activate","permalink":"/docs/proto/commands/activate"},"next":{"title":"bin","permalink":"/docs/proto/commands/bin"}}');var s=n(62540),i=n(43023),a=n(54291);const r={title:"alias"},c=void 0,l={},d=[{value:"Arguments",id:"arguments",level:3},{value:"Options",id:"options",level:2}];function p(e){const o={a:"a",code:"code",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,i.R)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsxs)(o.p,{children:["The ",(0,s.jsx)(o.code,{children:"proto alias <tool> <alias> <version>"})," (or ",(0,s.jsx)(o.code,{children:"proto a"}),") command will define a custom alias that\nmaps to a specific version for the provided tool. Aliases can be used anywhere a version is\naccepted."]}),"\n",(0,s.jsx)(o.pre,{children:(0,s.jsx)(o.code,{className:"language-shell",children:"$ proto alias node work 16.16\n"})}),"\n",(0,s.jsxs)(o.p,{children:["By default this will update the local ",(0,s.jsx)(o.a,{href:"../config",children:(0,s.jsx)(o.code,{children:"./.prototools"})})," file. Pass ",(0,s.jsx)(o.code,{children:"--to"})," to customize\nthe location."]}),"\n",(0,s.jsx)(o.h3,{id:"arguments",children:"Arguments"}),"\n",(0,s.jsxs)(o.ul,{children:["\n",(0,s.jsxs)(o.li,{children:[(0,s.jsx)(o.code,{children:"<tool>"})," - Type of tool."]}),"\n",(0,s.jsxs)(o.li,{children:[(0,s.jsx)(o.code,{children:"<alias>"})," - Name of the alias. Supports alphanumeric chars."]}),"\n",(0,s.jsxs)(o.li,{children:[(0,s.jsx)(o.code,{children:"<version>"})," - Version to map to the alias."]}),"\n"]}),"\n",(0,s.jsx)(o.h2,{id:"options",children:"Options"}),"\n",(0,s.jsxs)(o.ul,{children:["\n",(0,s.jsxs)(o.li,{children:[(0,s.jsx)(o.code,{children:"--to"})," - ",(0,s.jsxs)(o.a,{href:"../config#locations",children:["Location of ",(0,s.jsx)(o.code,{children:".prototools"})]})," to update. Supports ",(0,s.jsx)(o.code,{children:"global"}),", ",(0,s.jsx)(o.code,{children:"local"}),",\nand ",(0,s.jsx)(o.code,{children:"user"}),".","\n",(0,s.jsx)(a.A,{version:"0.41.0"}),"\n"]}),"\n"]})]})}function u(e={}){const{wrapper:o}={...(0,i.R)(),...e.components};return o?(0,s.jsx)(o,{...e,children:(0,s.jsx)(p,{...e})}):p(e)}},43023:(e,o,n)=>{n.d(o,{R:()=>a,x:()=>r});var t=n(63696);const s={},i=t.createContext(s);function a(e){const o=t.useContext(i);return t.useMemo((function(){return"function"==typeof e?e(o):{...o,...e}}),[o,e])}function r(e){let o;return o=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:a(e.components),t.createElement(i.Provider,{value:o},e.children)}},54291:(e,o,n)=>{n.d(o,{A:()=>i});var t=n(59115),s=n(62540);function i(e){let{header:o,inline:n,updated:i,version:a}=e;return(0,s.jsx)(t.A,{text:`v${a}`,variant:i?"success":"info",className:o?"absolute right-0 top-1.5":n?"inline-block":"ml-2"})}},59115:(e,o,n)=>{n.d(o,{A:()=>r});var t=n(11750),s=n(20916),i=n(62540);const a={failure:"bg-red-100 text-red-900",info:"bg-pink-100 text-pink-900",success:"bg-green-100 text-green-900",warning:"bg-orange-100 text-orange-900"};function r(e){let{className:o,icon:n,text:r,variant:c}=e;return(0,i.jsxs)("span",{className:(0,t.A)("inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase",c?a[c]:"bg-gray-100 text-gray-800",o),children:[n&&(0,i.jsx)(s.A,{icon:n,className:"mr-1"}),r]})}}}]);