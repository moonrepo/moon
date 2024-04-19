"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[42277],{45681:(e,n,o)=>{o.r(n),o.d(n,{assets:()=>l,contentTitle:()=>s,default:()=>u,frontMatter:()=>a,metadata:()=>i,toc:()=>c});var t=o(24246),r=o(71670);const a={slug:"moon-v1.3",title:"moon v1.3 - Advanced run targeting and an official proto plugin",authors:["milesj"],tags:["query","lang","proto","schema","toml","target"],image:"./img/moon/v1.3.png"},s=void 0,i={permalink:"/blog/moon-v1.3",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2023-04-24_moon-v1.3.mdx",source:"@site/blog/2023-04-24_moon-v1.3.mdx",title:"moon v1.3 - Advanced run targeting and an official proto plugin",description:"After many months of design and development, we're finally introducing MQL, our own unique query",date:"2023-04-24T00:00:00.000Z",tags:[{label:"query",permalink:"/blog/tags/query"},{label:"lang",permalink:"/blog/tags/lang"},{label:"proto",permalink:"/blog/tags/proto"},{label:"schema",permalink:"/blog/tags/schema"},{label:"toml",permalink:"/blog/tags/toml"},{label:"target",permalink:"/blog/tags/target"}],readingTime:2.35,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"moon-v1.3",title:"moon v1.3 - Advanced run targeting and an official proto plugin",authors:["milesj"],tags:["query","lang","proto","schema","toml","target"],image:"./img/moon/v1.3.png"},unlisted:!1,prevItem:{title:"proto v0.8 - Version detection and installation improvements",permalink:"/blog/proto-v0.8"},nextItem:{title:"proto v0.7 - First step towards plugins",permalink:"/blog/proto-v0.7"}},l={image:o(15627).Z,authorsImageUrls:[void 0]},c=[{value:"Run targets based on a query",id:"run-targets-based-on-a-query",level:2},{value:"Plugin support for proto",id:"plugin-support-for-proto",level:2},{value:"Other changes",id:"other-changes",level:2}];function d(e){const n={a:"a",blockquote:"blockquote",code:"code",em:"em",h2:"h2",li:"li",p:"p",pre:"pre",ul:"ul",...(0,r.a)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsx)(n.p,{children:"After many months of design and development, we're finally introducing MQL, our own unique query\nlanguage!"}),"\n",(0,t.jsx)(n.h2,{id:"run-targets-based-on-a-query",children:"Run targets based on a query"}),"\n",(0,t.jsxs)(n.p,{children:["Our ",(0,t.jsx)(n.a,{href:"/docs/commands/run",children:(0,t.jsx)(n.code,{children:"moon run"})})," command is pretty powerful. It allows you to run targets in\none, many, or all projects. It also supports running multiple targets in parallel. However, it\nwasn't powerful enough, as it couldn't run the following types of scenarios:"]}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsx)(n.li,{children:"Run a target in projects of a specific language."}),"\n",(0,t.jsx)(n.li,{children:"Run a target in libraries or applications."}),"\n",(0,t.jsx)(n.li,{children:"Run a target based on project file system path."}),"\n",(0,t.jsx)(n.li,{children:"Run a target in projects with a matching tag."}),"\n",(0,t.jsx)(n.li,{children:"Run a target in projects that match a keyword."}),"\n",(0,t.jsx)(n.li,{children:"Run a target in projects based on AND or OR conditions."}),"\n",(0,t.jsx)(n.li,{children:"Or a combination of these."}),"\n",(0,t.jsx)(n.li,{children:"And many more!"}),"\n"]}),"\n",(0,t.jsxs)(n.p,{children:["Supporting all of these scenarios through CLI arguments just feels like bad design, and would result\nin a poor developer experience. There had to be a better way to support this! So we set out to solve\nthis problem, and after much thought, we're stoked to introduce\n",(0,t.jsx)(n.a,{href:"/docs/concepts/query-lang",children:"MQL, a query language unique to moon"}),"."]}),"\n",(0,t.jsxs)(n.p,{children:['With MQL, you can now run scenarios like "I want to build all Node.js libraries", or "I want to lint\nand test all Rust projects". Simply pass an unscoped target and a query to the ',(0,t.jsx)(n.code,{children:"run"})," command:"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:'$ moon run :build --query "taskPlatform=node && projectType=library"\n\n$ moon run :lint :test --query "language=rust"\n'})}),"\n",(0,t.jsx)(n.p,{children:"This is only the first iteration of MQL and it's already quite powerful. Expect additional fields,\nfeatures, and functionality in the future!"}),"\n",(0,t.jsx)(n.h2,{id:"plugin-support-for-proto",children:"Plugin support for proto"}),"\n",(0,t.jsxs)(n.p,{children:["Earlier this week we announced ",(0,t.jsx)(n.a,{href:"./proto-v0.7",children:"plugin support for proto"}),", starting with a TOML based\nplugin. This is great as it allows ",(0,t.jsx)(n.em,{children:"any"})," kind of versioned tool to be managed in proto's toolchain,\nso why not moon? Starting with this release, you can now install and manage moon ",(0,t.jsx)(n.em,{children:"from"})," proto, using\nour officially maintained TOML plugin."]}),"\n",(0,t.jsxs)(n.p,{children:["In your ",(0,t.jsx)(n.code,{children:".prototools"})," or ",(0,t.jsx)(n.code,{children:"~/.proto/config.toml"})," file, add the following snippet:"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'[plugins]\nmoon = "source:https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml"\n'})}),"\n",(0,t.jsxs)(n.p,{children:["And as easy as that, you can now use ",(0,t.jsx)(n.code,{children:"moon"})," as a tool within any ",(0,t.jsx)(n.code,{children:"proto"})," command. For example:"]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:"$ proto install moon 1.3.0\n$ proto list-remote moon\n$ proto bin moon\n"})}),"\n",(0,t.jsx)(n.p,{children:"Furthermore, with proto, we can now pin the version of moon on a per-project basis. Perfect for\nenforcing the same version for all developers on your team!"}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-toml",metastring:'title=".prototools"',children:'moon = "1.3.0"\n'})}),"\n",(0,t.jsxs)(n.blockquote,{children:["\n",(0,t.jsxs)(n.p,{children:["When using this approach, be sure ",(0,t.jsx)(n.code,{children:"~/proto/.bin"})," is in your ",(0,t.jsx)(n.code,{children:"PATH"}),", and takes precedence over\n",(0,t.jsx)(n.code,{children:"~/.moon/bin"}),"."]}),"\n"]}),"\n",(0,t.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,t.jsxs)(n.p,{children:["View the ",(0,t.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/v1.3.0",children:"official release"})," for a full list of\nchanges."]}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsx)(n.li,{children:"Upgraded to proto v0.7."}),"\n",(0,t.jsx)(n.li,{children:"Improved accuracy of our globbing utilities, especially around dotfiles/dotfolders."}),"\n",(0,t.jsx)(n.li,{children:"Updated config loading to be strict and error on unknown fields for non-root fields."}),"\n"]})]})}function u(e={}){const{wrapper:n}={...(0,r.a)(),...e.components};return n?(0,t.jsx)(n,{...e,children:(0,t.jsx)(d,{...e})}):d(e)}},15627:(e,n,o)=>{o.d(n,{Z:()=>t});const t=o.p+"assets/images/v1.3-042bd7752666b4bbaf030b6572d4052e.png"},71670:(e,n,o)=>{o.d(n,{Z:()=>i,a:()=>s});var t=o(27378);const r={},a=t.createContext(r);function s(e){const n=t.useContext(a);return t.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function i(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:s(e.components),t.createElement(a.Provider,{value:n},e.children)}}}]);