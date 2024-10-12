"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[27460],{95684:(e,o,n)=>{n.r(o),n.d(o,{assets:()=>c,contentTitle:()=>i,default:()=>p,frontMatter:()=>a,metadata:()=>l,toc:()=>d});var t=n(24246),s=n(71670),r=n(79022);const a={title:"unalias"},i=void 0,l={id:"proto/commands/unalias",title:"unalias",description:"The proto unalias   (or proto ua) command will remove a custom alias for the",source:"@site/docs/proto/commands/unalias.mdx",sourceDirName:"proto/commands",slug:"/proto/commands/unalias",permalink:"/docs/proto/commands/unalias",draft:!1,unlisted:!1,editUrl:"https://github.com/moonrepo/moon/tree/master/website/docs/proto/commands/unalias.mdx",tags:[],version:"current",frontMatter:{title:"unalias"},sidebar:"proto",previous:{title:"status",permalink:"/docs/proto/commands/status"},next:{title:"uninstall",permalink:"/docs/proto/commands/uninstall"}},c={},d=[{value:"Arguments",id:"arguments",level:3},{value:"Options",id:"options",level:2}];function u(e){const o={a:"a",code:"code",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.a)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsxs)(o.p,{children:["The ",(0,t.jsx)(o.code,{children:"proto unalias <tool> <alias>"})," (or ",(0,t.jsx)(o.code,{children:"proto ua"}),") command will remove a custom alias for the\nprovided tool."]}),"\n",(0,t.jsx)(o.pre,{children:(0,t.jsx)(o.code,{className:"language-shell",children:"$ proto unalias node work\n"})}),"\n",(0,t.jsxs)(o.p,{children:["By default this will update the local ",(0,t.jsx)(o.a,{href:"../config",children:(0,t.jsx)(o.code,{children:"./.prototools"})})," file. Pass ",(0,t.jsx)(o.code,{children:"--from"})," to customize\nthe location."]}),"\n",(0,t.jsx)(o.h3,{id:"arguments",children:"Arguments"}),"\n",(0,t.jsxs)(o.ul,{children:["\n",(0,t.jsxs)(o.li,{children:[(0,t.jsx)(o.code,{children:"<tool>"})," - Type of tool."]}),"\n",(0,t.jsxs)(o.li,{children:[(0,t.jsx)(o.code,{children:"<alias>"})," - Name of the alias. Supports alphanumeric chars."]}),"\n"]}),"\n",(0,t.jsx)(o.h2,{id:"options",children:"Options"}),"\n",(0,t.jsxs)(o.ul,{children:["\n",(0,t.jsxs)(o.li,{children:[(0,t.jsx)(o.code,{children:"--from"})," - ",(0,t.jsxs)(o.a,{href:"../config#locations",children:["Location of ",(0,t.jsx)(o.code,{children:".prototools"})]})," to update.","\n",(0,t.jsx)(r.Z,{version:"0.41.0"}),"\n"]}),"\n",(0,t.jsxs)(o.li,{children:[(0,t.jsx)(o.code,{children:"--global"})," (deprecated) - Remove from the global ",(0,t.jsx)(o.code,{children:"~/.proto/.prototools"})," instead of the local\n",(0,t.jsx)(o.code,{children:"./.prototools"}),"."]}),"\n"]})]})}function p(e={}){const{wrapper:o}={...(0,s.a)(),...e.components};return o?(0,t.jsx)(o,{...e,children:(0,t.jsx)(u,{...e})}):u(e)}},79022:(e,o,n)=>{n.d(o,{Z:()=>r});var t=n(9619),s=n(24246);function r(e){let{header:o,inline:n,updated:r,version:a}=e;return(0,s.jsx)(t.Z,{text:`v${a}`,variant:r?"success":"info",className:o?"absolute right-0 top-1.5":n?"inline-block":"ml-2"})}},9619:(e,o,n)=>{n.d(o,{Z:()=>i});var t=n(40624),s=n(31792),r=n(24246);const a={failure:"bg-red-100 text-red-900",info:"bg-pink-100 text-pink-900",success:"bg-green-100 text-green-900",warning:"bg-orange-100 text-orange-900"};function i(e){let{className:o,icon:n,text:i,variant:l}=e;return(0,r.jsxs)("span",{className:(0,t.Z)("inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase",l?a[l]:"bg-gray-100 text-gray-800",o),children:[n&&(0,r.jsx)(s.Z,{icon:n,className:"mr-1"}),i]})}},71670:(e,o,n)=>{n.d(o,{Z:()=>i,a:()=>a});var t=n(27378);const s={},r=t.createContext(s);function a(e){const o=t.useContext(r);return t.useMemo((function(){return"function"==typeof e?e(o):{...o,...e}}),[o,e])}function i(e){let o;return o=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:a(e.components),t.createElement(r.Provider,{value:o},e.children)}}}]);