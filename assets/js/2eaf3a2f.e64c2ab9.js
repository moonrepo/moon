"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[31657],{43023:(e,o,n)=>{n.d(o,{R:()=>i,x:()=>l});var t=n(63696);const r={},s=t.createContext(r);function i(e){const o=t.useContext(s);return t.useMemo((function(){return"function"==typeof e?e(o):{...o,...e}}),[o,e])}function l(e){let o;return o=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:i(e.components),t.createElement(s.Provider,{value:o},e.children)}},56565:e=>{e.exports=JSON.parse('{"permalink":"/blog/proto-v0.11","editUrl":"https://github.com/moonrepo/moon/tree/master/website/blog/2023-06-25_proto-v0.11.mdx","source":"@site/blog/2023-06-25_proto-v0.11.mdx","title":"proto v0.11 - New shims and better logging","description":"This is a small release that improves shims and logs.","date":"2023-06-25T00:00:00.000Z","tags":[{"inline":true,"label":"proto","permalink":"/blog/tags/proto"},{"inline":true,"label":"shim","permalink":"/blog/tags/shim"},{"inline":true,"label":"logging","permalink":"/blog/tags/logging"}],"readingTime":1.04,"hasTruncateMarker":true,"authors":[{"name":"Miles Johnson","title":"Founder, developer","url":"https://github.com/milesj","imageURL":"/img/authors/miles.jpg","key":"milesj","page":null}],"frontMatter":{"slug":"proto-v0.11","title":"proto v0.11 - New shims and better logging","authors":["milesj"],"tags":["proto","shim","logging"]},"unlisted":false,"prevItem":{"title":"moon v1.9 - VCS hooks management and improved task inheritance","permalink":"/blog/moon-v1.9"},"nextItem":{"title":"moon v1.8 - Code owners and shared configuration","permalink":"/blog/moon-v1.8"}}')},69180:(e,o,n)=>{n.r(o),n.d(o,{assets:()=>a,contentTitle:()=>l,default:()=>d,frontMatter:()=>i,metadata:()=>t,toc:()=>h});var t=n(56565),r=n(62540),s=n(43023);const i={slug:"proto-v0.11",title:"proto v0.11 - New shims and better logging",authors:["milesj"],tags:["proto","shim","logging"]},l=void 0,a={authorsImageUrls:[void 0]},h=[{value:"New and improved shims",id:"new-and-improved-shims",level:2},{value:"Better logging",id:"better-logging",level:2},{value:"Other changes",id:"other-changes",level:2}];function c(e){const o={a:"a",code:"code",h2:"h2",p:"p",pre:"pre",...(0,s.R)(),...e.components};return(0,r.jsxs)(r.Fragment,{children:[(0,r.jsx)(o.p,{children:"This is a small release that improves shims and logs."}),"\n",(0,r.jsx)(o.h2,{id:"new-and-improved-shims",children:"New and improved shims"}),"\n",(0,r.jsxs)(o.p,{children:["The core facet of proto is our shims found at ",(0,r.jsx)(o.code,{children:"~/.proto/bin"}),". They exist purely to re-route tool\nexecutions internally to proto, so that we can detect the correct version of these tools to run.\nHowever, maintaining and creating these shims has historically been very complicated. So we chose to\nrewrite them from the ground-up!"]}),"\n",(0,r.jsxs)(o.p,{children:['All tools should continue to function exactly as they did before, if not better. Furthermore,\nbecause of this new shim layer, we\'re now able to create what we call "secondary shims", like\n',(0,r.jsxs)(o.a,{href:"https://bun.sh/docs/cli/bunx",children:[(0,r.jsx)(o.code,{children:"bunx"})," for Bun"]}),", ",(0,r.jsx)(o.code,{children:"pnpx"})," for pnpm, and ",(0,r.jsx)(o.code,{children:"yarnpkg"})," for Yarn."]}),"\n",(0,r.jsx)(o.h2,{id:"better-logging",children:"Better logging"}),"\n",(0,r.jsxs)(o.p,{children:["proto has supported logging since its initial release behind the ",(0,r.jsx)(o.code,{children:"PROTO_LOG"})," environment variable.\nHowever, this variable wasn't heavily documented, nor easily discoverable. So as an alternative, we\nnow support a global ",(0,r.jsx)(o.code,{children:"--log"})," option, which can be passed to any ",(0,r.jsx)(o.code,{children:"proto"})," command."]}),"\n",(0,r.jsx)(o.pre,{children:(0,r.jsx)(o.code,{className:"language-shell",children:"$ proto install node --log trace\n"})}),"\n",(0,r.jsx)(o.p,{children:"On top of this, we also ran an audit of all our log calls, to improve messaging, include additional\ninformation, rework applicable levels, and more. They should be far more readable!"}),"\n",(0,r.jsx)(o.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,r.jsxs)(o.p,{children:["View the ",(0,r.jsx)(o.a,{href:"https://github.com/moonrepo/proto/releases/tag/v0.11.0",children:"official release"})," for a full list\nof changes."]})]})}function d(e={}){const{wrapper:o}={...(0,s.R)(),...e.components};return o?(0,r.jsx)(o,{...e,children:(0,r.jsx)(c,{...e})}):c(e)}}}]);