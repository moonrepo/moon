"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[54709],{43023:(e,n,t)=>{t.d(n,{R:()=>a,x:()=>l});var o=t(63696);const c={},s=o.createContext(c);function a(e){const n=o.useContext(s);return o.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function l(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(c):e.components||c:a(e.components),o.createElement(s.Provider,{value:n},e.children)}},74292:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>i,contentTitle:()=>l,default:()=>d,frontMatter:()=>a,metadata:()=>o,toc:()=>r});const o=JSON.parse('{"id":"commands/clean","title":"clean","description":"The moon clean command will clean the current workspace by deleting stale cache. For the most","source":"@site/docs/commands/clean.mdx","sourceDirName":"commands","slug":"/commands/clean","permalink":"/docs/commands/clean","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/commands/clean.mdx","tags":[],"version":"current","frontMatter":{"title":"clean"},"sidebar":"docs","previous":{"title":"ci","permalink":"/docs/commands/ci"},"next":{"title":"completions","permalink":"/docs/commands/completions"}}');var c=t(62540),s=t(43023);const a={title:"clean"},l=void 0,i={},r=[{value:"Options",id:"options",level:3}];function m(e){const n={code:"code",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.R)(),...e.components};return(0,c.jsxs)(c.Fragment,{children:[(0,c.jsxs)(n.p,{children:["The ",(0,c.jsx)(n.code,{children:"moon clean"})," command will clean the current workspace by deleting stale cache. For the most\npart, the action pipeline will clean automatically, but this command can be used to reset the\nworkspace entirely."]}),"\n",(0,c.jsx)(n.pre,{children:(0,c.jsx)(n.code,{className:"language-shell",children:"$ moon clean\n\n# Delete cache with a custom lifetime\n$ moon clean --lifetime '24 hours'\n"})}),"\n",(0,c.jsx)(n.h3,{id:"options",children:"Options"}),"\n",(0,c.jsxs)(n.ul,{children:["\n",(0,c.jsxs)(n.li,{children:[(0,c.jsx)(n.code,{children:"--lifetime"}),' - The maximum lifetime of cached artifacts before being marked as stale. Defaults to\n"7 days".']}),"\n"]})]})}function d(e={}){const{wrapper:n}={...(0,s.R)(),...e.components};return n?(0,c.jsx)(n,{...e,children:(0,c.jsx)(m,{...e})}):m(e)}}}]);