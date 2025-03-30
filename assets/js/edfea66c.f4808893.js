"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[90247],{7545:(e,n,o)=>{o.r(n),o.d(n,{default:()=>t});const t=o.p+"assets/images/init-flow-2a3ba1d56ee42c65dabf1f219d732e98.png"},33057:(e,n,o)=>{o.d(n,{A:()=>t});const t=o.p+"assets/images/v0.18-dbb354a41e8d2854b61b9680283d2e65.png"},33146:(e,n,o)=>{o.d(n,{A:()=>i});var t=o(62540);function i(e){let{src:n,width:o="90%",alt:i="",title:s,align:r="center",padding:a="1rem"}=e;return(0,t.jsx)("div",{style:{marginBottom:a,marginTop:a,textAlign:r},children:(0,t.jsx)("img",{src:n.default,width:o,alt:i,title:s,className:"inline-block"})})}},43023:(e,n,o)=>{o.d(n,{R:()=>r,x:()=>a});var t=o(63696);const i={},s=t.createContext(i);function r(e){const n=t.useContext(s);return t.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function a(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(i):e.components||i:r(e.components),t.createElement(s.Provider,{value:n},e.children)}},70695:e=>{e.exports=JSON.parse('{"permalink":"/blog/v0.18","editUrl":"https://github.com/moonrepo/moon/tree/master/website/blog/2022-10-31_v0.18.mdx","source":"@site/blog/2022-10-31_v0.18.mdx","title":"moon v0.18 - Improved configuration and initialization flow","description":"Happy Halloween \ud83c\udf83 \ud83d\udc7b! With this spooky release, we\'ve focused heavily on our internals and","date":"2022-10-31T00:00:00.000Z","tags":[{"inline":true,"label":"project","permalink":"/blog/tags/project"},{"inline":true,"label":"config","permalink":"/blog/tags/config"},{"inline":true,"label":"init","permalink":"/blog/tags/init"},{"inline":true,"label":"node","permalink":"/blog/tags/node"}],"readingTime":2.09,"hasTruncateMarker":true,"authors":[{"name":"Miles Johnson","title":"Founder, developer","url":"https://github.com/milesj","imageURL":"/img/authors/miles.jpg","key":"milesj","page":null}],"frontMatter":{"slug":"v0.18","title":"moon v0.18 - Improved configuration and initialization flow","authors":["milesj"],"tags":["project","config","init","node"],"image":"./img/v0.18.png"},"unlisted":false,"prevItem":{"title":"moon v0.19 - Remote caching beta, affected files, and graph optimization","permalink":"/blog/v0.19"},"nextItem":{"title":"moon v0.17 - Webhooks, extended YAML, and improved runtime performance","permalink":"/blog/v0.17"}}')},81193:(e,n,o)=>{o.r(n),o.d(n,{assets:()=>d,contentTitle:()=>l,default:()=>m,frontMatter:()=>a,metadata:()=>t,toc:()=>c});var t=o(70695),i=o(62540),s=o(43023),r=o(33146);const a={slug:"v0.18",title:"moon v0.18 - Improved configuration and initialization flow",authors:["milesj"],tags:["project","config","init","node"],image:"./img/v0.18.png"},l=void 0,d={image:o(33057).A,authorsImageUrls:[void 0]},c=[{value:"Improved projects configuration",id:"improved-projects-configuration",level:2},{value:"Improved <code>moon init</code> flow",id:"improved-moon-init-flow",level:2},{value:"Customize <code>node</code> execution arguments",id:"customize-node-execution-arguments",level:2},{value:"Other changes",id:"other-changes",level:2},{value:"What&#39;s next?",id:"whats-next",level:2}];function h(e){const n={a:"a",blockquote:"blockquote",code:"code",em:"em",h2:"h2",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.R)(),...e.components};return(0,i.jsxs)(i.Fragment,{children:[(0,i.jsx)(n.p,{children:"Happy Halloween \ud83c\udf83 \ud83d\udc7b! With this spooky release, we've focused heavily on our internals and\nbenchmarking performance metrics, so it's rather light on new features, but we still have some to\nshow!"}),"\n",(0,i.jsx)(n.h2,{id:"improved-projects-configuration",children:"Improved projects configuration"}),"\n",(0,i.jsxs)(n.p,{children:["When moon initially launched, it required defining all\n",(0,i.jsx)(n.a,{href:"../docs/config/workspace#projects",children:(0,i.jsx)(n.code,{children:"projects"})})," using a map. In v0.3, we added support for globs to\nease the burden of defining many projects. At this point, you had to choose between the 2 patterns,\nwhich wasn't always ideal."]}),"\n",(0,i.jsxs)(n.p,{children:["To improve upon this, you can now define a map ",(0,i.jsx)(n.em,{children:"and"})," globs using a 3rd pattern, like so."]}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/workspace.yml"',children:"projects:\n  globs:\n    - 'apps/*'\n    - 'packages/*'\n  sources:\n    www: 'www'\n"})}),"\n",(0,i.jsxs)(n.h2,{id:"improved-moon-init-flow",children:["Improved ",(0,i.jsx)(n.code,{children:"moon init"})," flow"]}),"\n",(0,i.jsxs)(n.p,{children:["The ",(0,i.jsx)(n.code,{children:"moon init"})," command hasn't changed much since our v0.1 release, and with many new features and\nconfigurations over these last 18 releases, the initialization flow has fallen quite behind. We felt\nit was the perfect time to modernize this command a bit."]}),"\n",(0,i.jsx)(n.p,{children:"On top of automatically detecting settings from the environment, the command will also now prompt\nyou with additional questions while configuring Node.js or TypeScript. Here's an example of this\nflow:"}),"\n",(0,i.jsx)(r.A,{src:o(7545),width:"80%"}),"\n",(0,i.jsxs)(n.p,{children:["Furthermore, the command also supports enabling a new tool (appending configuration to\n",(0,i.jsx)(n.code,{children:".moon/workspace.yml"}),") into an ",(0,i.jsx)(n.em,{children:"existing"})," moon repository, by running ",(0,i.jsx)(n.code,{children:"moon init --tool <name>"}),"."]}),"\n",(0,i.jsxs)(n.h2,{id:"customize-node-execution-arguments",children:["Customize ",(0,i.jsx)(n.code,{children:"node"})," execution arguments"]}),"\n",(0,i.jsxs)(n.p,{children:["moon manages the Node.js binary in our toolchain, and runs all Node.js based tasks using this\nbinary, instead of relying on the binary found in the developer's environment. Because of this, how\n",(0,i.jsx)(n.code,{children:"node"})," is executed is abstracted away from end users."]}),"\n",(0,i.jsxs)(n.p,{children:["What if you wanted to use an ",(0,i.jsx)(n.a,{href:"https://nodejs.org/api/esm.html#loaders",children:"experimental loader"})," and\nexecute TypeScript code at ",(0,i.jsx)(n.em,{children:"runtime"}),"? Or to preserve symlinks? Well, you couldn't... but no longer,\nas we've added a new setting, ",(0,i.jsx)(n.a,{href:"../docs/config/toolchain#binexecargs",children:(0,i.jsx)(n.code,{children:"node.binExecArgs"})}),", that\nallows additional ",(0,i.jsx)(n.code,{children:"node"})," ",(0,i.jsx)(n.a,{href:"https://nodejs.org/api/cli.html#options",children:"CLI arguments"})," to be defined,\nthat will be passed to ",(0,i.jsx)(n.em,{children:"all"})," executions."]}),"\n",(0,i.jsx)(n.pre,{children:(0,i.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/workspace.yml" {2-4}',children:"node:\n  binExecArgs:\n    - '--loader'\n    - '@boost/module/loader'\n"})}),"\n",(0,i.jsxs)(n.blockquote,{children:["\n",(0,i.jsxs)(n.p,{children:["Learn more about the\n",(0,i.jsx)(n.a,{href:"https://boostlib.dev/docs/module#ecmascript-module-loaders",children:"Boost module loader"}),"!"]}),"\n"]}),"\n",(0,i.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,i.jsxs)(n.p,{children:["View the\n",(0,i.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/%40moonrepo%2Fcli%400.18.0",children:"official release"})," for a\nfull list of changes."]}),"\n",(0,i.jsxs)(n.ul,{children:["\n",(0,i.jsx)(n.li,{children:"Default Node.js version has been updated to v18.12 (the new LTS) from v16.17."}),"\n",(0,i.jsxs)(n.li,{children:["Updated the ",(0,i.jsx)(n.code,{children:"moon check"})," command to support an ",(0,i.jsx)(n.code,{children:"--all"})," flag."]}),"\n",(0,i.jsx)(n.li,{children:"Improvements to how we store stdout/stderr logs for ran targets."}),"\n",(0,i.jsx)(n.li,{children:"Work tree dirty checks when running migration commands."}),"\n"]}),"\n",(0,i.jsx)(n.h2,{id:"whats-next",children:"What's next?"}),"\n",(0,i.jsx)(n.p,{children:"Expect the following in the v0.19 release!"}),"\n",(0,i.jsxs)(n.ul,{children:["\n",(0,i.jsxs)(n.li,{children:["Laying the groundwork for ",(0,i.jsx)(n.em,{children:"remote caching"}),"!"]}),"\n",(0,i.jsx)(n.li,{children:"An in-repo secrets management layer."}),"\n",(0,i.jsx)(n.li,{children:"Performance and affected improvements."}),"\n"]})]})}function m(e={}){const{wrapper:n}={...(0,s.R)(),...e.components};return n?(0,i.jsx)(n,{...e,children:(0,i.jsx)(h,{...e})}):h(e)}}}]);