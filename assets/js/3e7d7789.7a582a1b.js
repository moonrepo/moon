"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[22108],{83828:(e,n,t)=>{t.d(n,{ZP:()=>d,gE:()=>c});var s=t(27378),r=t(3620),i=t(24246);const a=["/docs/install","/docs/setup-workspace","/docs/setup-toolchain","/docs/create-project","/docs/create-task","/docs/run-task","/docs/migrate-to-moon"];function o(){return"undefined"!=typeof window&&"localStorage"in window}function l(){return(o()?localStorage.getItem("moonrepo.language"):null)??"node"}function c(){const[e,n]=(0,s.useState)(l());return(0,s.useEffect)((()=>{const e=e=>{n(e.detail)};return window.addEventListener("onMoonrepoChangeLanguage",e),()=>{window.removeEventListener("onMoonrepoChangeLanguage",e)}})),e}function d(){const[e,n]=(0,s.useState)(l()),t=(0,r.TH)(),c=(0,s.useCallback)((e=>{let{target:t}=e;const s=t.value;if(n(s),o())try{localStorage.setItem("moonrepo.language",s)}catch{}window.dispatchEvent(new CustomEvent("onMoonrepoChangeLanguage",{bubbles:!0,detail:s}))}),[]);return a.some((e=>t.pathname.startsWith(e)))?(0,i.jsxs)("select",{value:e,onChange:c,className:"outline-none min-w-0 bg-white border border-solid border-gray-400 dark:border-transparent rounded-md p-0.5 text-sm text-gray-800 placeholder-gray-600 h-full font-sans",children:[(0,i.jsx)("option",{value:"bun",children:"Bun"}),(0,i.jsx)("option",{value:"deno",children:"Deno"}),(0,i.jsx)("option",{value:"go",children:"Go"}),(0,i.jsx)("option",{value:"node",children:"Node.js"}),(0,i.jsx)("option",{value:"php",children:"PHP"}),(0,i.jsx)("option",{value:"python",children:"Python"}),(0,i.jsx)("option",{value:"ruby",children:"Ruby"}),(0,i.jsx)("option",{value:"rust",children:"Rust"})]}):null}},78372:(e,n,t)=>{t.r(n),t.d(n,{default:()=>p});var s=t(45161),r=t(8862),i=t(98948),a=t(83828),o=t(75686),l=t(90728),c=t(30658),d=t(24246);function m(e){let{active:n,children:t,href:s}=e;return s?(0,d.jsx)(l.Z,{"aria-current":n?"page":void 0,href:s,itemProp:"item",size:"sm",variant:"muted",weight:"medium",children:(0,d.jsx)("span",{itemProp:"name",children:t})}):(0,d.jsx)(c.ZP,{"aria-current":n?"page":void 0,as:"span",itemProp:"item name",size:"sm",variant:"muted",weight:"medium",className:"m-0",children:t})}function u(e){let{children:n,index:t}=e;return(0,d.jsx)("li",{itemScope:!0,itemProp:"itemListElement",itemType:"https://schema.org/ListItem",children:(0,d.jsxs)("div",{className:"flex items-center",children:[(0,d.jsx)(o.Z,{icon:"material-symbols:arrow-forward-ios-rounded",className:"flex-shrink-0 text-gray-600 mr-2","aria-hidden":"true"}),n,(0,d.jsx)("meta",{itemProp:"position",content:String(t+1)})]})})}function h(){const e=(0,i.ZP)("/");return(0,d.jsx)("li",{children:(0,d.jsxs)(l.Z,{href:e,variant:"muted",children:[(0,d.jsx)(o.Z,{icon:"material-symbols:home-rounded",className:"flex-shrink-0","aria-hidden":"true",width:"1.1em",style:{paddingTop:5}}),(0,d.jsx)("span",{className:"sr-only",children:"Home"})]})})}function p(){const e=(0,s.s1)(),n=(0,r.Ns)();return e?(0,d.jsxs)(d.Fragment,{children:[(0,d.jsx)("span",{className:"float-right ml-2",children:(0,d.jsx)(a.ZP,{})}),(0,d.jsx)("nav",{className:"flex pl-1 mb-2","aria-label":"Breadcrumb",children:(0,d.jsxs)("ol",{role:"list",className:"list-none p-0 m-0 flex items-center space-x-2",itemScope:!0,itemType:"https://schema.org/BreadcrumbList",children:[n&&(0,d.jsx)(h,{}),e.map(((n,t)=>(0,d.jsx)(u,{index:t,children:(0,d.jsx)(m,{href:t<e.length?n.href:void 0,active:t===e.length-1,children:n.label})},t)))]})})]}):null}},44022:(e,n,t)=>{t.d(n,{Z:()=>o});var s=t(40624),r=t(75686),i=t(90728),a=t(24246);function o(e){let{permalink:n,title:t,isNext:o}=e;return(0,a.jsx)("div",{className:(0,s.Z)("flex-1",o?"text-right":"text-left"),children:(0,a.jsxs)(i.Z,{weight:"bold",to:n,className:"inline-flex",children:[!o&&(0,a.jsx)(r.Z,{className:"mr-1 icon-previous",icon:"material-symbols:chevron-left-rounded",width:"1.5em"}),(0,a.jsx)("span",{children:t}),o&&(0,a.jsx)(r.Z,{className:"ml-1 icon-next",icon:"material-symbols:chevron-right-rounded",width:"1.5em"})]})})}}}]);