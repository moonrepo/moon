"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[36300],{1161:e=>{e.exports=JSON.parse('{"permalink":"/blog/proto-v0.8","editUrl":"https://github.com/moonrepo/moon/tree/master/website/blog/2023-04-28_proto-v0.8.mdx","source":"@site/blog/2023-04-28_proto-v0.8.mdx","title":"proto v0.8 - Version detection and installation improvements","description":"In this release, we\'re dropping some quality of life workflow improvements.","date":"2023-04-28T00:00:00.000Z","tags":[{"inline":true,"label":"proto","permalink":"/blog/tags/proto"},{"inline":true,"label":"detect","permalink":"/blog/tags/detect"}],"readingTime":1.295,"hasTruncateMarker":true,"authors":[{"name":"Miles Johnson","title":"Founder, developer","url":"https://github.com/milesj","imageURL":"/img/authors/miles.jpg","key":"milesj","page":null}],"frontMatter":{"slug":"proto-v0.8","title":"proto v0.8 - Version detection and installation improvements","authors":["milesj"],"tags":["proto","detect"],"image":"./img/proto/v0.8.png"},"unlisted":false,"prevItem":{"title":"moon v1.4 - New tag target scope, MQL support for query commands, and more!","permalink":"/blog/moon-v1.4"},"nextItem":{"title":"moon v1.3 - Advanced run targeting and an official proto plugin","permalink":"/blog/moon-v1.3"}}')},43023:(e,o,t)=>{t.d(o,{R:()=>s,x:()=>a});var n=t(63696);const r={},i=n.createContext(r);function s(e){const o=n.useContext(i);return n.useMemo((function(){return"function"==typeof e?e(o):{...o,...e}}),[o,e])}function a(e){let o;return o=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:s(e.components),n.createElement(i.Provider,{value:o},e.children)}},80437:(e,o,t)=>{t.r(o),t.d(o,{assets:()=>l,contentTitle:()=>a,default:()=>h,frontMatter:()=>s,metadata:()=>n,toc:()=>c});var n=t(1161),r=t(62540),i=t(43023);const s={slug:"proto-v0.8",title:"proto v0.8 - Version detection and installation improvements",authors:["milesj"],tags:["proto","detect"],image:"./img/proto/v0.8.png"},a=void 0,l={image:t(98829).A,authorsImageUrls:[void 0]},c=[{value:"Built-in detection for <code>proto use</code>",id:"built-in-detection-for-proto-use",level:2},{value:"Smarter version detection",id:"smarter-version-detection",level:2},{value:"Other changes",id:"other-changes",level:2}];function d(e){const o={a:"a",code:"code",em:"em",h2:"h2",p:"p",pre:"pre",...(0,i.R)(),...e.components};return(0,r.jsxs)(r.Fragment,{children:[(0,r.jsx)(o.p,{children:"In this release, we're dropping some quality of life workflow improvements."}),"\n",(0,r.jsxs)(o.h2,{id:"built-in-detection-for-proto-use",children:["Built-in detection for ",(0,r.jsx)(o.code,{children:"proto use"})]}),"\n",(0,r.jsxs)(o.p,{children:["The ",(0,r.jsx)(o.a,{href:"/docs/proto/commands/use",children:(0,r.jsx)(o.code,{children:"proto use"})})," command is extremely useful for bootstrapping your\ndevelopment environment with all necessary tooling, but it had a hard requirement on the\n",(0,r.jsx)(o.a,{href:"/docs/proto/config",children:(0,r.jsx)(o.code,{children:".prototools"})})," configuration file. But what if you're already using non-proto\nversion files, like ",(0,r.jsx)(o.code,{children:".nvmrc"})," or ",(0,r.jsx)(o.code,{children:".dvmrc"}),"? Or maybe manifest settings, like ",(0,r.jsx)(o.code,{children:"packageManager"})," or\n",(0,r.jsx)(o.code,{children:"engines"})," in ",(0,r.jsx)(o.code,{children:"package.json"}),"?"]}),"\n",(0,r.jsxs)(o.p,{children:["Great questions, and we agree! As such, we've updated ",(0,r.jsx)(o.code,{children:"proto use"})," to ",(0,r.jsx)(o.em,{children:"also"})," detect a version from\nthe environment for the current working directory. We suggest using ",(0,r.jsx)(o.code,{children:".prototools"}),", but feel free to\nconfigure your environments as you so choose!"]}),"\n",(0,r.jsx)(o.pre,{children:(0,r.jsx)(o.code,{className:"language-shell",children:"# Install all the things!\n$ proto use\n"})}),"\n",(0,r.jsx)(o.h2,{id:"smarter-version-detection",children:"Smarter version detection"}),"\n",(0,r.jsxs)(o.p,{children:["One of proto's best features is its ",(0,r.jsx)(o.a,{href:"/docs/proto/detection",children:"contextual version detection"}),", but it\ndid have 1 shortcoming. When we detected a partial version, like ",(0,r.jsx)(o.code,{children:"1.2"}),", we'd resolve to a fully\nqualified version with the latest patch version (e.g. ",(0,r.jsx)(o.code,{children:"1.2.3"}),"). While this worked in most cases,\neverytime a new patch was released upstream (e.g. ",(0,r.jsx)(o.code,{children:"1.2.4"}),"), proto would error and require a manual\ninstall of this new version. This was pretty annoying as ",(0,r.jsx)(o.code,{children:"1.2.3"})," and ",(0,r.jsx)(o.code,{children:"1.2.4"})," are likely to be\ncompatible, and both satisfy the ",(0,r.jsx)(o.code,{children:"1.2"})," version constraint."]}),"\n",(0,r.jsxs)(o.p,{children:["To mitigate this scenario, we've updated the version detection to scan the locally installed\nversions ",(0,r.jsx)(o.em,{children:"first"})," when encountering a partial version. This solves the problem above by allowing\n",(0,r.jsx)(o.code,{children:"1.2.3"})," to satisfy the requirement, instead of forcing an install of ",(0,r.jsx)(o.code,{children:"1.2.4"}),"."]}),"\n",(0,r.jsx)(o.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,r.jsxs)(o.p,{children:["View the ",(0,r.jsx)(o.a,{href:"https://github.com/moonrepo/proto/releases/tag/v0.8.0",children:"official release"})," for a full list\nof changes."]})]})}function h(e={}){const{wrapper:o}={...(0,i.R)(),...e.components};return o?(0,r.jsx)(o,{...e,children:(0,r.jsx)(d,{...e})}):d(e)}},98829:(e,o,t)=>{t.d(o,{A:()=>n});const n=t.p+"assets/images/v0.8-9fbf22972083ec9959ca9045d7ea8f95.png"}}]);