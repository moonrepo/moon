"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[80108],{37524:(t,e,r)=>{r.d(e,{A:()=>a});var n=r(63696),o=r(36933),s=r(62540);function i(t){return(0,s.jsx)("code",{...t})}function a(t){return function(t){return void 0!==t.children&&n.Children.toArray(t.children).every((t=>"string"==typeof t&&!t.includes("\n")))}(t)?(0,s.jsx)(i,{...t}):(0,s.jsx)(o.default,{...t})}},43023:(t,e,r)=>{r.d(e,{R:()=>i,x:()=>a});var n=r(63696);const o={},s=n.createContext(o);function i(t){const e=n.useContext(s);return n.useMemo((function(){return"function"==typeof t?t(e):{...e,...t}}),[e,t])}function a(t){let e;return e=t.disableParentContext?"function"==typeof t.components?t.components(o):t.components||o:i(t.components),n.createElement(s.Provider,{value:e},t.children)}},58670:(t,e,r)=>{r.r(e),r.d(e,{assets:()=>v,contentTitle:()=>y,default:()=>N,frontMatter:()=>j,metadata:()=>n,toc:()=>A});const n=JSON.parse('{"id":"proto/tools","title":"Supported tools","description":"Built-in","source":"@site/docs/proto/tools.mdx","sourceDirName":"proto","slug":"/proto/tools","permalink":"/docs/proto/tools","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/proto/tools.mdx","tags":[],"version":"current","frontMatter":{"title":"Supported tools"},"sidebar":"proto","previous":{"title":"Version detection","permalink":"/docs/proto/detection"},"next":{"title":"Plugins","permalink":"/docs/proto/plugins"}}');var o=r(62540),s=r(43023),i=r(43067),a=r(63696),l=r(86257),c=r(99985),d=r(84295),u=r(24448),p=r(36933),m=r(37524),h=r(59115);function x(t){let{to:e,noMargin:r}=t;return(0,o.jsx)("a",{href:e,target:"_blank",className:"float-right block",style:{marginTop:r?0:"-3.75em"},children:(0,o.jsx)(h.A,{text:"TOML",icon:"material-symbols:extension",variant:"info"})})}function g(t){let{to:e,noMargin:r}=t;return(0,o.jsx)("a",{href:e,target:"_blank",className:"float-right block",style:{marginTop:r?0:"-3.75em"},children:(0,o.jsx)(h.A,{text:"WASM",icon:"material-symbols:extension",variant:"success"})})}function f(t){let{id:e,tool:r,builtin:n}=t;const s=r.bins??[],i=r.globalsDirs??[],a=r.detectionSources??[],h=r.id??e;let f=`proto install ${h}`;return r.locator&&!n&&(f=`proto plugin add ${h} "${r.locator}"\n${f}`),(0,o.jsxs)("div",{className:"relative rounded-lg px-2 py-2 border-solid border border-t-0 border-b-2 bg-gray-50 border-gray-200/75 dark:bg-slate-700 dark:border-slate-900/75",children:["toml"===r.format&&(0,o.jsx)(x,{to:r.repositoryUrl,noMargin:!0}),"wasm"===r.format&&(0,o.jsx)(g,{to:r.repositoryUrl,noMargin:!0}),(0,o.jsxs)(d.A,{level:5,className:"mb-1",children:[(0,o.jsx)(c.A,{href:r.homepageUrl??r.repositoryUrl,children:r.name}),!n&&(0,o.jsxs)(u.Ay,{as:"span",variant:"muted",size:"sm",className:"ml-1",children:["(",(0,l.L)(r.author),")"]})]}),(0,o.jsx)(u.Ay,{children:r.description}),(0,o.jsx)(p.default,{language:"shell",children:f}),s.length>0&&(0,o.jsxs)(u.Ay,{size:"sm",variant:"muted",className:"m-0 mt-1",children:["Available bins:"," ",s.map(((t,e)=>(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(m.A,{children:t}),e===s.length-1?"":", "]})))]}),i.length>0&&(0,o.jsxs)(u.Ay,{size:"sm",variant:"muted",className:"m-0 mt-1",children:["Globals directory:"," ",i.map(((t,e)=>(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(m.A,{children:t}),e===i.length-1?"":", "]})))]}),a.length>0&&(0,o.jsxs)(u.Ay,{size:"sm",variant:"muted",className:"m-0 mt-1",children:["Detection sources:"," ",a.map(((t,e)=>{let r=(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(m.A,{children:t.file}),t.label?" ":"",t.label]});return r=t.url?(0,o.jsx)(c.A,{href:t.url,children:r}):(0,o.jsx)("span",{children:r}),(0,o.jsxs)(o.Fragment,{children:[r,e===a.length-1?"":", "]})}))]})]})}function b(t){const[e,r]=(0,a.useState)([]),n="third-party"===t.data;return(0,a.useEffect)((()=>{(0,l.A)(t.data).then(r).catch(console.error)}),[]),(0,o.jsx)("div",{className:"grid grid-cols-2 gap-2",children:e.map(((t,e)=>{const r=`${t.id}-${n?(0,l.L)(t.author):"native"}-${e}`;return(0,o.jsx)("div",{id:r,children:(0,o.jsx)(f,{id:t.id,tool:t,builtin:!n})},r)}))})}const j={title:"Supported tools"},y=void 0,v={},A=[{value:"Built-in",id:"built-in",level:2},{value:"Third-party",id:"third-party",level:2}];function k(t){const e={a:"a",h2:"h2",p:"p",...(0,s.R)(),...t.components};return(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(e.h2,{id:"built-in",children:"Built-in"}),"\n",(0,o.jsx)(e.p,{children:"The following tools are supported natively in proto's toolchain."}),"\n",(0,o.jsx)(b,{data:"built-in"}),"\n",(0,o.jsx)(e.h2,{id:"third-party",children:"Third-party"}),"\n",(0,o.jsx)(i.A,{className:"float-right -mt-8",href:"https://github.com/moonrepo/proto/tree/master/registry",label:"Add tool"}),"\n",(0,o.jsxs)(e.p,{children:["Additional tools can be supported through ",(0,o.jsx)(e.a,{href:"./plugins",children:"plugins"}),"."]}),"\n",(0,o.jsx)(b,{data:"third-party"})]})}function N(t={}){const{wrapper:e}={...(0,s.R)(),...t.components};return e?(0,o.jsx)(e,{...t,children:(0,o.jsx)(k,{...t})}):k(t)}},59115:(t,e,r)=>{r.d(e,{A:()=>a});var n=r(11750),o=r(20916),s=r(62540);const i={failure:"bg-red-100 text-red-900",info:"bg-pink-100 text-pink-900",success:"bg-green-100 text-green-900",warning:"bg-orange-100 text-orange-900"};function a(t){let{className:e,icon:r,text:a,variant:l}=t;return(0,s.jsxs)("span",{className:(0,n.A)("inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase",l?i[l]:"bg-gray-100 text-gray-800",e),children:[r&&(0,s.jsx)(o.A,{icon:r,className:"mr-1"}),a]})}},86257:(t,e,r)=>{function n(t){return"string"==typeof t?t:t.name}async function o(t){const e=await fetch(`https://raw.githubusercontent.com/moonrepo/proto/master/registry/data/${t}.json`,{cache:"default"});return(await e.json()).plugins}r.d(e,{A:()=>o,L:()=>n})}}]);