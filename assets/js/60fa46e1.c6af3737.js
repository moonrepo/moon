"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[4043],{35318:(e,n,t)=>{t.d(n,{Zo:()=>c,kt:()=>d});var r=t(27378);function o(e,n,t){return n in e?Object.defineProperty(e,n,{value:t,enumerable:!0,configurable:!0,writable:!0}):e[n]=t,e}function a(e,n){var t=Object.keys(e);if(Object.getOwnPropertySymbols){var r=Object.getOwnPropertySymbols(e);n&&(r=r.filter((function(n){return Object.getOwnPropertyDescriptor(e,n).enumerable}))),t.push.apply(t,r)}return t}function s(e){for(var n=1;n<arguments.length;n++){var t=null!=arguments[n]?arguments[n]:{};n%2?a(Object(t),!0).forEach((function(n){o(e,n,t[n])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(t)):a(Object(t)).forEach((function(n){Object.defineProperty(e,n,Object.getOwnPropertyDescriptor(t,n))}))}return e}function i(e,n){if(null==e)return{};var t,r,o=function(e,n){if(null==e)return{};var t,r,o={},a=Object.keys(e);for(r=0;r<a.length;r++)t=a[r],n.indexOf(t)>=0||(o[t]=e[t]);return o}(e,n);if(Object.getOwnPropertySymbols){var a=Object.getOwnPropertySymbols(e);for(r=0;r<a.length;r++)t=a[r],n.indexOf(t)>=0||Object.prototype.propertyIsEnumerable.call(e,t)&&(o[t]=e[t])}return o}var l=r.createContext({}),p=function(e){var n=r.useContext(l),t=n;return e&&(t="function"==typeof e?e(n):s(s({},n),e)),t},c=function(e){var n=p(e.components);return r.createElement(l.Provider,{value:n},e.children)},m={inlineCode:"code",wrapper:function(e){var n=e.children;return r.createElement(r.Fragment,{},n)}},u=r.forwardRef((function(e,n){var t=e.components,o=e.mdxType,a=e.originalType,l=e.parentName,c=i(e,["components","mdxType","originalType","parentName"]),u=p(t),d=o,g=u["".concat(l,".").concat(d)]||u[d]||m[d]||a;return t?r.createElement(g,s(s({ref:n},c),{},{components:t})):r.createElement(g,s({ref:n},c))}));function d(e,n){var t=arguments,o=n&&n.mdxType;if("string"==typeof e||o){var a=t.length,s=new Array(a);s[0]=u;var i={};for(var l in n)hasOwnProperty.call(n,l)&&(i[l]=n[l]);i.originalType=e,i.mdxType="string"==typeof e?e:o,s[1]=i;for(var p=2;p<a;p++)s[p]=t[p];return r.createElement.apply(null,s)}return r.createElement.apply(null,t)}u.displayName="MDXCreateElement"},23574:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>l,contentTitle:()=>s,default:()=>m,frontMatter:()=>a,metadata:()=>i,toc:()=>p});var r=t(25773),o=(t(27378),t(35318));const a={slug:"moon-v1.1",title:"moon v1.1 - Task debugging and improvements",authors:["milesj"],tags:["tokens","tasks"],image:"./img/moon/v1.1.png"},s=void 0,i={permalink:"/blog/moon-v1.1",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2023-04-03_moon-v1.1.mdx",source:"@site/blog/2023-04-03_moon-v1.1.mdx",title:"moon v1.1 - Task debugging and improvements",description:"With this release, we're introducing some quality of life task improvements.",date:"2023-04-03T00:00:00.000Z",formattedDate:"April 3, 2023",tags:[{label:"tokens",permalink:"/blog/tags/tokens"},{label:"tasks",permalink:"/blog/tags/tasks"}],readingTime:1.775,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"moon-v1.1",title:"moon v1.1 - Task debugging and improvements",authors:["milesj"],tags:["tokens","tasks"],image:"./img/moon/v1.1.png"},prevItem:{title:"proto v0.5 - Version aliasing and global binaries",permalink:"/blog/proto-v0.5"},nextItem:{title:"proto v0.4 - Rust support, user configs, and more",permalink:"/blog/proto-v0.4"}},l={image:t(89585).Z,authorsImageUrls:[void 0]},p=[{value:"Token variable support in task commands",id:"token-variable-support-in-task-commands",level:2},{value:"Run targets in closest project",id:"run-targets-in-closest-project",level:2},{value:"View resolved task information",id:"view-resolved-task-information",level:2},{value:"Other changes",id:"other-changes",level:2}],c={toc:p};function m(e){let{components:n,...t}=e;return(0,o.kt)("wrapper",(0,r.Z)({},c,t,{components:n,mdxType:"MDXLayout"}),(0,o.kt)("p",null,"With this release, we're introducing some quality of life task improvements."),(0,o.kt)("h2",{id:"token-variable-support-in-task-commands"},"Token variable support in task commands"),(0,o.kt)("p",null,"moon supports a concept known as ",(0,o.kt)("a",{parentName:"p",href:"/docs/concepts/token"},"tokens")," where values are injected into tasks\nduring project graph creation. This allows for dynamic values in your tasks, such as the current\nproject language, or the current task name, and is crucial for task inheritance to work."),(0,o.kt)("p",null,"However, tokens were only supported by task args, inputs, and outputs, but not commands... until\nnow. Commands can now use token variables (but not token functions). For example, this is useful for\nreferencing shared scripts from the workspace root."),(0,o.kt)("pre",null,(0,o.kt)("code",{parentName:"pre",className:"language-yaml",metastring:'title="moon.yml"',title:'"moon.yml"'},"tasks:\n    precheck:\n        command: '$workspaceRoot/scripts/precheck.sh'\n")),(0,o.kt)("h2",{id:"run-targets-in-closest-project"},"Run targets in closest project"),(0,o.kt)("p",null,"The ",(0,o.kt)("a",{parentName:"p",href:"/docs/commands/run"},(0,o.kt)("inlineCode",{parentName:"a"},"moon run"))," command can run targets in an array of different formats, but\nwas unable to run targets based on the current working directory. Well no more! You can now run\ntasks from the closest project based on file path by omitting ",(0,o.kt)("inlineCode",{parentName:"p"},":")," from the target name."),(0,o.kt)("pre",null,(0,o.kt)("code",{parentName:"pre",className:"language-shell"},"$ cd packages/components\n\n# Runs `components:build` internally\n$ moon run build\n")),(0,o.kt)("h2",{id:"view-resolved-task-information"},"View resolved task information"),(0,o.kt)("p",null,"Debugging task issues can be a quite a pain, as there can be many points of failure. Are inputs too\ngreedy? Are outputs not being created? Does it exist at all? To help with this, you can now view\ntask information by running ",(0,o.kt)("a",{parentName:"p",href:"/docs/commands/task"},(0,o.kt)("inlineCode",{parentName:"a"},"moon task <target>")),"."),(0,o.kt)("pre",null,(0,o.kt)("code",{parentName:"pre",className:"language-shell"},"$ moon task app:build\n")),(0,o.kt)("p",null,"This command will display ",(0,o.kt)("em",{parentName:"p"},"resolved")," information, including inherited settings, and path resolved\ninputs and outputs. Here's an example:"),(0,o.kt)("pre",null,(0,o.kt)("code",{parentName:"pre"},"RUNTIME:BUILD\n\nID: build\nProject: runtime\nPlatform: node\nType: build\n\nPROCESS\n\nCommand: packemon build --addFiles --addExports --declaration\nEnvironment variables:\n - NODE_ENV = production\nWorking directory: /Projects/moon/packages/runtime\nRuns dependencies: Concurrently\nRuns in CI: Yes\n\nDEPENDS ON\n\n - types:build\n\nINPUTS\n\n - .moon/*.yml\n - packages/runtime/src/**/*\n - packages/runtime/tsconfig.*.json\n - packages/runtime/types/**/*\n - packages/runtime/package.json\n - packages/runtime/tsconfig.json\n - tsconfig.options.json\n\nOUTPUTS\n\n - packages/runtime/cjs\n")),(0,o.kt)("h2",{id:"other-changes"},"Other changes"),(0,o.kt)("p",null,"View the ",(0,o.kt)("a",{parentName:"p",href:"https://github.com/moonrepo/moon/releases/tag/v1.1.0"},"official release")," for a full list of\nchanges."),(0,o.kt)("ul",null,(0,o.kt)("li",{parentName:"ul"},"Support pnpm v8's new lockfile format."),(0,o.kt)("li",{parentName:"ul"},"Better handling for task's that execute the ",(0,o.kt)("inlineCode",{parentName:"li"},"moon")," binary."),(0,o.kt)("li",{parentName:"ul"},"Updated ",(0,o.kt)("inlineCode",{parentName:"li"},"noop")," tasks to be cacheable, so that they can be used for cache hit early returns.")))}m.isMDXComponent=!0},89585:(e,n,t)=>{t.d(n,{Z:()=>r});const r=t.p+"assets/images/v1.1-1447beccba44240528518a0af56c91f2.png"}}]);