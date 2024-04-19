"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[66205],{50599:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>l,contentTitle:()=>i,default:()=>m,frontMatter:()=>o,metadata:()=>r,toc:()=>c});var a=t(24246),s=t(71670);const o={slug:"moon-v1.23",title:"moon v1.23 - Codegen improvements, stack inheritance, internal tasks, and more",authors:["milesj"],tags:["tasks","codegen","template","tack","inheritance"],image:"./img/moon/v1.23.png"},i=void 0,r={permalink:"/blog/moon-v1.23",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2024-03-25_moon-v1.23.mdx",source:"@site/blog/2024-03-25_moon-v1.23.mdx",title:"moon v1.23 - Codegen improvements, stack inheritance, internal tasks, and more",description:"It's been a month since our last release, and we're excited to land major codegen and task",date:"2024-03-25T00:00:00.000Z",tags:[{label:"tasks",permalink:"/blog/tags/tasks"},{label:"codegen",permalink:"/blog/tags/codegen"},{label:"template",permalink:"/blog/tags/template"},{label:"tack",permalink:"/blog/tags/tack"},{label:"inheritance",permalink:"/blog/tags/inheritance"}],readingTime:2.575,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"moon-v1.23",title:"moon v1.23 - Codegen improvements, stack inheritance, internal tasks, and more",authors:["milesj"],tags:["tasks","codegen","template","tack","inheritance"],image:"./img/moon/v1.23.png"},unlisted:!1,prevItem:{title:"proto v0.34 - New detection strategy, status command, and outdated improvements",permalink:"/blog/proto-v0.34"},nextItem:{title:"proto v0.31 - Improved version pinning, removed global packages management, and more",permalink:"/blog/proto-v0.31"}},l={image:t(48738).Z,authorsImageUrls:[void 0]},c=[{value:"Template &amp; generator improvements",id:"template--generator-improvements",level:2},{value:"Git and npm template locators",id:"git-and-npm-template-locators",level:3},{value:"Custom template names",id:"custom-template-names",level:3},{value:"New variable settings",id:"new-variable-settings",level:3},{value:"Stack-based task inheritance",id:"stack-based-task-inheritance",level:2},{value:"Internal tasks",id:"internal-tasks",level:2},{value:"Other changes",id:"other-changes",level:2}];function d(e){const n={a:"a",blockquote:"blockquote",code:"code",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.a)(),...e.components};return(0,a.jsxs)(a.Fragment,{children:[(0,a.jsx)(n.p,{children:"It's been a month since our last release, and we're excited to land major codegen and task\nimprovements."}),"\n",(0,a.jsx)(n.h2,{id:"template--generator-improvements",children:"Template & generator improvements"}),"\n",(0,a.jsx)(n.p,{children:"Based on feedback and requests from community, we've made quite a few improvements to our code\ngeneration workflow!"}),"\n",(0,a.jsx)(n.h3,{id:"git-and-npm-template-locators",children:"Git and npm template locators"}),"\n",(0,a.jsxs)(n.p,{children:["Our ",(0,a.jsx)(n.a,{href:"/docs/config/workspace#templates",children:(0,a.jsx)(n.code,{children:"generator.templates"})})," setting has only supported file system\npaths, relative from the workspace root. This has made it quite difficult to share templates across\nrepositories, but no longer!"]}),"\n",(0,a.jsxs)(n.p,{children:["Template locations now support Git repositories and npm packages, through the ",(0,a.jsx)(n.code,{children:"git:"})," and ",(0,a.jsx)(n.code,{children:"npm:"}),"\nprefixes respectively. The ",(0,a.jsx)(n.code,{children:"git:"})," locator requires a Git repository URL and explicit revision\n(branch, commit, etc), while the ",(0,a.jsx)(n.code,{children:"npm:"})," locator requires a package name and explicit version. For\nexample:"]}),"\n",(0,a.jsx)(n.pre,{children:(0,a.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"generator:\n  templates:\n    - 'git:github.com/moonrepo/templates#master'\n    - 'npm:@moonrepo/templates#1.2.3'\n"})}),"\n",(0,a.jsxs)(n.blockquote,{children:["\n",(0,a.jsxs)(n.p,{children:["Learn more about this in the official\n",(0,a.jsx)(n.a,{href:"/docs/guides/codegen#configuring-template-locations",children:"code generation guide"}),"!"]}),"\n"]}),"\n",(0,a.jsx)(n.h3,{id:"custom-template-names",children:"Custom template names"}),"\n",(0,a.jsxs)(n.p,{children:["Historically, a template's name was derived from the directory the\n",(0,a.jsx)(n.a,{href:"/docs/config/template",children:(0,a.jsx)(n.code,{children:"template.yml"})})," file was located in. While this works great for small repos,\nit falls apart for large monorepos when there's multiple teams defining templates, as the chance of\nname collisions arise."]}),"\n",(0,a.jsxs)(n.p,{children:["To combat this problem, we're introducing a new ",(0,a.jsxs)(n.a,{href:"/docs/config/template#id",children:[(0,a.jsx)(n.code,{children:"id"})," setting"]})," for\ntemplates, which allows you to customize the exact name of the template. This setting is optional,\nand if not provided, the name will be derived from the directory as before."]}),"\n",(0,a.jsx)(n.pre,{children:(0,a.jsx)(n.code,{className:"language-yaml",metastring:'title="template.yml"',children:"id: 'my-template'\n"})}),"\n",(0,a.jsx)(n.h3,{id:"new-variable-settings",children:"New variable settings"}),"\n",(0,a.jsxs)(n.p,{children:["And lastly, we're introducing some additions and improvements to template\n",(0,a.jsx)(n.a,{href:"/docs/config/template#variables",children:(0,a.jsx)(n.code,{children:"variables"})}),"."]}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:["A new ",(0,a.jsx)(n.code,{children:"order"})," setting, which defines the order in which variables are prompted to the user."]}),"\n",(0,a.jsxs)(n.li,{children:["A new ",(0,a.jsx)(n.code,{children:"internal"})," setting, which avoids the value being set from the CLI."]}),"\n",(0,a.jsxs)(n.li,{children:["Enum ",(0,a.jsx)(n.code,{children:"default"})," values now support a list of values (cannot be provided by the CLI yet)."]}),"\n",(0,a.jsxs)(n.li,{children:["Enum ",(0,a.jsx)(n.code,{children:"prompt"}),"s are now optional, and will fallback to the default value if not provided."]}),"\n"]}),"\n",(0,a.jsx)(n.h2,{id:"stack-based-task-inheritance",children:"Stack-based task inheritance"}),"\n",(0,a.jsxs)(n.p,{children:["Last month in ",(0,a.jsx)(n.a,{href:"./moon-v1.22",children:"moon v1.22"}),", we introduced the ",(0,a.jsx)(n.a,{href:"/docs/config/project#stack",children:(0,a.jsx)(n.code,{children:"stack"})}),"\nsetting for organizing projects into what tech stack they belong to. This is primarily for\norganizational purposes, and improving our project constaints implementation."]}),"\n",(0,a.jsxs)(n.p,{children:["Based on community feeedback, we've expanded the ",(0,a.jsx)(n.code,{children:"stack"})," setting to also apply for\n",(0,a.jsx)(n.a,{href:"http://localhost:3000/docs/concepts/task-inheritance#scope-by-project-metadata",children:"task inheritance"}),".\nYou can now inherit tasks for the stack itself, or through a combination of the project language,\nplatform, and type. For example:"]}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsx)(n.li,{children:(0,a.jsx)(n.code,{children:".moon/tasks/backend.yml"})}),"\n",(0,a.jsx)(n.li,{children:(0,a.jsx)(n.code,{children:".moon/tasks/javascript-backend.yml"})}),"\n",(0,a.jsx)(n.li,{children:(0,a.jsx)(n.code,{children:".moon/tasks/frontend-library.yml"})}),"\n",(0,a.jsx)(n.li,{children:(0,a.jsx)(n.code,{children:".moon/tasks/bun-frontend-application.yml"})}),"\n"]}),"\n",(0,a.jsx)(n.h2,{id:"internal-tasks",children:"Internal tasks"}),"\n",(0,a.jsxs)(n.p,{children:["We're introducing a new ",(0,a.jsx)(n.a,{href:"/docs/concepts/task#modes",children:"task mode"})," called internal, which can be enabled\nwith the ",(0,a.jsx)(n.a,{href:"/docs/config/project#internal",children:(0,a.jsx)(n.code,{children:"internal"})})," task option. Internal tasks are tasks that are\nnot meant to be ran explicitly by the user (via ",(0,a.jsx)(n.a,{href:"/docs/commands/check",children:(0,a.jsx)(n.code,{children:"moon check"})})," or\n",(0,a.jsx)(n.a,{href:"/docs/commands/run",children:(0,a.jsx)(n.code,{children:"moon run"})}),"), but are used internally as dependencies of other tasks."]}),"\n",(0,a.jsx)(n.p,{children:"This functionality provides another way to organize your tasks."}),"\n",(0,a.jsx)(n.pre,{children:(0,a.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  prepare:\n    command: 'intermediate-step'\n    options:\n      internal: true\n"})}),"\n",(0,a.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,a.jsxs)(n.p,{children:["View the ",(0,a.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/v1.23.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,a.jsxs)(n.ul,{children:["\n",(0,a.jsxs)(n.li,{children:["Added a ",(0,a.jsx)(n.code,{children:"variables()"})," function for templates that returns an object of all variables available."]}),"\n",(0,a.jsxs)(n.li,{children:["Updated ",(0,a.jsx)(n.code,{children:"moon project"})," and ",(0,a.jsx)(n.code,{children:"moon task"})," to include the configuration files that tasks inherit from."]}),"\n",(0,a.jsxs)(n.li,{children:["Updated ",(0,a.jsx)(n.code,{children:"moon task"})," to include the modes it belongs to."]}),"\n"]})]})}function m(e={}){const{wrapper:n}={...(0,s.a)(),...e.components};return n?(0,a.jsx)(n,{...e,children:(0,a.jsx)(d,{...e})}):d(e)}},48738:(e,n,t)=>{t.d(n,{Z:()=>a});const a=t.p+"assets/images/v1.23-0ffbabc3b6ece04d0d3157cdfd2cb957.png"},71670:(e,n,t)=>{t.d(n,{Z:()=>r,a:()=>i});var a=t(27378);const s={},o=a.createContext(s);function i(e){const n=a.useContext(o);return a.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function r(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:i(e.components),a.createElement(o.Provider,{value:n},e.children)}}}]);